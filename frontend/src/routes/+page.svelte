<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type SystemStatus, type ToolsResponse, type Tool, type CustomCommandSpec, type CustomRestartRequest, type ExecuteCommandRequest, type McpExecuteResponse, type MonitoringHealthStatus, type MonitoringSystemAlerts } from '$lib/api';
  import { runtimeMode, modeStore, type ServiceStatus } from '$lib/stores/mode';
  import { systemMetrics, systemMetricsLoading, systemMetricsService } from '$lib/stores/systemMetrics';
  import { setRestartBanner, setModeSwitchBanner, setSuccessBanner, setErrorBanner, clearBannerOverride } from '$lib/stores/banner';
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
  
  // Mode switch specific state (separate from general restart)
  // Note: modeSwitching is already defined above for the button state
  let modeSwitchCountdown = 0;
  let modeSwitchResult: any = null;
  let showRestartDialog = false;
  let showActiveConnectionsDialog = false;
  let pendingRestartAction: 'restart' | 'mode_switch' = 'restart';
  let pendingTargetMode: string | null = null;
  let startupArgs = '--config magictunnel-config.yaml --log-level info';
  
  // Environment variables for custom restart
  let envVars = [
    { key: 'MAGICTUNNEL_ENV', value: 'development' },
    { key: 'MAGICTUNNEL_RUNTIME_MODE', value: 'advanced' },
    { key: 'MAGICTUNNEL_SEMANTIC_MODEL', value: 'ollama:nomic-embed-text' },
    { key: 'MAGICTUNNEL_DISABLE_SEMANTIC', value: 'false' },
    { key: 'OLLAMA_BASE_URL', value: 'http://localhost:11434' }
  ];
  
  // Add/remove environment variable helpers
  function addEnvVar() {
    envVars = [...envVars, { key: '', value: '' }];
  }
  
  function removeEnvVar(index: number) {
    envVars = envVars.filter((_, i) => i !== index);
  }

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

  // Mode switching state management
  let modeSwitching = false;
  let lastModeSwitchTime = 0;
  let modeSwitchAbortController: AbortController | null = null;
  const MODE_SWITCH_DEBOUNCE_MS = 2000; // 2 second debounce
  
  // Shared mode switching functionality - can be used by any component
  async function switchMode(targetMode?: string) {
    try {
      const currentTime = Date.now();
      
      // Debounce: Prevent multiple rapid mode switch attempts
      if (currentTime - lastModeSwitchTime < MODE_SWITCH_DEBOUNCE_MS) {
        console.warn('Mode switch request debounced - too soon after last attempt');
        return;
      }
      
      // Prevent concurrent mode switch requests
      if (modeSwitching) {
        console.warn('Mode switch already in progress - ignoring request');
        return;
      }
      
      const currentMode = $runtimeMode;
      const newMode = targetMode || (currentMode === 'proxy' ? 'advanced' : 'proxy');
      
      // Don't switch to the same mode
      if (newMode === currentMode) {
        console.log('Already in target mode:', newMode);
        return;
      }
      
      modeSwitching = true;
      loading = true;
      lastModeSwitchTime = currentTime;
      
      // Clear any existing banner and show mode switch banner
      clearBannerOverride();
      setModeSwitchBanner(0, `Preparing to switch from ${currentMode} mode to ${newMode} mode...`);
      
      // Cancel any previous request
      if (modeSwitchAbortController) {
        modeSwitchAbortController.abort();
      }
      
      // Create new abort controller for this request
      modeSwitchAbortController = new AbortController();
      
      console.log(`Switching mode from ${currentMode} to ${newMode}`);
      
      // Use the dedicated mode switching endpoint which delegates to custom restart
      // This ensures all restart functionality and environment variable handling is preserved
      const response = await fetch('/dashboard/api/system/switch-mode', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          mode: newMode
        }),
        signal: modeSwitchAbortController.signal
      });

      // Handle potential JSON parsing errors during server restart
      let result;
      try {
        const responseText = await response.text();
        if (responseText.trim()) {
          result = JSON.parse(responseText);
        } else {
          // Empty response during restart is expected and indicates success
          result = { status: 'success', message: 'Mode switch initiated' };
        }
      } catch (jsonError) {
        // If JSON parsing fails during mode switch, treat as success (server restarting)
        // Mode switches can return 500 errors or incomplete responses due to server restart
        if (response.ok || response.status === 500) {
          console.log('Mode switch initiated - server restarting (JSON parse failed as expected)');
          result = { status: 'success', message: 'Mode switch initiated' };
        } else {
          throw new Error(`JSON parsing failed with status ${response.status}: ${jsonError.message}`);
        }
      }
      
      if ((response.ok || response.status === 500) && result.status === 'success') {
        console.log('Mode switch restart initiated successfully:', result);
        // The system will restart with the new mode via custom restart
        
        // Show success message and trigger mode switch specific reconnection
        setModeSwitchBanner(0, `Mode switch to ${newMode} initiated successfully. System will restart shortly.`);
        error = `Switching to ${newMode} mode... System will restart shortly.`;
        
        // Start mode switch specific reconnection (separate from general restart)
        showModeSwitchReconnection();
        
        // Clear the success message after countdown starts
        setTimeout(() => {
          error = '';
        }, 3000);
      } else {
        throw new Error(result.message || 'Failed to restart system for mode switch');
      }
    } catch (err) {
      if (err.name === 'AbortError') {
        console.log('Mode switch request was aborted');
        return;
      }
      console.error('Failed to switch mode:', err);
      setErrorBanner('Mode Switch Failed', `Failed to switch runtime mode: ${err}`);
      error = `Failed to switch runtime mode: ${err}`;
    } finally {
      modeSwitching = false;
      loading = false;
      modeSwitchAbortController = null;
    }
  }
  
  // Event handler for mode switching events from other components
  function handleModeSwitch(event) {
    console.log('Received modeSwitch event:', event.detail);
    if (event.detail && event.detail.newMode) {
      // Use the dialog-based mode switch instead of direct switchMode()
      initiateModeSwitch(event.detail.newMode);
    } else {
      // Toggle mode with dialog
      initiateModeSwitch();
    }
  }






  function handleManualRefresh() {
    loadDashboardData();
  }

  // Helper function to get current active connections count
  function getActiveConnections(): number {
    return $systemMetrics?.mcp_services?.connections?.active || 0;
  }

  // Always show restart confirmation dialog
  function initiateRestart() {
    pendingRestartAction = 'restart';
    pendingTargetMode = null;
    showActiveConnectionsDialog = true;
  }

  // Always show mode switch confirmation dialog
  function initiateModeSwitch(targetMode?: string) {
    const currentMode = $runtimeMode;
    const newMode = targetMode || (currentMode === 'proxy' ? 'advanced' : 'proxy');
    
    pendingRestartAction = 'mode_switch';
    pendingTargetMode = newMode;
    showActiveConnectionsDialog = true;
  }

  // Handle user's confirmation to proceed with the action
  function confirmDialogAction() {
    showActiveConnectionsDialog = false;
    
    if (pendingRestartAction === 'restart') {
      confirmRestart();
    } else if (pendingRestartAction === 'mode_switch' && pendingTargetMode) {
      switchMode(pendingTargetMode);
    }
    
    // Reset pending action
    pendingRestartAction = 'restart';
    pendingTargetMode = null;
  }

  // Handle user's decision to cancel operation
  function cancelDialogAction() {
    showActiveConnectionsDialog = false;
    pendingRestartAction = 'restart';
    pendingTargetMode = null;
  }



  // MagicTunnel restart functions
  function restartMagicTunnel() {
    // Use smart restart logic that checks for active connections
    initiateRestart();
  }

  function closeRestartDialog() {
    showRestartDialog = false;
  }

  async function confirmRestart() {
    showRestartDialog = false;
    restartingMagicTunnel = true;
    restartResult = null;
    
    // Clear any existing banner and show restart banner
    clearBannerOverride();
    setRestartBanner(0, 'Preparing system restart...');
    
    try {
      // Parse startup args into array
      const args = startupArgs.trim().split(/\s+/).filter(arg => arg.length > 0);
      
      // Build environment variables object from the array, filtering out empty entries
      const envMap = {};
      envVars.forEach(env => {
        if (env.key.trim() && env.value.trim()) {
          envMap[env.key.trim()] = env.value.trim();
        }
      });
      
      // Use direct fetch to handle 500 responses during restart properly
      const requestBody = {
        start_args: args,
        env_vars: Object.keys(envMap).length > 0 ? envMap : undefined
      };
      
      const response = await fetch('/dashboard/api/system/custom-restart', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestBody)
      });

      // Handle potential JSON parsing errors during server restart
      let result;
      try {
        const responseText = await response.text();
        if (responseText.trim()) {
          result = JSON.parse(responseText);
        } else {
          // Empty response during restart is expected and indicates success
          result = { status: 'success', message: 'System restart initiated' };
        }
      } catch (jsonError) {
        // If JSON parsing fails during restart, treat as success (server restarting)
        // Restarts can return 500 errors or incomplete responses due to server restart
        if (response.ok || response.status === 500) {
          console.log('Restart initiated - server restarting (JSON parse failed as expected)');
          result = { status: 'success', message: 'System restart initiated' };
        } else {
          throw new Error(`JSON parsing failed with status ${response.status}: ${jsonError.message}`);
        }
      }
      
      restartResult = result;
      
      if ((response.ok || response.status === 500) && result.status === 'success') {
        setRestartBanner(0, 'System restart initiated successfully. Waiting for service to come back online...');
        showRestartCountdown();
      } else {
        throw new Error(result.message || 'Failed to initiate restart');
      }
    } catch (err) {
      console.error('Restart failed:', err);
      restartResult = {
        action: 'restart_magictunnel',
        status: 'error',
        message: `Failed to restart: ${err}`,
        timestamp: new Date().toISOString()
      };
      setErrorBanner('Restart Failed', `Failed to restart system: ${err}`);
      restartingMagicTunnel = false;
    }
  }

  function showRestartCountdown() {
    restartCountdown = 30; // 30 second max countdown
    const countdown = setInterval(() => {
      restartCountdown--;
      setRestartBanner(restartCountdown, restartCountdown > 0 ? 
        'System restarting... Checking server readiness.' : 
        'Attempting to reconnect to system...');
      
      if (restartCountdown <= 0) {
        clearInterval(countdown);
      }
    }, 1000);
    
    // Start checking for server readiness immediately (with a short delay for server to begin restart)
    setTimeout(() => {
      attemptReconnection(false, countdown);
    }, 3000);
  }

  function showModeSwitchReconnection() {
    // modeSwitching is already set to true by switchMode() function
    // Don't modify restartingMagicTunnel to avoid conflicts with restart button
    
    // Clear any previous error messages
    error = '';
    
    // Set up mode switch countdown (max 30 seconds)
    modeSwitchCountdown = 30;
    modeSwitchResult = {
      action: 'switch_mode',
      status: 'in_progress',
      message: 'System restarting... Please wait while the new mode is applied.',
      timestamp: new Date().toISOString()
    };
    
    const countdown = setInterval(() => {
      modeSwitchCountdown--;
      setModeSwitchBanner(modeSwitchCountdown, modeSwitchCountdown > 0 ? 
        'System restarting with new mode... Checking server readiness.' : 
        'Attempting to reconnect to system with new mode...');
      
      if (modeSwitchCountdown <= 0) {
        clearInterval(countdown);
      }
    }, 1000);
    
    // Start checking for server readiness after a short delay (3 seconds)
    setTimeout(() => {
      attemptModeSwitchReconnection(countdown);
    }, 3000);
  }
  
  async function attemptModeSwitchReconnection(countdownInterval = null) {
    let reconnectAttempts = 0;
    const maxAttempts = 6; // Try for 30 seconds (6 * 5 seconds)
    
    const tryReconnect = async () => {
      try {
        // Check if server is responding with a simple fetch to avoid API client issues
        const healthResponse = await fetch('/dashboard/api/status', {
          method: 'GET',
          headers: { 'Accept': 'application/json' }
        });
        
        if (!healthResponse.ok) {
          throw new Error(`Server not ready: ${healthResponse.status}`);
        }
        
        // Server is responding, now check if mode has actually changed
        const modeResponse = await fetch('/dashboard/api/mode', {
          method: 'GET',
          headers: { 'Accept': 'application/json' }
        });
        
        if (!modeResponse.ok) {
          throw new Error(`Mode API not ready: ${modeResponse.status}`);
        }
        
        const modeData = await modeResponse.json();
        console.log('Server back online, detected mode:', modeData.runtime_mode);
        
        // Server is responding - clear countdown and complete mode switch
        if (countdownInterval) {
          clearInterval(countdownInterval);
        }
        
        // Reset the mode switch state and show success result
        modeSwitching = false;
        modeSwitchResult = {
          action: 'switch_mode',
          status: 'success',
          message: `Mode switch successful! System switched to ${modeData.runtime_mode} mode and is now responding.`,
          timestamp: new Date().toISOString()
        };
        
        // Show success banner
        setSuccessBanner('Mode Switch Complete', `Successfully switched to ${modeData.runtime_mode} mode. Page will reload shortly.`);
        
        // Wait a moment to show success message, then reload
        setTimeout(() => {
          // Force a complete page reload to ensure all components reflect the new mode
          // This is the most reliable way to ensure everything updates correctly
          window.location.reload();
        }, 1500);
        
        // IMPORTANT: Return early to prevent further retry attempts
        return;
        
      } catch (err) {
        console.log(`Mode switch reconnection attempt ${reconnectAttempts + 1} failed:`, err.message);
        reconnectAttempts++;
        
        if (reconnectAttempts < maxAttempts) {
          setTimeout(tryReconnect, 5000); // Try again in 5 seconds
        } else {
          // Clear countdown on failure too
          if (countdownInterval) {
            clearInterval(countdownInterval);
          }
          
          modeSwitching = false;
          modeSwitchResult = {
            action: 'switch_mode',
            status: 'error',
            message: 'Failed to reconnect after mode switch. Please refresh the page manually.',
            timestamp: new Date().toISOString()
          };
          
          // Show error banner
          setErrorBanner('Mode Switch Timeout', 'Failed to reconnect after mode switch. Please refresh the page manually.');
        }
      }
    };
    
    tryReconnect();
  }

  async function attemptReconnection(isModeSwitch = false, countdownInterval = null) {
    let reconnectAttempts = 0;
    const maxAttempts = 6; // Try for 30 seconds (6 * 5 seconds)
    let reconnectionComplete = false; // Flag to prevent timeout errors after success
    
    const tryReconnect = async () => {
      try {
        // Use direct fetch to avoid API client issues during restart
        const healthResponse = await fetch('/dashboard/api/status', {
          method: 'GET',
          headers: { 'Accept': 'application/json' }
        });
        
        if (!healthResponse.ok) {
          throw new Error(`Server not ready: ${healthResponse.status}`);
        }
        
        // Server is responding - clear countdown and complete restart
        if (countdownInterval) {
          clearInterval(countdownInterval);
        }
        
        restartingMagicTunnel = false;
        restartResult = {
          action: isModeSwitch ? 'switch_mode' : 'restart_magictunnel',
          status: 'success',
          message: isModeSwitch 
            ? `MagicTunnel successfully switched to ${$runtimeMode} mode and is now responding`
            : 'MagicTunnel restarted successfully and is now responding',
          timestamp: new Date().toISOString()
        };
        
        // Mark reconnection as complete to prevent timeout errors
        reconnectionComplete = true;
        
        // For both restart and mode switch, show success banner and reload page
        if (isModeSwitch) {
          setSuccessBanner('Mode Switch Complete', `Successfully switched to ${$runtimeMode} mode and system is online.`);
        } else {
          setSuccessBanner('Restart Complete', 'System restarted successfully and is now online.');
        }
        
        // Wait a moment to show success message, then reload for both cases
        setTimeout(() => {
          // Force a complete page reload to ensure all components reflect any changes
          // This is the most reliable way to ensure everything updates correctly
          window.location.reload();
        }, 1500);
        
        // IMPORTANT: Return early to prevent further retry attempts
        return;
        
      } catch (err) {
        reconnectAttempts++;
        if (reconnectAttempts < maxAttempts) {
          setTimeout(tryReconnect, 5000); // Try again in 5 seconds
        } else {
          // Only show timeout error if reconnection hasn't already completed successfully
          if (!reconnectionComplete) {
            // Clear countdown on failure too
            if (countdownInterval) {
              clearInterval(countdownInterval);
            }
            
            restartingMagicTunnel = false;
            restartResult = {
              action: isModeSwitch ? 'switch_mode' : 'restart_magictunnel',
              status: 'error',
              message: isModeSwitch 
                ? 'Failed to reconnect to MagicTunnel after mode switch. Please check if the service is running.'
                : 'Failed to reconnect to MagicTunnel after restart. Please check if the service is running.',
              timestamp: new Date().toISOString()
            };
            
            // Show error banner
            if (isModeSwitch) {
              setErrorBanner('Mode Switch Timeout', 'Failed to reconnect after mode switch. Please check if the service is running.');
            } else {
              setErrorBanner('Restart Timeout', 'Failed to reconnect after restart. Please check if the service is running.');
            }
          }
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

  // Connection-aware mode switching - uses smart logic to check active connections
  function switchModeButton() {
    initiateModeSwitch(); // Use the connection-aware version
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
    
    // Add global mode switch event listener for components to use
    window.addEventListener('modeSwitch', handleModeSwitch);
    
    
    // Countdown timer - updates every second
    const countdownInterval = setInterval(() => {
      nextRefreshIn = Math.max(0, nextRefreshIn - 1);
      
      // Trigger refresh when countdown reaches 0
      if (nextRefreshIn === 0) {
        // Normal refresh - just update data
        loadDashboardData();
        loadMonitoringData();
        nextRefreshIn = 30; // Reset countdown for next cycle
      }
    }, 1000);
    
    return () => {
      clearInterval(countdownInterval);
      systemMetricsService.stop(); // Stop shared metrics service
      window.removeEventListener('modeSwitch', handleModeSwitch);
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
        <!-- Status messages now appear in the top banner via ModeAwareLayout -->

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
          on:click={switchModeButton}
          disabled={loading || modeSwitching}
          title="Click to switch between Proxy and Advanced modes"
        >
          {#if modeSwitching}
            <span class="animate-spin">⟳</span> Switching Mode...
          {:else}
            <div class="text-sm">
              {#if $runtimeMode === 'proxy'}
                Switch Mode (Current: Proxy)
              {:else if $runtimeMode === 'advanced'}  
                Switch Mode (Current: Advanced)
              {:else}
                Switch Mode (Current: Unknown)
              {/if}
            </div>
          {/if}
        </button>
      </div>
    </div>

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

      <a href="/mcp-servers" class="card hover:bg-gray-50 transition-colors cursor-pointer">
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
                    {$runtimeMode === 'proxy' ? 'Switch to Advanced mode to enable' : 'No advanced services detected'}
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
      <div class="flex flex-wrap justify-center gap-4 mb-6">
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
        
        {#if $runtimeMode === 'advanced'}
        <a 
          href="/security" 
          class="flex flex-col items-center gap-3 px-6 py-4 bg-gradient-to-br from-red-50 to-red-100 hover:from-red-100 hover:to-red-200 border border-red-200 rounded-xl transition-all duration-200 text-center group hover:shadow-md"
        >
          <div class="text-3xl group-hover:scale-110 transition-transform duration-200">🛡️</div>
          <div class="text-sm font-semibold text-red-700">Security Overview</div>
          <div class="text-xs text-red-600">Security dashboard</div>
        </a>
        {/if}
      </div>

      <!-- 7 Smaller Quick Action Buttons -->
      <div class="flex flex-wrap justify-center gap-3 mb-6">
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
        
        {#if $runtimeMode === 'advanced'}
        <a 
          href="/security/allowlist" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">🛡️</div>
          <div class="text-xs font-medium text-gray-700">Tool Allowlist</div>
        </a>
        {/if}
        
        <a 
          href="/build-commands" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">🔨</div>
          <div class="text-xs font-medium text-gray-700">Build Commands</div>
        </a>
        
        <a 
          href="/mcp-servers" 
          class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
        >
          <div class="text-xl group-hover:scale-110 transition-transform duration-200">🔌</div>
          <div class="text-xs font-medium text-gray-700">MCP Servers</div>
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
        <h4 class="text-sm font-medium text-gray-600 mb-3 uppercase tracking-wide text-center">Additional Tools</h4>
        <div class="flex flex-wrap justify-center gap-3">
          <a 
            href="/tool-metrics" 
            class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
          >
            <div class="text-xl group-hover:scale-110 transition-transform duration-200">⚡</div>
            <div class="text-xs font-medium text-gray-700">Tool Metrics</div>
          </a>
          
          {#if $runtimeMode === 'advanced'}
          <a 
            href="/security/audit" 
            class="flex flex-col items-center gap-2 px-3 py-3 bg-white hover:bg-gray-50 border border-gray-200 hover:border-gray-300 rounded-lg transition-all duration-200 text-center group hover:shadow-sm"
          >
            <div class="text-xl group-hover:scale-110 transition-transform duration-200">📋</div>
            <div class="text-xs font-medium text-gray-700">Audit Logs</div>
          </a>
          {/if}
          
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

        <!-- Environment Variables Section -->
        <div class="env-vars-section">
          <label class="env-vars-label">
            Environment Variables:
          </label>
          
          <div class="env-vars-container">
            {#each envVars as envVar, index}
              <div class="env-var-row">
                <input
                  type="text"
                  class="env-var-key"
                  placeholder="Variable name (e.g., MAGICTUNNEL_ENV)"
                  bind:value={envVar.key}
                />
                <span class="env-var-separator">=</span>
                <input
                  type="text"
                  class="env-var-value"
                  placeholder="Variable value (e.g., development)"
                  bind:value={envVar.value}
                />
                <button 
                  class="env-var-remove" 
                  on:click={() => removeEnvVar(index)}
                  title="Remove environment variable"
                  type="button"
                >
                  ✕
                </button>
              </div>
            {/each}
            
            <button 
              class="env-var-add" 
              on:click={addEnvVar}
              title="Add environment variable"
              type="button"
            >
              + Add Environment Variable
            </button>
          </div>
          
          <div class="env-vars-help">
            <p class="help-title">Common Environment Variables:</p>
            <div class="help-options">
              <div class="help-option">
                <code>MAGICTUNNEL_ENV</code> - Runtime environment (development, production)
              </div>
              <div class="help-option">
                <code>MAGICTUNNEL_RUNTIME_MODE</code> - Service mode (proxy, advanced)
              </div>
              <div class="help-option">
                <code>MAGICTUNNEL_SMART_DISCOVERY</code> - Enable smart discovery (true, false)
              </div>
              <div class="help-option">
                <code>MAGICTUNNEL_SEMANTIC_MODEL</code> - Semantic model (ollama:nomic-embed-text)
              </div>
              <div class="help-option">
                <code>MAGICTUNNEL_DISABLE_SEMANTIC</code> - Disable semantic search (true, false)
              </div>
              <div class="help-option">
                <code>OLLAMA_BASE_URL</code> - Ollama API endpoint (http://localhost:11434)
              </div>
              <div class="help-option">
                <code>OPENAI_API_KEY</code> - OpenAI API key for smart discovery
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

<!-- Confirmation Dialog with Connection Awareness -->
{#if showActiveConnectionsDialog}
  <div class="restart-dialog-overlay" on:click={cancelDialogAction}>
    <div class="restart-dialog" on:click|stopPropagation>
      <div class="restart-dialog-header">
        <h2 class="restart-dialog-title">
          {#if pendingRestartAction === 'mode_switch'}
            🔄 Confirm Mode Switch
          {:else}
            🚀 Confirm System Restart
          {/if}
        </h2>
        <button class="restart-dialog-close" on:click={cancelDialogAction}>&times;</button>
      </div>
      
      <div class="restart-dialog-content">
        <!-- Basic confirmation message -->
        <div class="text-gray-700 mb-4">
          {#if pendingRestartAction === 'mode_switch'}
            <p>Are you sure you want to switch to <strong>{pendingTargetMode}</strong> mode?</p>
            <p class="text-sm mt-2 text-gray-600">The system will restart with the new runtime configuration.</p>
          {:else}
            <p>Are you sure you want to restart MagicTunnel?</p>
            <p class="text-sm mt-2 text-gray-600">The system will restart and reconnect automatically.</p>
          {/if}
        </div>

        <!-- Connection warning (only shown when connections > 0) -->
        {#if getActiveConnections() > 0}
          <div class="restart-warning">
            <div class="restart-warning-icon">⚠️</div>
            <div class="restart-warning-text">
              <div class="restart-warning-title">
                {getActiveConnections()} Active MCP Connection{getActiveConnections() !== 1 ? 's' : ''} Detected
              </div>
              <div class="restart-warning-description">
                There {getActiveConnections() === 1 ? 'is' : 'are'} currently {getActiveConnections()} active connection{getActiveConnections() !== 1 ? 's' : ''} that will be disconnected.
              </div>
            </div>
          </div>
          
          <div class="text-sm text-gray-600 mt-3">
            <p><strong>Impact:</strong></p>
            <ul class="mt-1 list-disc list-inside space-y-1">
              <li>Connected clients (Claude Desktop, Cursor, etc.) will be disconnected</li>
              <li>Clients will need to reconnect after the system comes back online</li>
              <li>Any ongoing operations may be interrupted</li>
            </ul>
          </div>
        {:else}
          <div class="text-sm text-green-700 bg-green-50 p-3 rounded border border-green-200">
            ✅ <strong>Safe to proceed:</strong> No active connections detected.
          </div>
        {/if}
      </div>
      
      <div class="restart-dialog-footer">
        <button 
          class="restart-dialog-cancel" 
          on:click={cancelDialogAction}
        >
          Cancel
        </button>
        <button 
          class="{getActiveConnections() > 0 ? 'restart-dialog-confirm-danger' : 'restart-dialog-confirm'}" 
          on:click={confirmDialogAction}
        >
          {#if pendingRestartAction === 'mode_switch'}
            🔄 Switch Mode
          {:else}
            🚀 Restart System
          {/if}
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

  /* Restart dialog button styles */
  .restart-dialog-cancel {
    padding: 0.75rem 1.5rem;
    color: #374151;
    border: 1px solid #d1d5db;
    border-radius: 0.5rem;
    font-weight: 500;
    cursor: pointer;
    background: white;
    transition: background-color 0.2s;
  }

  .restart-dialog-cancel:hover {
    background: #f9fafb;
  }

  .restart-dialog-confirm-danger {
    padding: 0.75rem 1.5rem;
    background: #dc2626;
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .restart-dialog-confirm-danger:hover {
    background: #b91c1c;
  }

  .restart-dialog-confirm {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 0.5rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .restart-dialog-confirm:hover {
    background: #2563eb;
  }

  /* Environment Variables Section */
  .env-vars-section {
    border-top: 1px solid #e5e7eb;
    padding-top: 1.5rem;
  }

  .env-vars-label {
    display: block;
    font-weight: 600;
    color: #1f2937;
    margin-bottom: 0.75rem;
    font-size: 0.875rem;
  }

  .env-vars-container {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .env-var-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .env-var-key,
  .env-var-value {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    transition: border-color 0.2s;
  }

  .env-var-key:focus,
  .env-var-value:focus {
    outline: none;
    border-color: #3b82f6;
    ring: 2px;
    ring-color: rgba(59, 130, 246, 0.1);
  }

  .env-var-separator {
    color: #6b7280;
    font-weight: 600;
    font-size: 0.875rem;
  }

  .env-var-remove {
    background: #ef4444;
    color: white;
    border: none;
    border-radius: 0.375rem;
    width: 2rem;
    height: 2rem;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 0.875rem;
    transition: background-color 0.2s;
  }

  .env-var-remove:hover {
    background: #dc2626;
  }

  .env-var-add {
    background: #10b981;
    color: white;
    border: none;
    border-radius: 0.375rem;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.2s;
    align-self: flex-start;
  }

  .env-var-add:hover {
    background: #059669;
  }

  .env-vars-help {
    background: #f9fafb;
    border-radius: 0.375rem;
    padding: 1rem;
    border: 1px solid #e5e7eb;
  }

  .env-vars-help .help-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: #1f2937;
    margin-bottom: 0.75rem;
  }

  .env-vars-help .help-options {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .env-vars-help .help-option {
    display: flex;
    align-items: center;
    font-size: 0.875rem;
    color: #374151;
  }

  .env-vars-help .help-option code {
    background: white;
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
    border: 1px solid #d1d5db;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 0.75rem;
    color: #1f2937;
    margin-right: 0.5rem;
    min-width: 12rem;
  }
</style>