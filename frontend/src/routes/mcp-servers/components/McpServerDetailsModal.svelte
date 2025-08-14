<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { api, type McpServerDetailsResponse } from '$lib/api';

  export let serverName: string;

  const dispatch = createEventDispatcher();

  let serverDetails: McpServerDetailsResponse | null = null;
  let loading = true;
  let error = '';

  async function loadServerDetails() {
    try {
      loading = true;
      error = '';
      serverDetails = await api.getMcpServerDetails(serverName);
    } catch (err) {
      console.error('Failed to load server details:', err);
      error = err instanceof Error ? err.message : 'Failed to load server details';
    } finally {
      loading = false;
    }
  }

  function getFullCapabilitiesCount(serverDetails: McpServerDetailsResponse): { enabled: number; total: number } {
    if (!serverDetails) return { enabled: 0, total: 0 };
    
    // Count basic capabilities
    const basicCapabilities = [
      serverDetails.capabilities.sampling,
      serverDetails.capabilities.elicitation,
      serverDetails.capabilities.tools,
      serverDetails.capabilities.resources,
      serverDetails.capabilities.prompts,
      serverDetails.capabilities.roots
    ];
    
    let enabledCount = basicCapabilities.filter(Boolean).length;
    let totalCount = basicCapabilities.length;
    
    // Count notification capabilities if available
    if (serverDetails.notification_capabilities) {
      const notificationCapabilities = [
        serverDetails.notification_capabilities.tools_list_changed,
        serverDetails.notification_capabilities.resources_list_changed,
        serverDetails.notification_capabilities.prompts_list_changed,
        serverDetails.notification_capabilities.resource_subscriptions
      ];
      
      enabledCount += notificationCapabilities.filter(Boolean).length;
      totalCount += notificationCapabilities.length;
    }
    
    return { enabled: enabledCount, total: totalCount };
  }

  function closeModal() {
    dispatch('close');
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeModal();
    }
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

  function getTypeInfo(type: string) {
    switch (type) {
      case 'internal':
        return { icon: '🏠', label: 'Internal Server', color: 'bg-green-100 text-green-800' };
      case 'external_mcp':
        return { icon: '🔌', label: 'External MCP Server', color: 'bg-purple-100 text-purple-800' };
      default:
        return { icon: '❓', label: 'Unknown', color: 'bg-gray-100 text-gray-800' };
    }
  }

  onMount(() => {
    loadServerDetails();
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<!-- Modal Backdrop -->
<div 
  class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50"
  on:click={closeModal}
>
  <!-- Modal Content -->
  <div 
    class="relative top-20 mx-auto p-5 border w-11/12 md:w-3/4 lg:w-1/2 shadow-lg rounded-md bg-white"
    on:click|stopPropagation
  >
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div class="flex items-center gap-3">
        <h2 class="text-xl font-semibold text-gray-900">MCP Server Details</h2>
        {#if serverDetails}
          {@const typeInfo = getTypeInfo(serverDetails.type)}
          <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {typeInfo.color}">
            <span class="mr-1">{typeInfo.icon}</span>
            {typeInfo.label}
          </span>
        {/if}
      </div>
      <button
        on:click={closeModal}
        class="text-gray-400 hover:text-gray-600 transition-colors"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
        </svg>
      </button>
    </div>

    {#if error}
      <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
        <strong class="font-bold">Error:</strong>
        <span class="block sm:inline">{error}</span>
      </div>
    {/if}

    {#if loading}
      <div class="flex justify-center items-center py-8">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        <span class="ml-3 text-gray-600">Loading server details...</span>
      </div>
    {:else if serverDetails}
      <!-- Server Overview -->
      <div class="bg-gray-50 rounded-lg p-4 mb-6">
        <h3 class="text-lg font-medium text-gray-900 mb-3">{serverDetails.name}</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <p class="text-sm font-medium text-gray-600">Status</p>
            <p class="text-sm text-gray-900 flex items-center gap-1">
              {#if serverDetails.is_running}
                <span class="text-green-600">✅ Running</span>
              {:else}
                <span class="text-red-600">🔴 Stopped</span>
              {/if}
            </p>
          </div>
          
          <div>
            <p class="text-sm font-medium text-gray-600">Protocol Version</p>
            <p class="text-sm text-gray-900">{serverDetails.protocol_version}</p>
          </div>
          
          <div>
            <p class="text-sm font-medium text-gray-600">Capabilities</p>
            <p class="text-sm text-gray-900">{getFullCapabilitiesCount(serverDetails).enabled}/{getFullCapabilitiesCount(serverDetails).total} supported</p>
          </div>
          
          <div>
            <p class="text-sm font-medium text-gray-600">Uptime</p>
            <p class="text-sm text-gray-900">{formatUptime(serverDetails.uptime_seconds)}</p>
          </div>
          
          <div>
            <p class="text-sm font-medium text-gray-600">Tools Count</p>
            <p class="text-sm text-gray-900">{serverDetails.tools_count} tools</p>
          </div>
        </div>

        {#if serverDetails.server_info}
          <div class="mt-4">
            <p class="text-sm font-medium text-gray-600">Server Information</p>
            <p class="text-sm text-gray-900">{serverDetails.server_info}</p>
          </div>
        {/if}
      </div>

      <!-- Capabilities -->
      <div class="mb-6">
        <h4 class="text-base font-medium text-gray-900 mb-3">Supported Capabilities</h4>
        <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
          <div class="flex items-center gap-2">
            <div class="w-4 h-4 rounded-full {serverDetails.capabilities.sampling ? 'bg-green-500' : 'bg-gray-300'}"></div>
            <span class="text-sm text-gray-900">📊 Sampling</span>
          </div>
          
          <div class="flex items-center gap-2">
            <div class="w-4 h-4 rounded-full {serverDetails.capabilities.elicitation ? 'bg-green-500' : 'bg-gray-300'}"></div>
            <span class="text-sm text-gray-900">💬 Elicitation</span>
          </div>
          
          <div class="flex items-center gap-2">
            <div class="w-4 h-4 rounded-full {serverDetails.capabilities.tools ? 'bg-green-500' : 'bg-gray-300'}"></div>
            <span class="text-sm text-gray-900">🔧 Tools</span>
          </div>
          
          <div class="flex items-center gap-2">
            <div class="w-4 h-4 rounded-full {serverDetails.capabilities.resources ? 'bg-green-500' : 'bg-gray-300'}"></div>
            <span class="text-sm text-gray-900">📄 Resources</span>
          </div>
          
          <div class="flex items-center gap-2">
            <div class="w-4 h-4 rounded-full {serverDetails.capabilities.prompts ? 'bg-green-500' : 'bg-gray-300'}"></div>
            <span class="text-sm text-gray-900">💭 Prompts</span>
          </div>
          
          <div class="flex items-center gap-2">
            <div class="w-4 h-4 rounded-full {serverDetails.capabilities.roots ? 'bg-green-500' : 'bg-gray-300'}"></div>
            <span class="text-sm text-gray-900">🌳 Roots</span>
          </div>
        </div>
      </div>

      <!-- Notification Capabilities -->
      {#if serverDetails.notification_capabilities}
        <div class="mb-6">
          <h4 class="text-base font-medium text-gray-900 mb-3">Notification Capabilities</h4>
          <div class="grid grid-cols-2 md:grid-cols-2 gap-3">
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded-full {serverDetails.notification_capabilities.tools_list_changed ? 'bg-green-500' : 'bg-gray-300'}"></div>
              <span class="text-sm text-gray-900">🔧📢 Tools List Changed</span>
            </div>
            
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded-full {serverDetails.notification_capabilities.resources_list_changed ? 'bg-green-500' : 'bg-gray-300'}"></div>
              <span class="text-sm text-gray-900">📄📢 Resources List Changed</span>
            </div>
            
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded-full {serverDetails.notification_capabilities.prompts_list_changed ? 'bg-green-500' : 'bg-gray-300'}"></div>
              <span class="text-sm text-gray-900">💭📢 Prompts List Changed</span>
            </div>
            
            <div class="flex items-center gap-2">
              <div class="w-4 h-4 rounded-full {serverDetails.notification_capabilities.resource_subscriptions ? 'bg-green-500' : 'bg-gray-300'}"></div>
              <span class="text-sm text-gray-900">📄🔔 Resource Subscriptions</span>
            </div>
          </div>
        </div>
      {/if}

      <!-- Tools List -->
      {#if serverDetails.tools.length > 0}
        <div class="mb-6">
          <h4 class="text-base font-medium text-gray-900 mb-3">Available Tools ({serverDetails.tools.length})</h4>
          <div class="bg-gray-50 rounded-lg p-4 max-h-64 overflow-y-auto">
            <div class="space-y-3">
              {#each serverDetails.tools as tool}
                <div class="bg-white rounded p-3 border">
                  <div class="flex items-start justify-between">
                    <div>
                      <h5 class="font-medium text-gray-900">{tool.name}</h5>
                      {#if tool.description}
                        <p class="text-sm text-gray-600 mt-1">{tool.description}</p>
                      {/if}
                    </div>
                  </div>
                  
                  {#if tool.input_schema}
                    <details class="mt-2">
                      <summary class="text-xs text-blue-600 cursor-pointer hover:text-blue-800">
                        View Input Schema
                      </summary>
                      <pre class="text-xs bg-gray-100 p-2 rounded mt-2 overflow-x-auto">{JSON.stringify(tool.input_schema, null, 2)}</pre>
                    </details>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        </div>
      {:else}
        <div class="mb-6">
          <h4 class="text-base font-medium text-gray-900 mb-3">Available Tools</h4>
          <div class="text-center py-4 text-gray-500 bg-gray-50 rounded-lg">
            No tools available
          </div>
        </div>
      {/if}
    {/if}

    <!-- Footer -->
    <div class="flex justify-end pt-4 border-t border-gray-200">
      <button
        on:click={closeModal}
        class="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition-colors"
      >
        Close
      </button>
    </div>
  </div>
</div>