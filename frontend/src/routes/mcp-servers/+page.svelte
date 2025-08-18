<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type McpServersResponse, type McpServerCapabilities, type Capability, type CapabilitiesResponse, type ServiceStatus, type ServiceInfo, type ServiceDetailedMetrics } from '$lib/api';
  import { getInternalServers } from '$lib/utils/mcpServers';
  import McpServerDetailsModal from './components/McpServerDetailsModal.svelte';
  import AllowlistControlPanel from '$lib/components/security/AllowlistControlPanel.svelte';
  import type { AllowlistRule } from '$lib/types/security';

  let serversResponse: McpServersResponse | null = null;
  let capabilitiesResponse: CapabilitiesResponse | null = null;
  let serviceStatus: ServiceStatus | null = null;
  let loading = true;
  let error = '';
  let selectedServer: string | null = null;
  let refreshInterval: number;
  let nextRefreshIn = 30;
  
  // Allowlist state
  let serverAllowlistRules: Map<string, AllowlistRule | null> = new Map();
  let allowlistLoading = false;
  
  // Expand/collapse state for server details
  let expandedServers = new Set<string>();

  // Auto refresh every 30 seconds
  const REFRESH_INTERVAL = 30000;

  function getStatusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'healthy': return 'text-green-600 bg-green-100';
      case 'degraded': return 'text-orange-600 bg-orange-100';
      case 'unhealthy': return 'text-red-600 bg-red-100';
      case 'down': return 'text-gray-600 bg-gray-100';
      case 'unknown': return 'text-yellow-600 bg-yellow-100';
      case 'starting': return 'text-blue-600 bg-blue-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  }

  function getStatusIcon(status: string): string {
    switch (status.toLowerCase()) {
      case 'healthy': return '✅';
      case 'degraded': return '⚠️';
      case 'unhealthy': return '❌';
      case 'down': return '🔴';
      case 'unknown': return '❓';
      case 'starting': return '🔄';
      default: return '⚫';
    }
  }

  function formatTimestamp(timestamp: string): string {
    try {
      return new Date(timestamp).toLocaleString();
    } catch {
      return timestamp;
    }
  }

  function handleViewTools(serviceName: string) {
    // Navigate to tools page with filter for this service
    window.location.href = `/tools?service=${encodeURIComponent(serviceName)}`;
  }

  function handleViewLogs(serviceName: string) {
    // For now, show alert since logs viewer is not implemented
    alert(`Logs viewer for service "${serviceName}" is not yet implemented.\n\nThis feature is planned for Phase 5.5 with options for:\n- Native log viewer\n- Grafana/Loki integration\n- Simple file tail viewer`);
  }

  async function handleRestartService(serviceName: string) {
    if (!confirm(`Are you sure you want to restart the "${serviceName}" service?\n\nThis will temporarily stop the service and restart it.`)) {
      return;
    }

    try {
      const response = await api.restartService(serviceName);
      
      if (response.status === 'not_implemented') {
        alert(`Service Restart - Development Status\n\n` +
              `Service: ${serviceName}\n` +
              `Status: ${response.message}\n\n` +
              `Implementation Requirements:\n` +
              response.implementation_notes.map(note => `• ${note}`).join('\n') + 
              `\n\nThe MagicTunnel codebase already includes:\n` +
              `• ExternalMcpManager.restart_server() method\n` +
              `• Process lifecycle management infrastructure\n` +
              `• Restart attempt tracking and health monitoring`);
      } else {
        alert(`Service "${serviceName}" restart initiated successfully!`);
        // Refresh data
        loadServers();
      }
    } catch (error) {
      alert(`Failed to restart service "${serviceName}": ${error}`);
    }
  }

  async function handleStopService(serviceName: string) {
    if (!confirm(`Are you sure you want to stop the "${serviceName}" service?\n\nThis will stop the service and make its tools unavailable.`)) {
      return;
    }

    try {
      const response = await api.stopService(serviceName);
      
      if (response.status === 'not_implemented') {
        alert(`Service Stop - Development Status\n\n` +
              `Service: ${serviceName}\n` +
              `Status: ${response.message}\n\n` +
              `Implementation Requirements:\n` +
              response.implementation_notes.map(note => `• ${note}`).join('\n') + 
              `\n\nThe MagicTunnel codebase already includes:\n` +
              `• ExternalMcpProcess.stop() method\n` +
              `• Process termination and cleanup logic\n` +
              `• Health status tracking`);
      } else {
        alert(`Service "${serviceName}" stopped successfully!`);
        // Refresh data
        loadServers();
      }
    } catch (error) {
      alert(`Failed to stop service "${serviceName}": ${error}`);
    }
  }

  async function handleViewDetailedMetrics(serviceName: string) {
    try {
      const detailedMetrics = await api.getServiceMetrics(serviceName);
      
      // Create detailed metrics modal content
      const metricsInfo = `
        📊 Detailed Metrics for "${serviceName}"
        
        Current Status: ${detailedMetrics.current_metrics.status}
        Average Response Time: ${detailedMetrics.current_metrics.avg_response_time_ms.toFixed(2)}ms
        Success Rate: ${(detailedMetrics.current_metrics.success_rate * 100).toFixed(2)}%
        Error Rate: ${(detailedMetrics.current_metrics.error_rate * 100).toFixed(2)}%
        Total Requests: ${detailedMetrics.current_metrics.total_requests}
        Total Errors: ${detailedMetrics.current_metrics.total_errors}
        Uptime: ${detailedMetrics.current_metrics.uptime_percentage.toFixed(2)}%
        
        Last Successful Request: ${detailedMetrics.current_metrics.last_successful_request || 'Never'}
        Service Start Time: ${detailedMetrics.current_metrics.service_start_time || 'Unknown'}
      `;
      
      alert(metricsInfo);
    } catch (error) {
      alert(`Failed to load detailed metrics for "${serviceName}": ${error}`);
    }
  }

  async function loadServers() {
    try {
      loading = true;
      error = '';
      
      const [servers, capabilities, services] = await Promise.all([
        api.getMcpServers(),
        api.getCapabilities(),
        api.getServices()
      ]);
      
      serversResponse = servers;
      capabilitiesResponse = capabilities;
      serviceStatus = services;
      
      console.log('MCP Servers loaded:', servers);
      console.log('Capabilities loaded:', capabilities);
      console.log('Internal servers:', getInternalServers(capabilities));
      
      // Load allowlist rules for all servers
      await loadAllowlistRules();
    } catch (err) {
      console.error('Failed to load MCP servers:', err);
      error = err instanceof Error ? err.message : 'Failed to load MCP servers';
    } finally {
      loading = false;
      nextRefreshIn = 30; // Reset countdown
    }
  }

  function openServerDetails(serverName: string) {
    selectedServer = serverName;
  }

  function closeServerDetails() {
    selectedServer = null;
  }

  function getServerStatusColor(server: McpServerCapabilities): string {
    return server.is_running ? 'text-green-600 bg-green-100' : 'text-red-600 bg-red-100';
  }

  function getServerTypeIcon(server: McpServerCapabilities): string {
    // Determine server type based on name
    if (server.name === 'magictunnel-internal') {
      return '🏠'; // Internal server
    }
    return '🔌'; // External MCP server
  }

  function formatUptime(uptimeSeconds: number | null): string {
    if (!uptimeSeconds) return 'Not running';
    
    const hours = Math.floor(uptimeSeconds / 3600);
    const minutes = Math.floor((uptimeSeconds % 3600) / 60);
    const seconds = uptimeSeconds % 60;
    
    if (hours > 0) {
      return `${hours}h ${minutes}m ${seconds}s`;
    } else if (minutes > 0) {
      return `${minutes}m ${seconds}s`;
    } else {
      return `${seconds}s`;
    }
  }


  function toggleServerExpanded(serverName: string) {
    const newExpanded = new Set(expandedServers);
    if (newExpanded.has(serverName)) {
      newExpanded.delete(serverName);
    } else {
      newExpanded.add(serverName);
    }
    expandedServers = newExpanded;
  }

  async function loadAllowlistRules() {
    allowlistLoading = true;
    try {
      const allServers = [
        ...(serversResponse?.servers || []).map(s => s.name),
        ...getInternalServers(capabilitiesResponse).map(s => s.name)
      ];
      
      const rulePromises = allServers.map(async (serverName) => {
        try {
          const rule = await api.getServerAllowlistRule(serverName);
          return { serverName, rule };
        } catch (err) {
          console.warn(`Failed to load allowlist rule for server ${serverName}:`, err);
          return { serverName, rule: null };
        }
      });
      
      const results = await Promise.all(rulePromises);
      const newRulesMap = new Map();
      results.forEach(({ serverName, rule }) => {
        newRulesMap.set(serverName, rule);
      });
      serverAllowlistRules = newRulesMap;
    } catch (err) {
      console.error('Failed to load allowlist rules:', err);
    } finally {
      allowlistLoading = false;
    }
  }

  // Allowlist event handlers
  async function handleAllowlistChange(event: CustomEvent) {
    const { itemType, itemName, action, currentRule } = event.detail;
    
    try {
      if (action === 'remove') {
        // Remove the specific rule to use default policy
        if (currentRule) {
          await api.removeServerAllowlistRule(itemName);
          serverAllowlistRules.set(itemName, null);
        }
      } else {
        // Create or update the rule
        const ruleData = {
          type: itemType as 'tool' | 'server' | 'global',
          name: itemName,
          action: action as 'allow' | 'deny',
          enabled: true,
          reason: `${action === 'allow' ? 'Allow' : 'Deny'} access to ${itemName} MCP server`
        };
        
        const updatedRule = await api.setServerAllowlistRule(itemName, action as 'allow' | 'deny', `${action === 'allow' ? 'Allow' : 'Deny'} access to ${itemName} MCP server`);
        serverAllowlistRules.set(itemName, updatedRule);
      }
      
      // Trigger reactivity
      serverAllowlistRules = serverAllowlistRules;
    } catch (err) {
      console.error('Failed to update server allowlist rule:', err);
      error = `Failed to update allowlist: ${err}`;
    }
  }

  async function handleAllowlistToggle(event: CustomEvent) {
    const { itemType, itemName, enabled, currentRule } = event.detail;
    
    if (!currentRule) return;
    
    try {
      const ruleData = {
        type: currentRule.type,
        name: currentRule.name,
        action: currentRule.action,
        enabled: enabled,
        reason: currentRule.reason
      };
      
      const updatedRule = await api.setServerAllowlistRule(itemName, ruleData);
      serverAllowlistRules.set(itemName, updatedRule);
      
      // Trigger reactivity
      serverAllowlistRules = serverAllowlistRules;
    } catch (err) {
      console.error('Failed to toggle server allowlist rule:', err);
      error = `Failed to toggle allowlist: ${err}`;
    }
  }

  async function handleAllowlistEdit(event: CustomEvent) {
    const { itemType, itemName, currentRule } = event.detail;
    
    // For now, we'll just log this - in the future we could open a detailed editor
    console.log('Edit allowlist rule for server:', itemName, currentRule);
    
    // TODO: Open AllowlistRuleEditor modal
    error = 'Rule editing UI will be implemented in a future update';
  }

  onMount(() => {
    loadServers();
    
    // Set up auto-refresh
    refreshInterval = setInterval(loadServers, REFRESH_INTERVAL);
    
    // Set up countdown timer
    const countdownInterval = setInterval(() => {
      if (nextRefreshIn > 0) {
        nextRefreshIn -= 1;
      } else {
        nextRefreshIn = 30; // Reset when refresh happens
      }
    }, 1000);
    
    // Cleanup intervals on component destroy
    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
      }
      clearInterval(countdownInterval);
    };
  });
