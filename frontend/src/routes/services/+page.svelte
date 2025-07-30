<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type ServiceStatus, type ServiceInfo, type ServiceDetailedMetrics } from '$lib/api';

  let serviceStatus: ServiceStatus | null = null;
  let loading = true;
  let error = '';
  let nextRefreshIn = 30;

  // Expand/collapse state for service details
  let expandedServices = new Set<string>();

  function toggleServiceExpanded(serviceName: string) {
    if (expandedServices.has(serviceName)) {
      expandedServices.delete(serviceName);
    } else {
      expandedServices.add(serviceName);
    }
    expandedServices = expandedServices; // Trigger reactivity
  }

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

  async function loadServicesData() {
    loading = true;
    error = '';
    
    try {
      serviceStatus = await api.getServices();
    } catch (err) {
      error = `Failed to load services data: ${err}`;
      console.error('Services data loading error:', err);
    } finally {
      loading = false;
      nextRefreshIn = 30; // Reset countdown
    }
  }

  function handleManualRefresh() {
    loadServicesData();
  }

  function formatTimestamp(timestamp: string): string {
    try {
      return new Date(timestamp).toLocaleString();
    } catch {
      return timestamp;
    }
  }

  function formatArray(arr: any[]): string {
    if (!Array.isArray(arr) || arr.length === 0) return 'None';
    return arr.map(item => typeof item === 'string' ? item : JSON.stringify(item)).join(', ');
  }

  function formatEnvironment(env: { [key: string]: string }): string {
    if (!env || Object.keys(env).length === 0) return 'None';
    return Object.entries(env)
      .map(([key, value]) => `${key}=${value}`)
      .join(', ');
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
        // Refresh services data
        loadServicesData();
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
        // Refresh services data
        loadServicesData();
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
        Consecutive Failures: ${detailedMetrics.current_metrics.consecutive_failures}
        
        Last Successful Request: ${detailedMetrics.current_metrics.last_successful_request || 'Never'}
        Service Start Time: ${detailedMetrics.current_metrics.service_start_time || 'Unknown'}
        
        Request Types: ${Object.entries(detailedMetrics.request_distribution)
          .map(([type, count]) => `${type}: ${count}`).join(', ') || 'None'}
        
        Error Types: ${Object.entries(detailedMetrics.error_distribution)
          .map(([type, count]) => `${type}: ${count}`).join(', ') || 'None'}
        
        Recent Latencies: ${detailedMetrics.recent_latencies.slice(-10).map(l => `${l.toFixed(0)}ms`).join(', ')}
        
        Last Updated: ${formatTimestamp(detailedMetrics.last_updated)}
      `;
      
      alert(metricsInfo);
    } catch (error) {
      alert(`Failed to load detailed metrics for "${serviceName}": ${error}`);
    }
  }


  onMount(() => {
    loadServicesData();
    
    // Countdown timer - updates every second
    const countdownInterval = setInterval(() => {
      nextRefreshIn = Math.max(0, nextRefreshIn - 1);
    }, 1000);
    
    // Refresh data every 30 seconds
    const refreshInterval = setInterval(loadServicesData, 30000);
    
    return () => {
      clearInterval(countdownInterval);
      clearInterval(refreshInterval);
    };
  });
</script>

<svelte:head>
  <title>Services - MagicTunnel</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <div class="container mx-auto px-4 py-8">
    <!-- Header -->
    <header class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <div class="flex items-center gap-4 mb-2">
            <a href="/" class="btn-secondary text-sm">
              ← Back to Dashboard
            </a>
            <h1 class="text-4xl font-bold text-primary-700">External MCP Services</h1>
          </div>
          <p class="text-gray-600">Monitor and manage external MCP server connections</p>
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


    <!-- Services Overview Cards -->
    <div class="grid grid-cols-1 md:grid-cols-5 gap-6 mb-8">
      <div class="card">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">Total Services</h3>
        <div class="text-2xl font-bold text-primary-600">
          {serviceStatus?.total ?? '--'}
        </div>
        <p class="text-sm text-gray-500">
          External MCP servers
        </p>
      </div>

      <div class="card">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">Healthy</h3>
        <div class="text-2xl font-bold text-green-600">
          {serviceStatus?.healthy ?? '--'}
        </div>
        <p class="text-sm text-gray-500">
          Services responding
        </p>
      </div>

      <div class="card">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">Degraded</h3>
        <div class="text-2xl font-bold text-orange-600">
          {serviceStatus?.degraded ?? '--'}
        </div>
        <p class="text-sm text-gray-500">
          Services with issues
        </p>
      </div>

      <div class="card">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">Down</h3>
        <div class="text-2xl font-bold text-red-600">
          {(serviceStatus?.unhealthy ?? 0) + (serviceStatus?.down ?? 0)}
        </div>
        <p class="text-sm text-gray-500">
          Services not responding
        </p>
      </div>

      <div class="card">
        <h3 class="text-lg font-semibold text-gray-700 mb-2">Last Updated</h3>
        <div class="text-sm font-medium text-gray-600">
          {serviceStatus?.last_updated ? formatTimestamp(serviceStatus.last_updated) : '--'}
        </div>
        <p class="text-sm text-gray-500">
          Status check time
        </p>
      </div>
    </div>

    <!-- Services List -->
    <div class="card">
      <div class="flex items-center justify-between mb-6">
        <h3 class="text-xl font-semibold text-gray-700">Services List</h3>
        <div class="text-sm text-gray-500">
          {serviceStatus?.services?.length || 0} services configured
        </div>
      </div>

      {#if loading}
        <div class="text-center py-8">
          <div class="text-gray-500">🔄 Loading services...</div>
        </div>
      {:else if !serviceStatus?.services?.length}
        <div class="text-center py-8">
          <div class="text-gray-500 mb-2">📭 No external MCP services configured</div>
          <div class="text-sm text-gray-400">
            External MCP services can be configured in <code>external-mcp-servers.yaml</code>
          </div>
        </div>
      {:else}
        <div class="space-y-4">
          {#each serviceStatus.services as service}
            <div class="border border-gray-200 rounded-lg p-4 hover:bg-gray-50 transition-colors">
              <!-- Service Header -->
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                  <span class="text-lg">{getStatusIcon(service.status)}</span>
                  <div>
                    <h4 class="text-lg font-medium text-gray-900">{service.name}</h4>
                    <p class="text-sm text-gray-600">{service.command}</p>
                  </div>
                </div>
                
                <div class="flex items-center space-x-3">
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {getStatusColor(service.status)}">
                    {service.status}
                  </span>
                  
                  <div class="text-sm text-gray-500">
                    {service.tools_count} tools
                  </div>
                  
                  <button 
                    class="p-1 text-gray-400 hover:text-gray-600 transition-colors"
                    on:click={() => toggleServiceExpanded(service.name)}
                    title="Toggle details"
                  >
                    {expandedServices.has(service.name) ? '▼' : '▶'}
                  </button>
                </div>
              </div>

              <!-- Expanded Service Details -->
              {#if expandedServices.has(service.name)}
                <div class="mt-4 pt-4 border-t border-gray-200">
                  <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                      <h5 class="text-sm font-medium text-gray-700 mb-2">Configuration</h5>
                      <div class="space-y-2 text-sm">
                        <div>
                          <span class="font-medium text-gray-600">Type:</span>
                          <span class="ml-2 text-gray-800">{service.type}</span>
                        </div>
                        <div>
                          <span class="font-medium text-gray-600">Command:</span>
                          <span class="ml-2 font-mono text-gray-800">{service.command}</span>
                        </div>
                        <div>
                          <span class="font-medium text-gray-600">Arguments:</span>
                          <span class="ml-2 font-mono text-gray-800">{formatArray(service.args)}</span>
                        </div>
                        <div>
                          <span class="font-medium text-gray-600">Environment:</span>
                          <div class="ml-2 font-mono text-gray-800 text-xs">
                            {formatEnvironment(service.env)}
                          </div>
                        </div>
                      </div>
                    </div>
                    
                    <div>
                      <h5 class="text-sm font-medium text-gray-700 mb-2">Runtime Info</h5>
                      <div class="space-y-2 text-sm">
                        <div>
                          <span class="font-medium text-gray-600">Process ID:</span>
                          <span class="ml-2 text-gray-800">{service.pid}</span>
                        </div>
                        <div>
                          <span class="font-medium text-gray-600">Uptime:</span>
                          <span class="ml-2 text-gray-800">{service.uptime}</span>
                        </div>
                        <div>
                          <span class="font-medium text-gray-600">Last Seen:</span>
                          <span class="ml-2 text-gray-800">{formatTimestamp(service.last_seen)}</span>
                        </div>
                        <div>
                          <span class="font-medium text-gray-600">Tools Count:</span>
                          <span class="ml-2 text-gray-800">{service.tools_count}</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  <!-- Enhanced Metrics Section -->
                  {#if service.metrics.has_metrics}
                  <div class="mt-4 pt-4 border-t border-gray-200">
                    <h5 class="text-sm font-medium text-gray-700 mb-3">📊 Performance Metrics</h5>
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                      <div class="bg-gray-50 p-3 rounded-lg">
                        <div class="text-xs text-gray-500 mb-1">Avg Response Time</div>
                        <div class="text-sm font-semibold text-gray-800">
                          {service.metrics.response_time_ms ? `${service.metrics.response_time_ms.toFixed(0)}ms` : 'N/A'}
                        </div>
                      </div>
                      <div class="bg-gray-50 p-3 rounded-lg">
                        <div class="text-xs text-gray-500 mb-1">Success Rate</div>
                        <div class="text-sm font-semibold {service.metrics.success_rate ? (service.metrics.success_rate >= 0.95 ? 'text-green-600' : service.metrics.success_rate >= 0.90 ? 'text-orange-600' : 'text-red-600') : 'text-gray-800'}">
                          {service.metrics.success_rate ? `${(service.metrics.success_rate * 100).toFixed(1)}%` : 'N/A'}
                        </div>
                      </div>
                      <div class="bg-gray-50 p-3 rounded-lg">
                        <div class="text-xs text-gray-500 mb-1">Total Requests</div>
                        <div class="text-sm font-semibold text-gray-800">
                          {service.metrics.total_requests ?? 'N/A'}
                        </div>
                      </div>
                      <div class="bg-gray-50 p-3 rounded-lg">
                        <div class="text-xs text-gray-500 mb-1">Uptime</div>
                        <div class="text-sm font-semibold {service.metrics.uptime_percentage ? (service.metrics.uptime_percentage >= 99 ? 'text-green-600' : service.metrics.uptime_percentage >= 95 ? 'text-orange-600' : 'text-red-600') : 'text-gray-800'}">
                          {service.metrics.uptime_percentage ? `${service.metrics.uptime_percentage.toFixed(1)}%` : 'N/A'}
                        </div>
                      </div>
                    </div>
                  </div>
                  {:else}
                  <div class="mt-4 pt-4 border-t border-gray-200">
                    <div class="text-sm text-gray-500 italic">📊 No metrics data available yet</div>
                  </div>
                  {/if}

                  <!-- Service Actions -->
                  <div class="mt-4 pt-4 border-t border-gray-200">
                    <div class="flex space-x-3">
                      <button 
                        class="btn-secondary text-sm" 
                        on:click={() => handleRestartService(service.name)}
                      >
                        🔄 Restart Service
                      </button>
                      <button 
                        class="btn-secondary text-sm" 
                        on:click={() => handleStopService(service.name)}
                      >
                        ⏹️ Stop Service
                      </button>
                      <button 
                        class="btn-primary text-sm" 
                        on:click={() => handleViewTools(service.name)}
                      >
                        📊 View Tools ({service.tools_count})
                      </button>
                      <button 
                        class="btn-secondary text-sm" 
                        on:click={() => handleViewLogs(service.name)}
                      >
                        📝 View Logs
                      </button>
                      <button 
                        class="btn-primary text-sm" 
                        on:click={() => handleViewDetailedMetrics(service.name)}
                        disabled={!service.metrics.has_metrics}
                        title={service.metrics.has_metrics ? 'View detailed metrics' : 'No metrics available'}
                      >
                        📊 Detailed Metrics
                      </button>
                    </div>
                    <div class="text-xs text-gray-500 mt-2">
                      Tools viewer available • Process management planned for future release
                    </div>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>


    <!-- Configuration Help -->
    <div class="mt-8 card">
      <h3 class="text-xl font-semibold text-gray-700 mb-4">Configuration Help</h3>
      <div class="space-y-4 text-sm">
        <div>
          <h4 class="font-medium text-gray-700 mb-2">Adding External MCP Services</h4>
          <p class="text-gray-600 mb-2">
            External MCP services are configured in the <code class="bg-gray-100 px-2 py-1 rounded">external-mcp-servers.yaml</code> file.
          </p>
          <div class="bg-gray-50 p-4 rounded-lg font-mono text-xs">
            <div class="text-gray-500"># Example configuration:</div>
            <div>mcpServers:</div>
            <div class="ml-2">my-service:</div>
            <div class="ml-4">command: "npx"</div>
            <div class="ml-4">args: ["@modelcontextprotocol/server-filesystem", "/path/to/directory"]</div>
            <div class="ml-4">env:</div>
            <div class="ml-6">NODE_ENV: "production"</div>
          </div>
        </div>
        
        <div>
          <h4 class="font-medium text-gray-700 mb-2">Service Status</h4>
          <div class="space-y-1 text-gray-600">
            <div>• <span class="text-green-600">✅ Healthy</span> - Service is running and responding normally</div>
            <div>• <span class="text-orange-600">⚠️ Degraded</span> - Service has some issues but is functional</div>
            <div>• <span class="text-red-600">❌ Unhealthy</span> - Service has significant issues</div>
            <div>• <span class="text-gray-600">🔴 Down</span> - Service is not responding or crashed</div>
            <div>• <span class="text-yellow-600">❓ Unknown</span> - Service status cannot be determined</div>
            <div>• <span class="text-blue-600">🔄 Starting</span> - Service is starting up</div>
          </div>
        </div>
      </div>
    </div>


    <!-- Back to Dashboard -->
    <div class="mt-6 text-center">
      <a href="/" class="btn-secondary">
        ← Back to Dashboard
      </a>
    </div>
  </div>
</div>

<style>
  .card {
    @apply bg-white rounded-lg shadow p-6;
  }
  
  .btn-secondary {
    @apply px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded transition-colors;
  }
  
  .btn-primary {
    @apply px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors;
  }
</style>