<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type SystemStatus, type ToolsResponse, type Tool, type MakefileCommandsResponse, type MakefileCommand, type MakefileExecuteResponse, type CustomCommandSpec, type CustomRestartRequest, type ExecuteCommandRequest, type McpExecuteResponse, type MonitoringSystemMetrics, type MonitoringHealthStatus, type MonitoringSystemAlerts } from '$lib/api';
  import ToolExecutionModal from '$lib/components/ToolExecutionModal.svelte';
  import SmartDiscoveryVisualizer from '$lib/components/SmartDiscoveryVisualizer.svelte';
  import SystemMetricsCard from '$lib/components/SystemMetricsCard.svelte';
  import HealthStatusCard from '$lib/components/HealthStatusCard.svelte';
  import ToolMetricsCompact from '$lib/components/ToolMetricsCompact.svelte';

  let systemStatus: SystemStatus | null = null;
  let toolsData: ToolsResponse | null = null;
  let loading = true;
  let error = '';
  let nextRefreshIn = 30;
  
  // Monitoring data
  let monitoringMetrics: MonitoringSystemMetrics | null = null;
  let healthStatus: MonitoringHealthStatus | null = null;
  let systemAlerts: MonitoringSystemAlerts | null = null;
  let monitoringLoading = false;
  
  // Modal state
  let isModalOpen = false;
  let smartDiscoveryTool: Tool | null = null;
  
  // Enhanced Smart Discovery state
  let discoveryRequest = '';
  let discoveryResult: any = null;
  let discoveryLoading = false;
  let discoveryError = '';
  let showSmartDiscovery = true; // Show by default
  
  // Execution mode state
  let executionMode = 'http'; // 'http', 'mcp', 'stdio' (simulate Claude)

  // Makefile commands state
  let makefileCommands: MakefileCommandsResponse | null = null;
  let makefileLoading = true;
  let executingCommand: string | null = null;
  let lastCommandResult: MakefileExecuteResponse | null = null;
  let expandedCommands: Set<string> = new Set();

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

  // MCP testing state
  let showMcpTester = false;
  let mcpMethod = '';
  let mcpParams = '';
  let executingMcpCommand = false;
  let mcpResult: McpExecuteResponse | null = null;

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
      
      // Find the smart_tool_discovery tool for the modal
      if (tools?.tools) {
        smartDiscoveryTool = tools.tools.find(tool => tool.name === 'smart_tool_discovery') || null;
      }
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
      // Load monitoring data in parallel
      const [metrics, health, alerts] = await Promise.all([
        api.getSystemMetrics().catch(() => null),
        api.getHealthStatus().catch(() => null),
        api.getSystemAlerts().catch(() => null)
      ]);
      
      monitoringMetrics = metrics;
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

  function handleTestTool() {
    showSmartDiscovery = !showSmartDiscovery;
  }

  async function runSmartDiscovery() {
    if (!discoveryRequest.trim()) {
      discoveryError = 'Please enter a discovery request';
      return;
    }

    discoveryLoading = true;
    discoveryError = '';
    discoveryResult = null;

    try {
      // Use selected execution mode
      let result;
      if (executionMode === 'mcp') {
        result = await api.executeToolMcp('smart_tool_discovery', {
          request: discoveryRequest.trim(),
          confidence_threshold: 0.5
        });
      } else if (executionMode === 'stdio') {
        result = await api.executeToolStdio('smart_tool_discovery', {
          request: discoveryRequest.trim(),
          confidence_threshold: 0.5
        });
      } else {
        result = await api.executeToolTest('smart_tool_discovery', {
          request: discoveryRequest.trim(),
          confidence_threshold: 0.5
        });
      }
      
      // Handle different response structures between HTTP, MCP, and stdio
      if (executionMode === 'mcp' || executionMode === 'stdio') {
        // MCP mode - extract result and map to expected structure
        const mcpResult = result.result || result;
        
        // Map MCP structure to match what visualizer expects
        discoveryResult = {
          // Extract output from MCP content structure
          output: mcpResult.content?.[0]?.text || 'No output available',
          
          // MCP/stdio doesn't provide these fields, so use fallbacks
          original_tool: `smart_tool_discovery (via ${executionMode.toUpperCase()})`,
          execution_time: 'N/A',
          confidence_score: 0.85, // Default confidence for MCP/stdio calls
          reasoning: `Tool executed successfully via ${executionMode.toUpperCase()} protocol`,
          discovery_reasoning: `Tool executed successfully via ${executionMode.toUpperCase()} protocol`,
          
          // Include MCP-specific fields
          is_error: mcpResult.is_error || !mcpResult.success,
          success: mcpResult.success,
          
          // Execution mode for internal tracking
          _execution_mode: 'mcp'
        };
      } else {
        // HTTP mode - extract and clean up the response structure
        const httpResult = result.result || result;
        
        // Clean up duplicated data structure and create a normalized response
        discoveryResult = {
          // Core execution data
          output: httpResult.output,
          original_tool: httpResult.original_tool,
          execution_time: httpResult.execution_time,
          confidence_score: httpResult.confidence_score,
          reasoning: httpResult.reasoning,
          discovery_reasoning: httpResult.metadata?.discovery_reasoning || httpResult.reasoning,
          
          // Status and error handling
          is_error: httpResult.is_error,
          status: httpResult.status,
          
          // Use the primary tool_candidates array (avoid duplicates)
          tool_candidates: httpResult.tool_candidates || httpResult.metadata?.tool_candidates || [],
          
          // Metadata - use flattened structure from metadata field to avoid duplication
          metadata: {
            executed_via: httpResult.metadata?.executed_via,
            server_name: httpResult.metadata?.server_name,
            routing_type: httpResult.metadata?.routing_type,
            registry_lookup: httpResult.metadata?.registry_lookup,
            source: httpResult.metadata?.source,
            validated: httpResult.metadata?.validated
          },
          
          // Execution mode for internal tracking
          _execution_mode: 'http'
        };
      }
    } catch (err) {
      discoveryError = `Discovery failed: ${err}`;
      console.error('Smart Discovery error:', err);
    } finally {
      discoveryLoading = false;
    }
  }

  function handleExecuteTool(event: CustomEvent) {
    const { toolName, parameters } = event.detail;
    // Execute the discovered tool
    api.executeToolTest(toolName, parameters).then(result => {
      alert(`Tool executed successfully:\n\n${JSON.stringify(result, null, 2)}`);
    }).catch(err => {
      alert(`Tool execution failed: ${err}`);
    });
  }

  async function handleShowToolDetails(event: CustomEvent) {
    const toolName = event.detail;
    console.log('handleShowToolDetails called with tool:', toolName);
    
    try {
      // Load the tool details from the tools API
      console.log('Loading tools from API...');
      const toolsResponse = await api.getTools();
      console.log('Tools response:', toolsResponse);
      
      const tool = toolsResponse.tools.find(t => t.name === toolName);
      console.log('Found tool:', tool);
      
      if (tool) {
        // Set the tool for the modal and open it
        smartDiscoveryTool = tool;
        isModalOpen = true;
        console.log('Modal should be open now with tool:', tool.name);
      } else {
        console.error(`Tool "${toolName}" not found in tools:`, toolsResponse.tools.map(t => t.name));
        alert(`Tool "${toolName}" not found in the tools registry`);
      }
    } catch (err) {
      alert(`Failed to load tool details: ${err}`);
      console.error('Failed to load tool details:', err);
    }
  }

  function closeModal() {
    isModalOpen = false;
  }

  async function handleToolExecution(event: CustomEvent) {
    const { tool, arguments: args } = event.detail;
    
    try {
      const result = await api.executeToolTest(tool.name, args);
      
      // Show result in a nice format
      const resultText = JSON.stringify(result, null, 2);
      alert(`Smart Discovery Result:\n\n${resultText}`);
      
      closeModal();
    } catch (err) {
      alert(`Smart Discovery Test Failed:\n\n${err}`);
    }
  }


  function handleViewLogs() {
    window.location.href = '/logs';
  }

  function handleManualRefresh() {
    loadDashboardData();
  }

  // Makefile commands functions
  async function loadMakefileCommands() {
    makefileLoading = true;
    try {
      makefileCommands = await api.getMakefileCommands();
    } catch (err) {
      console.error('Failed to load Makefile commands:', err);
      makefileCommands = null;
    }
    makefileLoading = false;
  }

  async function executeMakefileCommand(command: string) {
    if (executingCommand) return; // Prevent multiple executions
    
    executingCommand = command;
    try {
      const result = await api.executeMakefileCommand(command);
      lastCommandResult = result;
    } catch (err) {
      console.error('Makefile command execution failed:', err);
      lastCommandResult = {
        action: 'execute_makefile',
        command,
        status: 'error',
        message: `Failed to execute command: ${err}`,
        timestamp: new Date().toISOString()
      };
    }
    executingCommand = null;
  }

  function toggleCommandScript(commandName: string) {
    const newExpanded = new Set(expandedCommands);
    if (newExpanded.has(commandName)) {
      newExpanded.delete(commandName);
    } else {
      newExpanded.add(commandName);
    }
    expandedCommands = newExpanded;
  }

  function toggleMcpTester() {
    console.log('toggleMcpTester called, current state:', showMcpTester);
    showMcpTester = !showMcpTester;
    if (!showMcpTester) {
      // Reset state when closing
      mcpMethod = '';
      mcpParams = '';
      mcpResult = null;
    }
  }

  async function executeMcpCommand() {
    if (!mcpMethod.trim() || executingMcpCommand) return;

    executingMcpCommand = true;
    mcpResult = null;

    try {
      let params = null;
      if (mcpParams.trim()) {
        try {
          params = JSON.parse(mcpParams);
        } catch (e) {
          throw new Error(`Invalid JSON params: ${e}`);
        }
      }

      mcpResult = await api.executeMcpCommand(mcpMethod.trim(), params);
    } catch (err) {
      console.error('MCP command execution failed:', err);
      mcpResult = {
        action: 'execute_mcp',
        method: mcpMethod.trim(),
        status: 'error',
        message: `Failed to execute MCP command: ${err}`,
        timestamp: new Date().toISOString()
      };
    }
    executingMcpCommand = false;
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

  // Custom restart workflow functions
  function getCategoryIcon(category: string): string {
    switch (category) {
      case 'build': return '🔨';
      case 'test': return '🧪';
      case 'quality': return '✨';
      case 'maintenance': return '🔧';
      case 'docs': return '📚';
      default: return '⚙️';
    }
  }

  function getCommandStatusColor(status: string): string {
    switch (status) {
      case 'success': return 'bg-green-100 text-green-800';
      case 'error': return 'bg-red-100 text-red-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  }

  onMount(() => {
    loadDashboardData();
    loadMakefileCommands();
    loadMonitoringData(); // Load monitoring data
    
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
        <h3 class="text-lg font-semibold text-gray-700 mb-2">Environment Variables</h3>
        <div class="space-y-1 max-h-64 overflow-y-auto">
          {#if systemStatus?.environment}
            {#each Object.entries(systemStatus.environment) as [key, value]}
              <div class="flex items-center justify-between text-xs">
                <span class="text-gray-600 font-mono">{key.toUpperCase().replace(/_/g, '_')}</span>
                <div class="flex items-center gap-1">
                  <span class="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium {
                    typeof value === 'boolean' ? 
                      (value ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600') :
                      'bg-blue-100 text-blue-700'
                  }">
                    {typeof value === 'boolean' ? 
                      (value ? '✅' : '❌') : 
                      (typeof value === 'string' && value.length > 20 ? value.substring(0, 17) + '...' : value)
                    }
                  </span>
                  
                  <!-- Copy button for specific API keys and URLs -->
                  {#if (key === 'openai_api_key_set' && value === true) || 
                       (key === 'anthropic_api_key_set' && value === true) ||
                       (key === 'ollama_base_url' && value && typeof value === 'string') ||
                       (key === 'magictunnel_semantic_model' && value && typeof value === 'string')}
                    <button 
                      class="p-0.5 text-gray-400 hover:text-gray-600 transition-colors"
                      on:click={() => {
                        let label = '';
                        let copyValue = '';
                        
                        if (key === 'openai_api_key_set') {
                          label = 'OpenAI API Key';
                          copyValue = getCopyableValue('openai_api_key_set');
                        } else if (key === 'anthropic_api_key_set') {
                          label = 'Anthropic API Key';
                          copyValue = getCopyableValue('anthropic_api_key_set');
                        } else if (key === 'ollama_base_url') {
                          label = 'Ollama Base URL';
                          copyValue = value; // Use the value directly since it's the actual URL
                        } else if (key === 'magictunnel_semantic_model') {
                          label = 'Semantic Model';
                          copyValue = value; // Use the value directly since it's the actual model name
                        }
                        
                        copyToClipboard(copyValue, label);
                      }}
                      title="Copy to clipboard"
                    >
                      📋
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          {:else}
            <div class="text-sm text-gray-500">Loading environment data...</div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Monitoring Overview -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-6">
        <h3 class="text-xl font-semibold text-gray-700">📊 System Monitoring</h3>
        <div class="flex items-center gap-3">
          {#if systemAlerts && systemAlerts.total_alerts > 0}
            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium
              {systemAlerts.critical_count > 0 ? 'bg-red-100 text-red-800' :
               systemAlerts.error_count > 0 ? 'bg-orange-100 text-orange-800' :
               'bg-yellow-100 text-yellow-800'}">
              {systemAlerts.total_alerts} {systemAlerts.total_alerts === 1 ? 'alert' : 'alerts'}
            </span>
          {:else if systemAlerts}
            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
              All Clear
            </span>
          {/if}
          <a 
            href="/monitoring" 
            class="text-sm text-blue-600 hover:text-blue-800 underline"
          >
            View Full Dashboard →
          </a>
        </div>
      </div>
      
      <!-- Compact Monitoring Cards -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- System Metrics -->
        <SystemMetricsCard 
          metrics={monitoringMetrics} 
          loading={monitoringLoading}
          uptime={monitoringMetrics ? `${Math.floor(monitoringMetrics.uptime_seconds / 3600)}h ${Math.floor((monitoringMetrics.uptime_seconds % 3600) / 60)}m` : null}
        />
        
        <!-- Health Status -->
        <HealthStatusCard 
          healthStatus={healthStatus}
          loading={monitoringLoading}
        />
        
        <!-- Tool Metrics -->
        <ToolMetricsCompact />
      </div>

      <!-- Quick Alerts Summary -->
      {#if systemAlerts && systemAlerts.total_alerts > 0}
        <div class="mt-6 pt-4 border-t border-gray-100">
          <div class="flex items-center justify-between mb-3">
            <h4 class="text-lg font-medium text-gray-700">Recent Alerts</h4>
            <a href="/monitoring" class="text-sm text-blue-600 hover:text-blue-800 underline">
              View All
            </a>
          </div>
          <div class="space-y-2">
            {#each systemAlerts.alerts.slice(0, 3) as alert}
              <div class="flex items-center p-3 bg-gray-50 rounded-lg">
                <span class="mr-3 text-lg">
                  {alert.severity === 'critical' ? '🚨' : 
                   alert.severity === 'error' ? '❌' : '⚠️'}
                </span>
                <div class="flex-1">
                  <div class="text-sm font-medium text-gray-900">{alert.title}</div>
                  <div class="text-xs text-gray-500">{alert.description}</div>
                </div>
                <span class="text-xs text-gray-400">
                  {new Date(alert.timestamp).toLocaleTimeString()}
                </span>
              </div>
            {/each}
            {#if systemAlerts.alerts.length > 3}
              <div class="text-center pt-2">
                <a href="/monitoring" class="text-sm text-blue-600 hover:text-blue-800 underline">
                  +{systemAlerts.alerts.length - 3} more alerts
                </a>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- Quick Actions -->
    <div class="card mb-8">
      <h3 class="text-xl font-semibold text-gray-700 mb-4">Quick Actions</h3>
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
        <button class="btn-secondary" on:click={() => showSmartDiscovery = !showSmartDiscovery}>
          {#if showSmartDiscovery}
            🧠 Hide Smart Discovery
          {:else}
            🧠 Show Smart Discovery
          {/if}
        </button>
        <button 
          class="btn-secondary" 
          on:click={() => {
            console.log('MCP button clicked');
            toggleMcpTester();
          }}
        >
          🔧 Test MCP Commands
        </button>
        <a href="/config" class="btn-secondary text-center">
          ⚙️ Configuration
        </a>
        <a href="/monitoring" class="btn-secondary text-center">
          📊 Monitoring
        </a>
        <a href="/tool-metrics" class="btn-secondary text-center">
          🔧 Tool Metrics
        </a>
        <button class="btn-secondary" on:click={handleViewLogs}>
          📋 Logs
        </button>
      </div>
      
      <!-- MCP Protocol Management -->
      <div class="mt-6">
        <h4 class="text-lg font-medium text-gray-700 mb-3">MCP Protocol Management</h4>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
          <a href="/resources" class="btn-secondary text-center">
            📁 MCP Resources
          </a>
          <a href="/prompts" class="btn-secondary text-center">
            💬 MCP Prompts
          </a>
          <a href="/services" class="btn-secondary text-center">
            🔗 MCP Services
          </a>
          <a href="/tools" class="btn-secondary text-center">
            🛠️ MCP Tools
          </a>
          <a href="/llm-services" class="btn-secondary text-center">
            🧠 LLM Services
          </a>
        </div>
      </div>
    </div>

    <!-- Enhanced Smart Discovery Section -->
    {#if showSmartDiscovery}
      <div class="card mb-8">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-xl font-semibold text-gray-700">🧠 Smart Tool Discovery</h3>
          <div class="flex items-center gap-4">
            <!-- Execution Mode Selector -->
            <div class="flex items-center gap-2">
              <span class="text-xs text-gray-600">Mode:</span>
              <select bind:value={executionMode} class="text-sm bg-white border border-gray-300 rounded px-2 py-1 focus:outline-none focus:ring-2 focus:ring-blue-500">
                <option value="http">HTTP API</option>
                <option value="mcp">MCP Client</option>
                <option value="stdio">Simulate Claude</option>
              </select>
            </div>
            <button 
              class="text-gray-400 hover:text-gray-600 text-2xl"
              on:click={() => showSmartDiscovery = false}
              title="Close Smart Discovery"
            >
              ×
            </button>
          </div>
        </div>
        
        <!-- Discovery Input -->
        <div class="mb-6">
          <div class="flex gap-3">
            <input
              type="text"
              bind:value={discoveryRequest}
              placeholder="Describe what you want to do (e.g., 'ping google.com', 'check website status', 'search for files')"
              class="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent text-lg"
              on:keypress={(e) => e.key === 'Enter' && runSmartDiscovery()}
              disabled={discoveryLoading}
            />
            <button
              class="btn-primary px-6"
              on:click={runSmartDiscovery}
              disabled={discoveryLoading || !discoveryRequest.trim()}
            >
              {discoveryLoading ? '🔍 Analyzing...' : '🔍 Discover'}
            </button>
          </div>
          
          {#if discoveryError && !discoveryLoading}
            <div class="mt-3 text-sm text-red-600 bg-red-50 border border-red-200 rounded-lg p-3">
              {discoveryError}
            </div>
          {/if}
        </div>
        
        <!-- Discovery Results -->
        <SmartDiscoveryVisualizer
          {discoveryResult}
          isLoading={discoveryLoading}
          error={discoveryError}
          on:execute-tool={handleExecuteTool}
          on:show-tool-details={handleShowToolDetails}
        />
      </div>
    {/if}

    <!-- MagicTunnel System Management -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-xl font-semibold text-gray-700">System Management</h3>
        <div class="text-sm text-gray-500">
          Control MagicTunnel service lifecycle
        </div>
      </div>

      <!-- Restart Status Display -->
      {#if restartingMagicTunnel}
        <div class="mb-4 p-4 bg-blue-50 border border-blue-200 rounded-lg">
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
        <div class="mb-4 p-4 border rounded-lg {restartResult.status === 'success' ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'}">
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
        <div class="mb-4 p-4 border rounded-lg {
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
          
          {#if healthCheckResult.details}
            <details class="text-xs">
              <summary class="cursor-pointer {
                healthCheckResult.status === 'success' ? 'text-green-500 hover:text-green-700' :
                healthCheckResult.status === 'warning' ? 'text-yellow-500 hover:text-yellow-700' :
                'text-red-500 hover:text-red-700'
              }">View Details</summary>
              <div class="mt-2 p-2 bg-white rounded border">
                <div class="grid grid-cols-1 md:grid-cols-3 gap-2">
                  <div>
                    <div class="font-semibold">System Status</div>
                    <div class="text-gray-600">
                      Status: {healthCheckResult.details.system_status.status}<br>
                      Version: {healthCheckResult.details.system_status.version}<br>
                      Uptime: {healthCheckResult.details.system_status.uptime}
                    </div>
                  </div>
                  <div>
                    <div class="font-semibold">Tools Service</div>
                    <div class="text-gray-600">
                      Status: {healthCheckResult.details.tools_service.status}<br>
                      Tools: {healthCheckResult.details.tools_service.tools_count}
                    </div>
                  </div>
                  <div>
                    <div class="font-semibold">External MCP</div>
                    <div class="text-gray-600">
                      Configured: {healthCheckResult.details.external_mcp.servers_configured}<br>
                      Active: {healthCheckResult.details.external_mcp.servers_active}
                    </div>
                  </div>
                </div>
                {#if healthCheckResult.response_time}
                  <div class="mt-2 text-gray-500">
                    Response time: {healthCheckResult.response_time}ms
                  </div>
                {/if}
              </div>
            </details>
          {/if}
        </div>
      {/if}

      <!-- System Management Actions -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
        <button 
          class="btn-primary" 
          on:click={restartMagicTunnel}
          disabled={restartingMagicTunnel}
        >
          🔄 {restartingMagicTunnel ? 'Restarting...' : 'Restart MagicTunnel'}
        </button>

        <button 
          class="btn-secondary" 
          on:click={performHealthCheck}
          disabled={performingHealthCheck}
        >
          🏥 {performingHealthCheck ? 'Checking...' : 'Health Check'}
        </button>

        <button 
          class="btn-secondary" 
          on:click={handleManualRefresh}
          disabled={loading}
        >
          🔄 {loading ? 'Refreshing...' : 'Refresh Data'}
        </button>

      </div>

      <div class="mt-3 text-xs text-gray-500">
        <strong>Restart MagicTunnel:</strong> Safely restart the main service with supervisor.<br>
        <strong>Health Check:</strong> Perform comprehensive system health analysis with detailed diagnostics.<br>
        <strong>Refresh Data:</strong> Reload dashboard data and check system health.
      </div>
    </div>

    <!-- Build & Management Commands -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-6">
        <h3 class="text-xl font-semibold text-gray-700">Build & Management Commands</h3>
        <div class="text-sm text-gray-500">
          {makefileCommands?.total || 0} commands available
        </div>
      </div>

      {#if makefileLoading}
        <div class="text-center py-8">
          <div class="text-gray-500">🔄 Loading commands...</div>
        </div>
      {:else if !makefileCommands?.commands?.length}
        <div class="text-center py-8">
          <div class="text-gray-500 mb-2">📋 No Makefile commands found</div>
          <div class="text-sm text-gray-400">
            Makefile commands enable build, test, and management operations
          </div>
        </div>
      {:else}
        <!-- Command Execution Result -->
        {#if lastCommandResult}
          <div class="mb-6 p-4 border border-gray-200 rounded-lg bg-gray-50">
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-sm font-medium text-gray-700">Last Command Result</h4>
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {getCommandStatusColor(lastCommandResult.status)}">
                {lastCommandResult.status}
              </span>
            </div>
            <div class="text-lg font-mono font-semibold text-gray-900 mb-2">
              {lastCommandResult.command}
            </div>
            <div class="text-sm text-gray-600 mb-2">{lastCommandResult.message}</div>
            {#if lastCommandResult.output}
              <details class="text-xs">
                <summary class="cursor-pointer text-gray-500 hover:text-gray-700">View Output</summary>
                <pre class="mt-2 p-2 bg-white rounded border text-gray-800 whitespace-pre-wrap max-h-40 overflow-y-auto">{lastCommandResult.output}</pre>
              </details>
            {/if}
            {#if lastCommandResult.exit_code !== undefined}
              <div class="text-xs text-gray-500 mt-2">
                Exit code: {lastCommandResult.exit_code}
              </div>
            {/if}
          </div>
        {/if}

        <!-- Commands by Category -->
        {#if makefileCommands.categories}
          <div class="space-y-6">
            {#each Object.entries(makefileCommands.categories) as [category, commands]}
              <div>
                <h4 class="text-lg font-medium text-gray-700 mb-3 flex items-center">
                  <span class="mr-2">{getCategoryIcon(category)}</span>
                  {category.charAt(0).toUpperCase() + category.slice(1)} Commands
                  <span class="ml-2 text-sm text-gray-500">({commands.length})</span>
                </h4>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {#each commands as command}
                    {#if makefileCommands.commands}
                      {@const fullCommand = makefileCommands.commands.find(c => c.name === command)}
                      {#if fullCommand}
                        <div class="border border-gray-200 rounded-lg p-4 hover:bg-gray-50 transition-colors">
                          <!-- Prominent Command Name -->
                          <div class="text-xl font-bold text-gray-900 mb-2">
                            {fullCommand.name}
                          </div>
                          
                          <!-- Command Details -->
                          <div class="text-sm font-mono text-gray-600 mb-3 bg-gray-100 px-2 py-1 rounded">
                            make {fullCommand.name}
                          </div>
                          
                          <!-- Description -->
                          <p class="text-sm text-gray-600 mb-3 min-h-[2.5rem]">
                            {fullCommand.description}
                          </p>
                          
                          <!-- Show Commands Section -->
                          {#if fullCommand.script}
                            <div class="mb-3">
                              <button
                                class="text-xs text-blue-600 hover:text-blue-800 underline"
                                on:click={() => toggleCommandScript(fullCommand.name)}
                              >
                                {expandedCommands.has(fullCommand.name) ? '🔼 Hide Commands' : '🔽 Show Commands'}
                              </button>
                              
                              {#if expandedCommands.has(fullCommand.name)}
                                <div class="mt-2 p-3 bg-gray-50 rounded-md border-l-4 border-blue-200">
                                  <div class="text-xs text-gray-500 mb-1">Command Script:</div>
                                  <pre class="text-xs font-mono text-gray-800 whitespace-pre-wrap overflow-x-auto">{fullCommand.script}</pre>
                                </div>
                              {/if}
                            </div>
                          {/if}
                          
                          <!-- Status indicators and action -->
                          <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                              {#if !fullCommand.safe_for_production}
                                <span class="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-800" title="Requires confirmation">
                                  ⚠️
                                </span>
                              {/if}
                              {#if fullCommand.requires_env?.length}
                                <span class="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800" title="Requires: {fullCommand.requires_env.join(', ')}">
                                  🔧
                                </span>
                              {/if}
                            </div>
                            
                            <button
                              class="btn-sm btn-primary"
                              disabled={executingCommand === fullCommand.name}
                              on:click={() => executeMakefileCommand(fullCommand.name)}
                            >
                              {executingCommand === fullCommand.name ? '⏳' : '▶️'} Execute
                            </button>
                          </div>
                        </div>
                      {/if}
                    {/if}
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Development Status -->
    <div class="card">
      <h3 class="text-xl font-semibold text-gray-700 mb-4">Development Status</h3>
      <div class="space-y-2">
        <div class="flex items-center text-gray-600">
          <span class="text-green-500 mr-2">✅</span>
          Frontend development setup with hot reload
        </div>
        <div class="flex items-center text-gray-600">
          <span class="text-green-500 mr-2">✅</span>
          API proxy configuration to Rust backend
        </div>
        <div class="flex items-center text-gray-600">
          <span class="text-green-500 mr-2">✅</span>
          TypeScript API client with type safety
        </div>
        <div class="flex items-center text-gray-600">
          <span class="{systemStatus ? 'text-green-500' : 'text-yellow-500'} mr-2">{systemStatus ? '✅' : '🔄'}</span>
          Real-time data loading: {systemStatus ? 'Connected' : 'Connecting...'}
        </div>
        <div class="flex items-center text-gray-600">
          <span class="text-green-500 mr-2">✅</span>
          Basic dashboard components and real-time updates
        </div>
      </div>
      
      {#if systemStatus}
        <div class="mt-4 p-3 bg-green-50 rounded-md">
          <p class="text-sm text-green-800">
            🎉 <strong>Success!</strong> Frontend is successfully connected to the Rust backend!<br>
            Backend version: {systemStatus.version} | Memory: {systemStatus.memory_usage}
          </p>
        </div>
      {/if}
    </div>

    <!-- MCP Testing Section -->
    {#if showMcpTester}
      <div class="card mb-8">
        <div class="flex items-center justify-between mb-6">
          <h3 class="text-xl font-semibold text-gray-700">MCP Command Tester</h3>
          <button class="btn-sm btn-secondary" on:click={toggleMcpTester}>
            ✕ Close
          </button>
        </div>
        
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              MCP Method <span class="text-red-500">*</span>
            </label>
            <input
              type="text"
              bind:value={mcpMethod}
              placeholder="e.g., tools/list, tools/call, initialize"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              disabled={executingMcpCommand}
            />
            <p class="text-xs text-gray-500 mt-1">
              Common methods: tools/list, tools/call, capabilities, resources/list
            </p>
          </div>

          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Parameters (JSON, optional)
            </label>
            <textarea
              bind:value={mcpParams}
              placeholder="{`{\"name\": \"smart_tool_discovery\", \"arguments\": {\"request\": \"ping google.com\"}}`}"
              rows="4"
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
              disabled={executingMcpCommand}
            ></textarea>
            <p class="text-xs text-gray-500 mt-1">
              Leave empty for methods without parameters. Must be valid JSON if provided.
            </p>
          </div>

          <div class="flex gap-3">
            <button
              class="btn-primary"
              on:click={executeMcpCommand}
              disabled={!mcpMethod.trim() || executingMcpCommand}
            >
              {executingMcpCommand ? '⏳ Executing...' : '🚀 Execute MCP Command'}
            </button>
            
            <button
              class="btn-secondary"
              on:click={() => { mcpMethod = ''; mcpParams = ''; mcpResult = null; }}
              disabled={executingMcpCommand}
            >
              🗑️ Clear
            </button>
          </div>

          <!-- Results Section -->
          {#if mcpResult}
            <div class="mt-6 border-t pt-4">
              <h4 class="text-lg font-medium text-gray-700 mb-3">
                MCP Response
                <span class="text-sm font-normal text-gray-500">({mcpResult.timestamp})</span>
              </h4>
              
              <div class="space-y-3">
                <!-- Status Badge -->
                <div class="flex items-center gap-2">
                  <span class="text-sm font-medium text-gray-600">Status:</span>
                  <span class="inline-flex items-center px-2 py-1 rounded text-xs font-medium {mcpResult.status === 'success' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                    {mcpResult.status === 'success' ? '✅' : '❌'} {mcpResult.status.toUpperCase()}
                  </span>
                </div>

                <!-- Method and Action -->
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
                  <div>
                    <span class="font-medium text-gray-600">Method:</span>
                    <code class="ml-2 px-2 py-1 bg-gray-100 rounded">{mcpResult.method}</code>
                  </div>
                  <div>
                    <span class="font-medium text-gray-600">Action:</span>
                    <code class="ml-2 px-2 py-1 bg-gray-100 rounded">{mcpResult.action}</code>
                  </div>
                </div>

                <!-- Error Message (if any) -->
                {#if mcpResult.status === 'error' && mcpResult.message}
                  <div class="p-3 bg-red-50 rounded-md border-l-4 border-red-200">
                    <div class="text-sm font-medium text-red-800 mb-1">Error Message</div>
                    <div class="text-sm text-red-700">{mcpResult.message}</div>
                  </div>
                {/if}

                <!-- Response Data -->
                {#if mcpResult.response}
                  <div>
                    <div class="text-sm font-medium text-gray-600 mb-2">Response Data:</div>
                    <div class="bg-gray-50 rounded-md p-3 border">
                      <pre class="text-xs font-mono text-gray-800 whitespace-pre-wrap overflow-x-auto">{JSON.stringify(mcpResult.response, null, 2)}</pre>
                    </div>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Refresh Button -->
    <div class="mt-6 text-center">
      <button 
        class="btn-secondary" 
        on:click={loadDashboardData}
        disabled={loading}
      >
        {loading ? '🔄 Loading...' : '🔄 Refresh Data'}
      </button>
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
<ToolExecutionModal 
  bind:isOpen={isModalOpen} 
  tool={smartDiscoveryTool}
  on:close={closeModal}
  on:execute={handleToolExecution}
/>

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