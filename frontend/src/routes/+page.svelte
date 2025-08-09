<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type SystemStatus, type ToolsResponse, type Tool, type CustomCommandSpec, type CustomRestartRequest, type ExecuteCommandRequest, type McpExecuteResponse, type MonitoringHealthStatus, type MonitoringSystemAlerts } from '$lib/api';
  import { runtimeMode, modeStore, type ServiceStatus } from '$lib/stores/mode';
  import { systemMetrics, systemMetricsLoading, systemMetricsService } from '$lib/stores/systemMetrics';
  import SystemMetricsCard from '$lib/components/SystemMetricsCard.svelte';
  import HealthStatusCard from '$lib/components/HealthStatusCard.svelte';
  import ToolMetricsCompact from '$lib/components/ToolMetricsCompact.svelte';

  let systemStatus: SystemStatus | null = null;
  let toolsData: ToolsResponse | null = null;
  let loading = true;
  let error = '';
  let nextRefreshIn = 30;
  
  // Monitoring data
  let healthStatus: MonitoringHealthStatus | null = null;
  let systemAlerts: MonitoringSystemAlerts | null = null;
  let monitoringLoading = false;
  
  


  // MagicTunnel restart state
  let restartingMagicTunnel = false;
  let restartCountdown = 0;
  let restartResult: any = null;
  let showRestartDialog = false;
  let startupArgs = '--config magictunnel-config.yaml --log-level info';

  // Health check state
  let performingHealthCheck = false;
  let healthCheckResult: any = null;

  // Custom restart workflow state
  let showCustomRestartBuilder = false;
  let customRestartWorkflow: CustomRestartRequest = {
    pre_commands: [],
    start_args: [],
    post_commands: []
  };
  let executingCustomRestart = false;
  let customRestartResult: any = null;
  let showCommandEditor = false;
  let editingCommand: CustomCommandSpec | null = null;
  let editingIndex = -1;
  let editingType: 'pre' | 'post' = 'pre';


  // Copy to clipboard functionality
  async function copyToClipboard(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      alert(`${label} copied to clipboard!`);
    } catch (err) {
      console.error('Failed to copy: ', err);
      alert(`Failed to copy ${label}`);
    }
  }

  // Get copyable value from system status
  function getCopyableValue(key: string): string {
    if (!systemStatus?.environment) return '';
    
    // Use the specific value fields provided by backend
    switch (key) {
      case 'openai_api_key_set':
        return systemStatus.environment.openai_api_key_full || 'API key not available';
      case 'anthropic_api_key_set':
        return systemStatus.environment.anthropic_api_key_full || 'API key not available';
      case 'ollama_base_url':
        return systemStatus.environment.ollama_base_url || 'http://localhost:11434';
      case 'magictunnel_semantic_model':
        return systemStatus.environment.magictunnel_semantic_model || 'ollama:nomic-embed-text';
      default:
        return systemStatus.environment[key] || '';
    }
  }

  async function loadDashboardData() {
    loading = true;
    error = '';
    
    try {
      // Load system status and tools data
      const [status, tools] = await Promise.all([
        api.getSystemStatus().catch(() => null),
        api.getTools().catch(() => null)
      ]);
      
      systemStatus = status;
      toolsData = tools;
      
    } catch (err) {
      error = `Failed to load dashboard data: ${err}`;
      console.error('Dashboard data loading error:', err);
    } finally {
      loading = false;
      nextRefreshIn = 30; // Reset countdown
    }
  }

  async function loadMonitoringData() {
    monitoringLoading = true;
    
    try {
      // Load monitoring data in parallel (system metrics now come from shared store)
      const [health, alerts] = await Promise.all([
        api.getHealthStatus().catch(() => null),
        api.getSystemAlerts().catch(() => null)
      ]);
      
      healthStatus = health;
      systemAlerts = alerts;
    } catch (err) {
      console.error('Failed to load monitoring data:', err);
      // Don't set error as this is secondary data
    } finally {
      monitoringLoading = false;
    }
  }

  function handleViewTools() {
    window.location.href = '/tools';
  }









  function handleManualRefresh() {
    loadDashboardData();
  }



  // MagicTunnel restart functions
  function restartMagicTunnel() {
    // Show the restart confirmation dialog
    showRestartDialog = true;
  }

  function closeRestartDialog() {
    showRestartDialog = false;
  }

  async function confirmRestart() {
    showRestartDialog = false;
    restartingMagicTunnel = true;
    restartResult = null;
    
    try {
      // Parse startup args into array
      const args = startupArgs.trim().split(/\s+/).filter(arg => arg.length > 0);
      
      // Use custom restart with startup args
      const result = await api.customRestartMagicTunnel({
        start_args: args
      });
      
      restartResult = result;
      
      if (result.status === 'success') {
        showRestartCountdown();
      }
    } catch (err) {
      console.error('Restart failed:', err);
      restartResult = {
        action: 'restart_magictunnel',
        status: 'error',
        message: `Failed to restart: ${err}`,
        timestamp: new Date().toISOString()
      };
      restartingMagicTunnel = false;
    }
  }

  function showRestartCountdown() {
    restartCountdown = 30; // 30 second countdown
    const countdown = setInterval(() => {
      restartCountdown--;
      if (restartCountdown <= 0) {
        clearInterval(countdown);
        attemptReconnection();
      }
    }, 1000);
  }

  async function attemptReconnection() {
    let reconnectAttempts = 0;
    const maxAttempts = 12; // Try for 60 seconds (12 * 5 seconds)
    
    const tryReconnect = async () => {
      try {
        await api.getSystemStatus();
        // If successful, reconnection is complete
        restartingMagicTunnel = false;
        restartResult = {
          action: 'restart_magictunnel',
          status: 'success',
          message: 'MagicTunnel restarted successfully and is now responding',
          timestamp: new Date().toISOString()
        };
        // Reload all data
        loadDashboardData();
        loadMakefileCommands();
      } catch (err) {
        reconnectAttempts++;
        if (reconnectAttempts < maxAttempts) {
          setTimeout(tryReconnect, 5000); // Try again in 5 seconds
        } else {
          restartingMagicTunnel = false;
          restartResult = {
            action: 'restart_magictunnel',
            status: 'error',
            message: 'Failed to reconnect to MagicTunnel after restart. Please check if the service is running.',
            timestamp: new Date().toISOString()
          };
        }
      }
    };
    
    tryReconnect();
  }

  // Health check function
  async function performHealthCheck() {
    if (performingHealthCheck) return; // Prevent multiple simultaneous checks
    
    performingHealthCheck = true;
    healthCheckResult = null;
    
    try {
      const startTime = Date.now();
      
      // Perform comprehensive health check
      const [systemStatus, toolsStatus] = await Promise.all([
        api.getSystemStatus().catch(err => ({ error: err.toString() })),
        api.getTools().catch(err => ({ error: err.toString() }))
      ]);
      
      const responseTime = Date.now() - startTime;
      
      // Analyze results
      const systemHealthy = systemStatus && !systemStatus.error;
      const toolsHealthy = toolsStatus && !toolsStatus.error && toolsStatus.tools;
      const overallHealthy = systemHealthy && toolsHealthy;
      
      healthCheckResult = {
        action: 'health_check',
        status: overallHealthy ? 'success' : 'warning',
        overall_status: overallHealthy ? 'healthy' : 'degraded',
        response_time: responseTime,
        timestamp: new Date().toISOString(),
        details: {
          system_status: {
            status: systemHealthy ? 'healthy' : 'error',
            error: systemStatus?.error || null,
            version: systemStatus?.version || 'unknown',
            uptime: systemStatus?.uptime || 'unknown'
          },
          tools_service: {
            status: toolsHealthy ? 'healthy' : 'error',
            error: toolsStatus?.error || null,
            tools_count: toolsStatus?.total || 0
          },
          external_mcp: {
            servers_configured: systemStatus?.external_mcp?.servers_configured || 0,
            servers_active: systemStatus?.external_mcp?.servers_active || 0
          }
        },
        message: overallHealthy 
          ? `All systems healthy. Response time: ${responseTime}ms`
          : `Some issues detected. System: ${systemHealthy ? 'OK' : 'ERROR'}, Tools: ${toolsHealthy ? 'OK' : 'ERROR'}`
      };
      
      // Also refresh the main data if health check was successful
      if (overallHealthy) {
        toolsData = toolsStatus;
      }
      
    } catch (err) {
      console.error('Health check failed:', err);
      healthCheckResult = {
        action: 'health_check',
        status: 'error',
        overall_status: 'unhealthy',
        message: `Health check failed: ${err}`,
        timestamp: new Date().toISOString()
      };
    } finally {
      performingHealthCheck = false;
    }
  }

  // Switch runtime mode function - uses same logic as ModeIndicator via window event
  function switchMode() {
    // Toggle between Proxy and Advanced modes
    const newMode = $runtimeMode === 'Proxy' ? 'Advanced' : 'Proxy';
    
    // Dispatch the same event that ModeIndicator would dispatch
    const modeToggleEvent = new CustomEvent('modeToggle', { 
      detail: { newMode }
    });
    window.dispatchEvent(modeToggleEvent);
  }

  // Reactive statements for service status
  $: serviceStatus = $modeStore.service_status;
  $: allServices = serviceStatus?.services || [];
  
  // Filter services by category with fallback logic
  $: coreServices = allServices.filter(service => 
    service.category === 'core' || service.category === 'proxy' || 
    (!service.category && isCoreService(service.name))
  ) || [];
  
  $: advancedServices = allServices.filter(service => 
    service.category === 'advanced' || service.category === 'enterprise' ||
    (!service.category && isAdvancedService(service.name))
  ) || [];

  // Helper functions to identify service types by name if category is missing
  function isCoreService(serviceName: string): boolean {
    const coreServiceNames = [
      'mcp_server', 'registry_service', 'smart_discovery', 'sampling_service', 
      'elicitation_service', 'web_dashboard', 'core_llm_services'
    ];
    return coreServiceNames.some(name => serviceName.toLowerCase().includes(name.toLowerCase()));
  }

  function isAdvancedService(serviceName: string): boolean {
    const advancedServiceNames = [
      'tool_enhancement', 'security_suite', 'rbac', 'audit', 'allowlist', 
      'sanitization', 'enterprise', 'advanced'
    ];
    return advancedServiceNames.some(name => serviceName.toLowerCase().includes(name.toLowerCase()));
  }

  // Debug logging (remove in production)
  $: if (serviceStatus && typeof window !== 'undefined') {
    console.log('Service Status Debug:', {
      total_services: serviceStatus.total_services,
      all_services_count: allServices.length,
      core_services_count: coreServices.length,
      advanced_services_count: advancedServices.length,
      services_sample: allServices.slice(0, 3).map(s => ({ name: s.name, category: s.category, status: s.status }))
    });
  }

  onMount(() => {
    loadDashboardData();
    loadMonitoringData(); // Load monitoring data
    systemMetricsService.start(); // Start shared metrics service
    
    // Countdown timer - updates every second
    const countdownInterval = setInterval(() => {
      nextRefreshIn = Math.max(0, nextRefreshIn - 1);
    }, 1000);
    
    // Refresh data every 30 seconds
    const refreshInterval = setInterval(() => {
      loadDashboardData();
      loadMonitoringData(); // Refresh monitoring data too
    }, 30000);
    
    return () => {
      clearInterval(countdownInterval);
      clearInterval(refreshInterval);
      systemMetricsService.stop(); // Stop shared metrics service
    };
  });
