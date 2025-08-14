<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type McpServersResponse, type McpServerCapabilities } from '$lib/api';
  import McpServerCard from './components/McpServerCard.svelte';
  import McpServerDetailsModal from './components/McpServerDetailsModal.svelte';

  let serversResponse: McpServersResponse | null = null;
  let loading = true;
  let error = '';
  let selectedServer: string | null = null;
  let refreshInterval: number;

  // Auto refresh every 30 seconds
  const REFRESH_INTERVAL = 30000;

  async function loadServers() {
    try {
      loading = true;
      error = '';
      serversResponse = await api.getMcpServers();
    } catch (err) {
      console.error('Failed to load MCP servers:', err);
      error = err instanceof Error ? err.message : 'Failed to load MCP servers';
    } finally {
      loading = false;
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

  onMount(() => {
    loadServers();
    
    // Set up auto-refresh
    refreshInterval = setInterval(loadServers, REFRESH_INTERVAL);
    
    // Cleanup interval on component destroy
    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
      }
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
    <!-- Statistics Overview -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-sm font-medium text-gray-600">Total Servers</p>
            <p class="text-2xl font-bold text-gray-900">{serversResponse.total_servers}</p>
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
            <p class="text-2xl font-bold text-gray-900">{serversResponse.internal_servers}</p>
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
    </div>

    <!-- Servers List -->
    <div class="space-y-4">
      {#each serversResponse.servers as server (server.name)}
        <McpServerCard 
          {server} 
          on:viewDetails={() => openServerDetails(server.name)}
        />
      {/each}
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