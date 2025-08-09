<script lang="ts">
  import MonitoringDashboard from '$lib/components/MonitoringDashboard.svelte';
  
  // Page metadata
  let title = 'System Monitoring - MagicTunnel';
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content="Real-time system monitoring and observability dashboard for MagicTunnel" />
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <!-- Page Header -->
  <div class="bg-white border-b border-gray-200">
    <div class="container mx-auto px-4 py-6">
      <div class="flex items-center justify-between">
        <div>
          <nav class="flex text-sm text-gray-500 mb-2">
            <a href="/" class="hover:text-gray-700 transition-colors">Dashboard</a>
            <span class="mx-2">›</span>
            <span class="text-gray-900 font-medium">Monitoring</span>
          </nav>
          <h1 class="text-3xl font-bold text-gray-900">System Monitoring</h1>
          <p class="text-gray-600 mt-1">Real-time system health, performance metrics, and alerts</p>
        </div>
        
        <!-- Quick Actions -->
        <div class="flex items-center space-x-3">
          <a 
            href="/services" 
            class="inline-flex items-center px-3 py-2 border border-transparent text-sm leading-4 font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors"
          >
            🔗 Manage Services
          </a>
        </div>
      </div>
    </div>
  </div>

  <!-- Monitoring Dashboard -->
  <MonitoringDashboard 
    refreshInterval={30000}
    showAlerts={true}
    showSystemMetrics={true}
    showHealthStatus={true}
    showServiceMetrics={true}
  />
</div>

<style>
  /* Custom page styles if needed */
</style>