</script>

<div class="container mx-auto px-6 py-8">
  <div class="flex justify-between items-center mb-8">
    <div>
      <h1 class="text-3xl font-bold text-gray-900">MCP Servers</h1>
      <p class="text-gray-600 mt-2">Manage Model Context Protocol servers and their capabilities</p>
    </div>
    <button
      on:click={loadServers}
      disabled={loading}
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
    >
      <span class="text-sm">🔄</span>
      {loading ? 'Refreshing...' : 'Refresh'}
    </button>
  </div>

  {#if error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6">
      <strong class="font-bold">Error:</strong>
      <span class="block sm:inline">{error}</span>
    </div>
  {/if}

  {#if loading && !serversResponse}
    <div class="flex justify-center items-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      <span class="ml-3 text-gray-600">Loading MCP servers...</span>
    </div>
  {:else if serversResponse}
    <!-- Enhanced Statistics Overview -->
    <div class="grid grid-cols-1 md:grid-cols-6 gap-6 mb-8">
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-600">Total Servers</p>
            <p class="text-2xl font-bold text-gray-900">{(serversResponse.servers?.length || 0) + (capabilitiesResponse ? getInternalServers(capabilitiesResponse).length : 0)}</p>
          </div>
          <div class="p-3 bg-blue-100 rounded-full">
            <span class="text-xl">🖥️</span>
          </div>
        </div>
      </div>
      
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-600">Internal Servers</p>
            <p class="text-2xl font-bold text-gray-900">{capabilitiesResponse ? getInternalServers(capabilitiesResponse).length : serversResponse?.internal_servers || 0}</p>
          </div>
          <div class="p-3 bg-green-100 rounded-full">
            <span class="text-xl">🏠</span>
          </div>
        </div>
      </div>
      
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-600">External Servers</p>
            <p class="text-2xl font-bold text-gray-900">{serversResponse.external_servers}</p>
          </div>
          <div class="p-3 bg-purple-100 rounded-full">
            <span class="text-xl">🔌</span>
          </div>
        </div>
      </div>
      
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-600">Running Servers</p>
            <p class="text-2xl font-bold text-gray-900">
              {serversResponse.servers.filter(s => s.is_running).length}
            </p>
          </div>
          <div class="p-3 bg-green-100 rounded-full">
            <span class="text-xl">✅</span>
          </div>
        </div>
      </div>
      
      <!-- Health Status Cards -->
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-600">Healthy</p>
            <p class="text-2xl font-bold text-green-900">{serviceStatus?.healthy || 0}</p>
          </div>
          <div class="p-3 bg-green-100 rounded-full">
            <span class="text-xl">✅</span>
          </div>
        </div>
      </div>
      
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-600">Degraded/Down</p>
            <p class="text-2xl font-bold text-red-900">{(serviceStatus?.degraded || 0) + (serviceStatus?.unhealthy || 0) + (serviceStatus?.down || 0)}</p>
          </div>
          <div class="p-3 bg-red-100 rounded-full">
            <span class="text-xl">⚠️</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Last Updated Info -->
    {#if serviceStatus?.last_updated}
      <div class="mb-6 bg-blue-50 border border-blue-200 rounded-lg p-4">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <span class="text-blue-600">📊</span>
            <div>
              <p class="text-sm font-medium text-blue-900">System Status</p>
              <p class="text-xs text-blue-700">Last updated: {formatTimestamp(serviceStatus.last_updated)}</p>
            </div>
          </div>
          <div class="text-right">
            <p class="text-xs text-blue-600">Auto-refresh in {nextRefreshIn}s</p>
          </div>
        </div>
      </div>
    {/if}

    <!-- Internal File-Based Servers -->
    <div class="mb-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">🏠 Internal File-Based Servers</h2>
      
      {#if loading}
        <div class="text-center py-8">
          <div class="text-gray-500">🔄 Loading internal servers...</div>
        </div>
      {:else if !capabilitiesResponse || getInternalServers(capabilitiesResponse).length === 0}
        <div class="text-center py-8">
          <div class="text-gray-500 mb-2">📭 No internal servers found</div>
          <div class="text-sm text-gray-400">
            Internal servers are created from YAML files in the <code>capabilities/</code> directory
          </div>
        </div>
      {:else}
        <div class="space-y-4">
          {#each getInternalServers(capabilitiesResponse) as internalServer}
            <div class="bg-white p-6 rounded-lg shadow border">
              <!-- Internal Server Header -->
              <div class="flex items-center justify-between mb-4">
                <div class="flex items-center space-x-3">
                  <span class="text-2xl">🏠</span>
                  <div>
                    <h3 class="text-xl font-semibold text-gray-900">{internalServer.name}</h3>
                    <p class="text-gray-600">Internal file-based server</p>
                  </div>
                </div>
                
                <div class="flex items-center space-x-4">
                  <span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-green-100 text-green-800">
                    Always Running
                  </span>
                  
                  <div class="text-right">
                    <p class="text-sm text-gray-500">Tools</p>
                    <p class="text-lg font-semibold text-gray-900">{internalServer.capabilities.length}</p>
                  </div>
                  
                  <button 
                    class="p-2 text-gray-400 hover:text-gray-600 transition-colors"
                    on:click={() => toggleServerExpanded(`internal-${internalServer.name}`)}
                    title="Toggle details"
                  >
                    {expandedServers.has(`internal-${internalServer.name}`) ? '🔽' : '▶️'}
                  </button>
                </div>
              </div>
              
              <!-- Quick server info -->
              <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
                <div>
                  <p class="text-sm text-gray-500">Type</p>
                  <p class="font-medium">Internal File-Based</p>
                </div>
                <div>
                  <p class="text-sm text-gray-500">Source</p>
                  <p class="font-medium font-mono text-sm">{internalServer.source}</p>
                </div>
                <div>
                  <p class="text-sm text-gray-500">Status</p>
                  <p class="font-medium text-green-600">✅ Always Available</p>
                </div>
              </div>

              <!-- Allowlist Control (Compact View) -->
              {#if allowlistLoading}
                <div class="border-t pt-4">
                  <div class="flex items-center gap-2 text-sm text-gray-500">
                    <div class="animate-spin rounded-full h-3 w-3 border-b-2 border-gray-400"></div>
                    Loading allowlist rules...
                  </div>
                </div>
              {:else}
                <div class="border-t pt-4">
                  <AllowlistControlPanel
                    itemType="server"
                    itemName={internalServer.name}
                    currentRule={serverAllowlistRules.get(internalServer.name) || null}
                    compact={true}
                    on:allowlist-change={handleAllowlistChange}
                    on:allowlist-toggle={handleAllowlistToggle}
                    on:allowlist-edit={handleAllowlistEdit}
                  />
                </div>
              {/if}

              <!-- Expanded Details -->
              {#if expandedServers.has(`internal-${internalServer.name}`)}
                <div class="mt-6 pt-6 border-t border-gray-200">
                  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                    <div>
                      <h4 class="font-semibold text-gray-900 mb-3">Available Tools</h4>
                      <div class="space-y-2 max-h-64 overflow-y-auto">
                        {#each internalServer.capabilities as capability}
                          <div class="flex items-center gap-3 p-2 bg-gray-50 rounded">
                            <span class="w-2 h-2 bg-green-500 rounded-full"></span>
                            <div class="flex-1">
                              <p class="font-medium text-sm">{capability.name}</p>
                              <p class="text-xs text-gray-600">{capability.description}</p>
                            </div>
                            <span class="text-xs bg-gray-200 text-gray-700 px-2 py-1 rounded">
                              {capability.category}
                            </span>
                          </div>
                        {/each}
                      </div>
                    </div>
                    
                    <div>
                      <h4 class="font-semibold text-gray-900 mb-3">🛡️ Allowlist Control</h4>
                      {#if allowlistLoading}
                        <div class="bg-gray-50 border border-gray-200 rounded-lg p-4">
                          <div class="flex items-center gap-3">
                            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-400"></div>
                            <span class="text-sm text-gray-600">Loading allowlist rules...</span>
                          </div>
                        </div>
                      {:else}
                        <AllowlistControlPanel
                          itemType="server"
                          itemName={internalServer.name}
                          currentRule={serverAllowlistRules.get(internalServer.name) || null}
                          compact={false}
                          on:allowlist-change={handleAllowlistChange}
                          on:allowlist-toggle={handleAllowlistToggle}
                          on:allowlist-edit={handleAllowlistEdit}
                        />
                      {/if}
                    </div>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- External MCP Servers -->
    <div class="mb-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">🔌 External MCP Servers</h2>
      
      <div class="space-y-4">
        {#each serversResponse.servers as server (server.name)}
          {@const serviceInfo = serviceStatus?.services?.find(s => s.name === server.name)}
          <div class="bg-white p-6 rounded-lg shadow border">
            <!-- Enhanced Server Header with Health Status -->
            <div class="flex items-center justify-between mb-4">
              <div class="flex items-center space-x-3">
                <span class="text-2xl">🔌</span>
                <div>
                  <h3 class="text-xl font-semibold text-gray-900">{server.name}</h3>
                  <p class="text-gray-600">External MCP Server</p>
                </div>
              </div>
              
              <div class="flex items-center space-x-4">
                <!-- Health Status -->
                {#if serviceInfo}
                  <span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {getStatusColor(serviceInfo.status)}">
                    {getStatusIcon(serviceInfo.status)} {serviceInfo.status}
                  </span>
                {:else}
                  <span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {getStatusColor(server.is_running ? 'healthy' : 'down')}">
                    {getStatusIcon(server.is_running ? 'healthy' : 'down')} {server.is_running ? 'Running' : 'Stopped'}
                  </span>
                {/if}
                
                <div class="text-right">
                  <p class="text-sm text-gray-500">Tools</p>
                  <p class="text-lg font-semibold text-gray-900">{server.capabilities?.length || 0}</p>
                </div>
                
                <button 
                  class="p-2 text-gray-400 hover:text-gray-600 transition-colors"
                  on:click={() => toggleServerExpanded(`external-${server.name}`)}
                  title="Toggle details"
                >
                  {expandedServers.has(`external-${server.name}`) ? '🔽' : '▶️'}
                </button>
              </div>
            </div>
            
            <!-- Quick server info -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
              <div>
                <p class="text-sm text-gray-500">Type</p>
                <p class="font-medium">External MCP Server</p>
              </div>
              <div>
                <p class="text-sm text-gray-500">Status</p>
                <p class="font-medium {server.is_running ? 'text-green-600' : 'text-red-600'}">
                  {server.is_running ? '✅ Running' : '🔴 Stopped'}
                </p>
              </div>
              <div>
                <p class="text-sm text-gray-500">Uptime</p>
                <p class="font-medium">{formatUptime(server.uptime_seconds)}</p>
              </div>
              <div>
                <p class="text-sm text-gray-500">Tools</p>
                <p class="font-medium">{server.capabilities?.length || 0} available</p>
              </div>
            </div>

            <!-- Service Management Actions -->
            <div class="flex items-center gap-2 mb-4 p-3 bg-gray-50 rounded-lg">
              <button
                class="px-3 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
                on:click={() => handleViewTools(server.name)}
              >
                🔧 View Tools
              </button>
              <button
                class="px-3 py-2 bg-green-600 text-white rounded hover:bg-green-700 text-sm"
                on:click={() => handleRestartService(server.name)}
              >
                🔄 Restart
              </button>
              <button
                class="px-3 py-2 bg-red-600 text-white rounded hover:bg-red-700 text-sm"
                on:click={() => handleStopService(server.name)}
              >
                🛑 Stop
              </button>
              <button
                class="px-3 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 text-sm"
                on:click={() => handleViewLogs(server.name)}
              >
                📋 Logs
              </button>
              {#if serviceInfo}
                <button
                  class="px-3 py-2 bg-purple-600 text-white rounded hover:bg-purple-700 text-sm"
                  on:click={() => handleViewDetailedMetrics(server.name)}
                >
                  📊 Metrics
                </button>
              {/if}
            </div>

            <!-- Allowlist Control (Compact View) -->
            {#if allowlistLoading}
              <div class="border-t pt-4">
                <div class="flex items-center gap-2 text-sm text-gray-500">
                  <div class="animate-spin rounded-full h-3 w-3 border-b-2 border-gray-400"></div>
                  Loading allowlist rules...
                </div>
              </div>
            {:else}
              <div class="border-t pt-4">
                <AllowlistControlPanel
                  itemType="server"
                  itemName={server.name}
                  currentRule={serverAllowlistRules.get(server.name) || null}
                  compact={true}
                  on:allowlist-change={handleAllowlistChange}
                  on:allowlist-toggle={handleAllowlistToggle}
                  on:allowlist-edit={handleAllowlistEdit}
                />
              </div>
            {/if}

            <!-- Expanded Details -->
            {#if expandedServers.has(`external-${server.name}`)}
              <div class="mt-6 pt-6 border-t border-gray-200">
                <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                  <div>
                    <h4 class="font-semibold text-gray-900 mb-3">Server Information</h4>
                    <div class="space-y-2 text-sm">
                      {#if serviceInfo}
                        <div><span class="font-medium">Health:</span> {serviceInfo.status}</div>
                        <div><span class="font-medium">Response Time:</span> {serviceInfo.avg_response_time_ms}ms</div>
                        <div><span class="font-medium">Success Rate:</span> {(serviceInfo.success_rate * 100).toFixed(1)}%</div>
                        <div><span class="font-medium">Total Requests:</span> {serviceInfo.total_requests}</div>
                        <div><span class="font-medium">Errors:</span> {serviceInfo.total_errors}</div>
                      {:else}
                        <div><span class="font-medium">Status:</span> {server.is_running ? 'Running' : 'Stopped'}</div>
                        <div><span class="font-medium">Uptime:</span> {formatUptime(server.uptime_seconds)}</div>
                      {/if}
                    </div>
                  </div>
                  
                  <div>
                    <h4 class="font-semibold text-gray-900 mb-3">🛡️ Allowlist Control</h4>
                    {#if allowlistLoading}
                      <div class="bg-gray-50 border border-gray-200 rounded-lg p-4">
                        <div class="flex items-center gap-3">
                          <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-400"></div>
                          <span class="text-sm text-gray-600">Loading allowlist rules...</span>
                        </div>
                      </div>
                    {:else}
                      <AllowlistControlPanel
                        itemType="server"
                        itemName={server.name}
                        currentRule={serverAllowlistRules.get(server.name) || null}
                        compact={false}
                        on:allowlist-change={handleAllowlistChange}
                        on:allowlist-toggle={handleAllowlistToggle}
                        on:allowlist-edit={handleAllowlistEdit}
                      />
                    {/if}
                  </div>
                </div>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    {#if serversResponse.servers.length === 0}
      <div class="text-center py-12">
        <div class="mx-auto h-12 w-12 text-gray-400 mb-4">
          <span class="text-4xl">🔌</span>
        </div>
        <h3 class="mt-2 text-sm font-medium text-gray-900">No MCP servers</h3>
        <p class="mt-1 text-sm text-gray-500">
          No MCP servers are currently configured or running.
        </p>
      </div>
    {/if}
  {/if}
</div>

<!-- Server Details Modal -->
{#if selectedServer}
  <McpServerDetailsModal 
    serverName={selectedServer} 
    on:close={closeServerDetails}
  />
{/if}