</script>

<div class="min-h-screen bg-gray-50">
  <div class="container mx-auto px-4 py-8">
    <!-- Header -->
    <header class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-4xl font-bold text-primary-700 mb-2">MagicTunnel Dashboard</h1>
          <p class="text-gray-600">Intelligent bridge between MCP clients and diverse agents/endpoints</p>
        </div>
        
        <!-- Auto-refresh timer -->
        <div class="text-right">
          <div class="text-sm text-gray-500 mb-1">Auto-refresh</div>
          <div class="flex items-center gap-2 text-sm">
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {loading ? 'bg-blue-100 text-blue-800' : 'bg-green-100 text-green-800'}">
              {#if loading}
                🔄 Refreshing...
              {:else}
                ⏱️ Next in {nextRefreshIn}s
              {/if}
            </span>
            <button 
              class="px-2 py-1 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 rounded transition-colors"
              on:click={handleManualRefresh}
              disabled={loading}
              title="Refresh now"
            >
              🔄
            </button>
          </div>
        </div>
      </div>
      
      {#if error}
        <div class="mt-4 text-sm text-red-600">❌ {error}</div>
      {/if}
    </header>

    <!-- Status Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
      <div class="card">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">System Status</h3>
        <div class="flex items-center">
          <div class="w-3 h-3 bg-green-500 rounded-full mr-2"></div>
          <span class="text-green-700 font-medium">
            {systemStatus?.status || 'Loading...'}
          </span>
        </div>
        {#if systemStatus}
          <div class="mt-2 text-sm text-gray-500">
            Version: {systemStatus.version}<br>
            Uptime: {systemStatus.uptime}
          </div>
        {/if}
      </div>

      <a href="/tools" class="card hover:bg-gray-50 transition-colors cursor-pointer">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">Available Tools</h3>
        <div class="text-2xl font-bold text-primary-600">
          {toolsData?.total ?? '--'}
        </div>
        <p class="text-sm text-gray-500">
          {toolsData ? 'Tools loaded' : 'Loading...'}
        </p>
      </a>

      <a href="/services" class="card hover:bg-gray-50 transition-colors cursor-pointer">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">External MCP Servers</h3>
        <div class="text-2xl font-bold text-primary-600">
          {systemStatus?.external_mcp?.servers_active ?? '--'}
        </div>
        <p class="text-sm text-gray-500">
          Configured: {systemStatus?.external_mcp?.servers_configured ?? '--'}
        </p>
      </a>

      <div class="card">
        <h3 class="text-lg font-semibold text-gray-700 mb-3">Service Status</h3>
        <div class="space-y-3">
          {#if serviceStatus && allServices.length > 0}
            <!-- Core/Proxy Services -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <h4 class="text-sm font-medium text-gray-600">Core Services</h4>
                <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                  {coreServices.length}
                </span>
              </div>
              <div class="space-y-1">
                {#if coreServices.length > 0}
                  {#each coreServices as service}
                    <div class="flex items-center justify-between text-xs">
                      <span class="text-gray-600 truncate" title={service.name}>{service.name}</span>
                      <span class="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium {
                        service.status === 'active' || service.status === 'running' ? 'bg-green-100 text-green-800' :
                        service.status === 'disabled' || service.status === 'stopped' ? 'bg-gray-100 text-gray-800' :
                        'bg-red-100 text-red-800'
                      }">
                        {service.status === 'active' || service.status === 'running' ? '✅' :
                         service.status === 'disabled' || service.status === 'stopped' ? '⏸️' : '❌'}
                      </span>
                    </div>
                  {/each}
                {:else}
                  <p class="text-xs text-gray-400">No core services detected</p>
                {/if}
              </div>
            </div>

            <!-- Advanced Services -->
            <div class="border-t pt-3">
              <div class="flex items-center justify-between mb-2">
                <h4 class="text-sm font-medium {advancedServices.length > 0 ? 'text-gray-600' : 'text-gray-400'}">Advanced Services</h4>
                <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {
                  advancedServices.length > 0 ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
                }">
                  {advancedServices.length}
                </span>
              </div>
              <div class="space-y-1">
                {#if advancedServices.length > 0}
                  {#each advancedServices as service}
                    <div class="flex items-center justify-between text-xs">
                      <span class="text-gray-600 truncate" title={service.name}>{service.name}</span>
                      <span class="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium {
                        service.status === 'active' || service.status === 'running' ? 'bg-green-100 text-green-800' :
                        service.status === 'disabled' || service.status === 'stopped' ? 'bg-gray-100 text-gray-800' :
                        'bg-red-100 text-red-800'
                      }">
                        {service.status === 'active' || service.status === 'running' ? '✅' :
                         service.status === 'disabled' || service.status === 'stopped' ? '⏸️' : '❌'}
                      </span>
                    </div>
                  {/each}
                {:else}
                  <p class="text-xs text-gray-400">
                    {$runtimeMode === 'Proxy' ? 'Switch to Advanced mode to enable' : 'No advanced services detected'}
                  </p>
                {/if}
              </div>
            </div>
          {:else if serviceStatus}
            <!-- Service status exists but no services found -->
            <div class="text-center py-4">
              <p class="text-sm text-gray-500">No services detected</p>
              <p class="text-xs text-gray-400 mt-1">Total: {serviceStatus.total_services || 0} services</p>
            </div>
          {:else}
            <div class="text-sm text-gray-500 text-center py-2">Loading services...</div>
          {/if}
        </div>
      </div>

    </div>

    <!-- Environment Variables -->
    <div class="card mb-8">
      <h3 class="text-lg font-semibold text-gray-700 mb-4">Environment Variables</h3>
      <div class="space-y-2 max-h-48 overflow-y-auto">
        {#if systemStatus?.environment}
          {#each Object.entries(systemStatus.environment) as [key, value]}
            <div class="flex items-center justify-between py-2 border-b border-gray-100 last:border-b-0">
              <span class="text-sm font-medium text-gray-600">{key.replace(/_/g, '_').toUpperCase()}</span>
              <span class="inline-flex items-center px-2 py-1 rounded text-sm font-medium {
                typeof value === 'boolean' ? 
                  (value ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600') :
                  'bg-blue-100 text-blue-800'
              }">
                {typeof value === 'boolean' ? 
                  (value ? '✅' : '❌') : 
                  (typeof value === 'string' && value.length > 30 ? value.substring(0, 27) + '...' : value)
                }
              </span>
            </div>
          {/each}
        {:else}
          <div class="text-sm text-gray-500 text-center py-4">Loading environment data...</div>
        {/if}
      </div>
    </div>

    <!-- System Management -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-6">
        <div class="flex items-center gap-3">
          <h3 class="text-xl font-semibold text-gray-700">⚙️ System Management</h3>
          <div class="text-sm text-gray-500">System control and monitoring</div>
        </div>
        <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
          {systemStatus?.status || 'Unknown'}
        </span>
      </div>
        
      <!-- Status Displays -->
      <div class="space-y-3 mb-6">
        <!-- Restart Status Display -->
        {#if restartingMagicTunnel}
          <div class="p-4 bg-blue-50 border border-blue-200 rounded-lg">
            <div class="flex items-center mb-2">
              <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600 mr-3"></div>
              <div class="text-blue-700 font-medium">
                {#if restartCountdown > 0}
                  Restarting MagicTunnel... ({restartCountdown}s remaining)
                {:else}
                  Attempting to reconnect...
                {/if}
              </div>
            </div>
            <div class="text-sm text-blue-600">
              The service is being restarted. The dashboard will automatically reconnect when the service is back online.
            </div>
          </div>
        {/if}

        <!-- Restart Result Display -->
        {#if restartResult && !restartingMagicTunnel}
          <div class="p-4 border rounded-lg {restartResult.status === 'success' ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'}">
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-sm font-medium {restartResult.status === 'success' ? 'text-green-700' : 'text-red-700'}">
                {restartResult.status === 'success' ? '✅ Restart Successful' : '❌ Restart Failed'}
              </h4>
              <span class="text-xs {restartResult.status === 'success' ? 'text-green-600' : 'text-red-600'}">
                {new Date(restartResult.timestamp).toLocaleTimeString()}
              </span>
            </div>
            <div class="text-sm {restartResult.status === 'success' ? 'text-green-600' : 'text-red-600'}">
              {restartResult.message}
            </div>
          </div>
        {/if}

        <!-- Health Check Result Display -->
        {#if healthCheckResult}
          <div class="p-4 border rounded-lg {
            healthCheckResult.status === 'success' ? 'bg-green-50 border-green-200' :
            healthCheckResult.status === 'warning' ? 'bg-yellow-50 border-yellow-200' :
            'bg-red-50 border-red-200'
          }">
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-sm font-medium {
                healthCheckResult.status === 'success' ? 'text-green-700' :
                healthCheckResult.status === 'warning' ? 'text-yellow-700' :
                'text-red-700'
              }">
                {#if healthCheckResult.status === 'success'}
                  ✅ System Healthy
                {:else if healthCheckResult.status === 'warning'}
                  ⚠️ System Degraded
                {:else}
                  ❌ System Unhealthy
                {/if}
              </h4>
              <span class="text-xs {
                healthCheckResult.status === 'success' ? 'text-green-600' :
                healthCheckResult.status === 'warning' ? 'text-yellow-600' :
                'text-red-600'
              }">
                {new Date(healthCheckResult.timestamp).toLocaleTimeString()}
              </span>
            </div>
            <div class="text-sm {
              healthCheckResult.status === 'success' ? 'text-green-600' :
              healthCheckResult.status === 'warning' ? 'text-yellow-600' :
              'text-red-600'
            } mb-2">
              {healthCheckResult.message}
            </div>
          </div>
        {/if}
      </div>

      <!-- Management Actions -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <!-- Restart Button -->
        <button
          class="flex items-center justify-center gap-2 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors disabled:opacity-50"
          on:click={() => showRestartDialog = true}
          disabled={loading || restartingMagicTunnel}
        >
          {#if restartingMagicTunnel}
            <span class="animate-spin">⟳</span> Restarting...
          {:else}
            🔄 Restart System
          {/if}
        </button>

        <!-- Health Check Button -->
        <button
          class="flex items-center justify-center gap-2 px-4 py-3 bg-white hover:bg-gray-50 text-gray-700 border border-gray-300 font-medium rounded-lg transition-colors disabled:opacity-50"
          on:click={performHealthCheck}
          disabled={performingHealthCheck || loading}
        >
          {#if performingHealthCheck}
            <span class="animate-spin">⟳</span> Checking...
          {:else}
            🏥 Health Check
          {/if}
        </button>

        <!-- Mode Switch Button -->
        <button
          class="flex items-center justify-center gap-2 px-4 py-3 bg-white hover:bg-gray-50 text-gray-700 border border-gray-300 font-medium rounded-lg transition-colors disabled:opacity-50"
          on:click={switchMode}
          disabled={loading}
          title="Click to switch between Proxy and Advanced modes"
        >
          <div class="text-sm">
            {#if $runtimeMode === 'Proxy'}
              Switch Mode (Current: Proxy)
            {:else if $runtimeMode === 'Advanced'}  
              Switch Mode (Current: Advanced)
            {:else}
              Switch Mode (Current: Unknown)
            {/if}
          </div>
        </button>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-6">
        <div class="flex items-center gap-3">
          <h3 class="text-xl font-semibold text-gray-700">⚡ Quick Actions</h3>
          <div class="text-sm text-gray-500">Frequently used operations and tools</div>
        </div>
        <div class="text-xs text-gray-400">
          Essential operations for MagicTunnel management
        </div>
      </div>
      
      <!-- Main 4 Quick Actions -->
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <a 
          href="/smart-discovery" 
          class="flex flex-col items-center gap-3 px-6 py-4 bg-gradient-to-br from-blue-50 to-blue-100 hover:from-blue-100 hover:to-blue-200 border border-blue-200 rounded-xl transition-all duration-200 text-center group hover:shadow-md"
        >
          <div class="text-3xl group-hover:scale-110 transition-transform duration-200">🧠</div>
          <div class="text-sm font-semibold text-blue-700">Smart Discovery</div>
          <div class="text-xs text-blue-600">Intelligent tool finder</div>
        </a>
        
        <a 
          href="/mcp-commands" 
          class="flex flex-col items-center gap-3 px-6 py-4 bg-gradient-to-br from-green-50 to-green-100 hover:from-green-100 hover:to-green-200 border border-green-200 rounded-xl transition-all duration-200 text-center group hover:shadow-md"
        >
          <div class="text-3xl group-hover:scale-110 transition-transform duration-200">🔧</div>
          <div class="text-sm font-semibold text-green-700">MCP Commands</div>
          <div class="text-xs text-green-600">Protocol testing</div>
        </a>
        
        <a 
          href="/monitoring" 
          class="flex flex-col items-center gap-3 px-6 py-4 bg-gradient-to-br from-teal-50 to-teal-100 hover:from-teal-100 hover:to-teal-200 border border-teal-200 rounded-xl transition-all duration-200 text-center group hover:shadow-md"
        >
          <div class="text-3xl group-hover:scale-110 transition-transform duration-200">📊</div>
          <div class="text-sm font-semibold text-teal-700">System Monitoring</div>
          <div class="text-xs text-teal-600">Performance metrics</div>
        </a>
        
        <a 
          href="/security" 
          class="flex flex-col items-center gap-3 px-6 py-4 bg-gradient-to-br from-red-50 to-red-100 hover:from-red-100 hover:to-red-200 border border-red-200 rounded-xl transition-all duration-200 text-center group hover:shadow-md"
        >
          <div class="text-3xl group-hover:scale-110 transition-transform duration-200">🛡️</div>
          <div class="text-sm font-semibold text-red-700">Security Overview</div>
          <div class="text-xs text-red-600">Security dashboard</div>
        </a>
      </div>

      <!-- 7 Smaller Quick Action Buttons -->
      <div class="grid grid-cols-1 sm:grid-cols-3 lg:grid-cols-7 gap-3 mb-6">
        <a 
          href="/tools" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">🛠️</div>
          <div class="text-xs font-medium text-gray-700">MCP Tools</div>
        </a>
        
        <a 
          href="/llm-services" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">🤖</div>
          <div class="text-xs font-medium text-gray-700">LLM Services</div>
        </a>
        
        <a 
          href="/security/allowlist" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">✅</div>
          <div class="text-xs font-medium text-gray-700">Allowlisting</div>
        </a>
        
        <a 
          href="/build-commands" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">🔨</div>
          <div class="text-xs font-medium text-gray-700">Build Commands</div>
        </a>
        
        <a 
          href="/services" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">🔌</div>
          <div class="text-xs font-medium text-gray-700">MCP Services</div>
        </a>
        
        <a 
          href="/prompts" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">💬</div>
          <div class="text-xs font-medium text-gray-700">Prompts</div>
        </a>
        
        <a 
          href="/resources" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">📁</div>
          <div class="text-xs font-medium text-gray-700">Resources</div>
        </a>
      </div>

      <!-- Additional Tools Section -->
      <div class="mb-6">
        <h4 class="text-sm font-medium text-gray-600 mb-3 uppercase tracking-wide">Additional Tools</h4>
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
          <a 
            href="/tool-metrics" 
            class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
          >
            <div class="text-xl group-hover:scale-110 transition-transform duration-200">⚡</div>
            <div class="text-xs font-medium text-gray-700">Tool Metrics</div>
          </a>
          
          <a 
            href="/security/audit" 
            class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
          >
            <div class="text-xl group-hover:scale-110 transition-transform duration-200">📋</div>
            <div class="text-xs font-medium text-gray-700">Audit Logs</div>
          </a>
          
          <a 
            href="/logs" 
            class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
          >
            <div class="text-xl group-hover:scale-110 transition-transform duration-200">📜</div>
            <div class="text-xs font-medium text-gray-700">System Logs</div>
          </a>
          
          <a 
            href="/config" 
            class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
          >
            <div class="text-xl group-hover:scale-110 transition-transform duration-200">⚙️</div>
            <div class="text-xs font-medium text-gray-700">Configuration</div>
          </a>
        </div>
      </div>
    </div>

    <!-- Frontend Status -->
    <div class="card">
      <h3 class="text-xl font-semibold text-gray-700 mb-4">Frontend Status</h3>
      {#if systemStatus}
        <div class="p-3 bg-green-50 rounded-md">
          <p class="text-sm text-green-800">
            🎉 <strong>Success!</strong> Frontend is successfully connected to the Rust backend!<br>
            Backend version: {systemStatus.version} | Memory: {systemStatus.memory_usage}
          </p>
        </div>
      {:else}
        <div class="p-3 bg-yellow-50 rounded-md">
          <p class="text-sm text-yellow-800">
            🔄 <strong>Connecting...</strong> Establishing connection to the Rust backend...
          </p>
        </div>
      {/if}
    </div>

  </div>
</div>

<!-- Restart Confirmation Dialog -->
{#if showRestartDialog}
  <div class="restart-dialog-overlay" on:click={closeRestartDialog}>
    <div class="restart-dialog" on:click|stopPropagation>
      <div class="restart-dialog-header">
        <h2 class="restart-dialog-title">🚀 Restart MagicTunnel</h2>
        <button class="restart-dialog-close" on:click={closeRestartDialog}>
          ✕
        </button>
      </div>
      
      <div class="restart-dialog-content">
        <div class="restart-warning">
          <div class="restart-warning-icon">⚠️</div>
          <div class="restart-warning-text">
            <p class="restart-warning-title">Confirm Restart</p>
            <p class="restart-warning-description">
              This will restart MagicTunnel with your specified startup options. 
              The system will be temporarily unavailable during the restart process.
            </p>
          </div>
        </div>

        <div class="startup-options">
          <label class="startup-options-label" for="startup-args">
            Startup Arguments:
          </label>
          
          <!-- Preset Buttons -->
          <div class="preset-buttons">
            <p class="preset-label">Quick Presets:</p>
            <div class="preset-button-group">
              <button 
                class="btn-preset" 
                on:click={() => startupArgs = '--config magictunnel-config.yaml --log-level info'}
                title="Standard production configuration"
              >
                📦 Standard
              </button>
              <button 
                class="btn-preset" 
                on:click={() => startupArgs = '--config magictunnel-config.yaml --log-level debug'}
                title="Development mode with debug logging"
              >
                🔧 Debug
              </button>
              <button 
                class="btn-preset" 
                on:click={() => startupArgs = '--config magictunnel-config.yaml --stdio'}
                title="Stdio mode for MCP clients like Claude Desktop"
              >
                🤖 MCP Client
              </button>
              <button 
                class="btn-preset" 
                on:click={() => startupArgs = '--config magictunnel-config.yaml --log-level trace'}
                title="Maximum logging for troubleshooting"
              >
                🔍 Verbose
              </button>
            </div>
          </div>
          
          <textarea
            id="startup-args"
            bind:value={startupArgs}
            class="startup-args-input"
            rows="3"
            placeholder="Enter startup arguments (e.g., --config magictunnel-config.yaml --log-level info)"
          ></textarea>
          <div class="startup-options-help">
            <p class="help-title">Common Options:</p>
            <div class="help-options">
              <div class="help-option">
                <code>--config &lt;path&gt;</code> - Configuration file path
              </div>
              <div class="help-option">
                <code>--log-level &lt;level&gt;</code> - Log level: trace, debug, info, warn, error
              </div>
              <div class="help-option">
                <code>--stdio</code> - Run in stdio mode for MCP clients
              </div>
              <div class="help-option">
                <code>--host &lt;host&gt;</code> - Server host override
              </div>
              <div class="help-option">
                <code>--port &lt;port&gt;</code> - Server port override
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <div class="restart-dialog-footer">
        <button class="btn-cancel" on:click={closeRestartDialog}>
          Cancel
        </button>
        <button class="btn-confirm-restart" on:click={confirmRestart}>
          🚀 Restart Now
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Tool Execution Modal -->

<style>
  /* Restart Confirmation Dialog */
  .restart-dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }

  .restart-dialog {
    background: white;
    border-radius: 0.5rem;
    box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25);
    max-width: 32rem;
    width: 100%;
    margin: 1rem;
    max-height: 90vh;
    overflow-y: auto;
  }

  .restart-dialog-header {
    padding: 1.5rem 1.5rem 1rem 1.5rem;
    border-bottom: 1px solid #e5e7eb;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .restart-dialog-title {
    font-size: 1.25rem;
    font-weight: 600;
    color: #1f2937;
  }

  .restart-dialog-close {
    color: #9ca3af;
    font-size: 1.25rem;
    line-height: 1;
    cursor: pointer;
    background: none;
    border: none;
  }

  .restart-dialog-close:hover {
    color: #4b5563;
  }

  .restart-dialog-content {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .restart-warning {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    padding: 1rem;
    background: #fef3c7;
    border: 1px solid #fbbf24;
    border-radius: 0.5rem;
  }

  .restart-warning-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .restart-warning-text {
    flex: 1;
  }

  .restart-warning-title {
    font-weight: 600;
    color: #92400e;
    margin-bottom: 0.25rem;
  }

  .restart-warning-description {
    font-size: 0.875rem;
    color: #a16207;
  }

  .startup-options {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .startup-options-label {
    display: block;
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
    margin-bottom: 0.5rem;
  }

  .preset-buttons {
    margin-bottom: 1rem;
  }

  .preset-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
    margin-bottom: 0.5rem;
  }

  .preset-button-group {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .btn-preset {
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    background: #dbeafe;
    color: #1d4ed8;
    border-radius: 0.5rem;
    border: 1px solid #bfdbfe;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .btn-preset:hover {
    background: #bfdbfe;
  }

  .startup-args-input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 0.5rem;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
    font-size: 0.875rem;
    resize: vertical;
  }

  .startup-args-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .startup-options-help {
    margin-top: 1rem;
    padding: 1rem;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 0.5rem;
  }

  .help-title {
    font-weight: 500;
    color: #1f2937;
    margin-bottom: 0.75rem;
  }

  .help-options {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .help-option {
    font-size: 0.875rem;
    color: #4b5563;
  }

  .help-option code {
    background: #e5e7eb;
    color: #1f2937;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  }

  .restart-dialog-footer {
    padding: 1rem 1.5rem 1.5rem 1.5rem;
    border-top: 1px solid #e5e7eb;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
  }

  .btn-cancel {
    padding: 0.5rem 1rem;
    color: #374151;
    border: 1px solid #d1d5db;
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
    background: white;
    transition: background-color 0.2s;
  }

  .btn-cancel:hover {
    background: #f9fafb;
  }

  .btn-confirm-restart {
    padding: 0.5rem 1rem;
    background: #ea580c;
    color: white;
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: background-color 0.2s;
  }

  .btn-confirm-restart:hover {
    background: #c2410c;
  }
</style>