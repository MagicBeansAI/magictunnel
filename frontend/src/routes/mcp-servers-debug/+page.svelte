<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type McpServersResponse } from '$lib/api';

  let serversResponse: McpServersResponse | null = null;
  let loading = true;
  let error = '';

  async function loadServers() {
    try {
      console.log('Starting to load servers...');
      loading = true;
      error = '';
      
      const servers = await api.getMcpServers();
      console.log('Servers loaded:', servers);
      
      serversResponse = servers;
    } catch (err) {
      console.error('Failed to load MCP servers:', err);
      error = err instanceof Error ? err.message : 'Failed to load MCP servers';
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    console.log('Component mounted, loading servers...');
    loadServers();
  });
</script>

<div class="container mx-auto px-6 py-8">
  <h1 class="text-3xl font-bold text-gray-900 mb-8">MCP Servers Debug</h1>

  {#if error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6">
      <strong class="font-bold">Error:</strong>
      <span class="block sm:inline">{error}</span>
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center items-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      <span class="ml-3 text-gray-600">Loading MCP servers...</span>
    </div>
  {:else if serversResponse}
    <div class="bg-white p-6 rounded-lg shadow border mb-6">
      <h2 class="text-xl font-bold mb-4">Raw Server Data</h2>
      <pre class="bg-gray-100 p-4 rounded text-sm overflow-x-auto">{JSON.stringify(serversResponse, null, 2)}</pre>
    </div>

    <div class="space-y-4">
      <h2 class="text-xl font-bold">Servers ({serversResponse.servers.length})</h2>
      {#each serversResponse.servers as server}
        <div class="bg-white p-4 rounded-lg shadow border">
          <h3 class="font-semibold">{server.name}</h3>
          <p class="text-sm text-gray-600">Type: {server.type || 'unknown'}</p>
          <p class="text-sm text-gray-600">Status: {server.status || 'unknown'}</p>
          <p class="text-sm text-gray-600">Tools: {server.tools_count}</p>
        </div>
      {/each}
    </div>
  {/if}
</div>