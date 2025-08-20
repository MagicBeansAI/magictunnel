<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { McpServerCapabilities } from '$lib/api';

  export let server: McpServerCapabilities;

  const dispatch = createEventDispatcher();

  function getServerTypeInfo(server: McpServerCapabilities) {
    if (server.name === 'magictunnel-internal') {
      return {
        icon: '🏠',
        type: 'Internal',
        color: 'bg-green-100 text-green-800'
      };
    }
    return {
      icon: '🔌',
      type: 'External MCP',
      color: 'bg-purple-100 text-purple-800'
    };
  }

  function getStatusInfo(isRunning: boolean) {
    return isRunning 
      ? { icon: '✅', text: 'Running', color: 'text-green-600' }
      : { icon: '🔴', text: 'Stopped', color: 'text-red-600' };
  }

  function formatUptime(uptimeSeconds: number | null): string {
    if (!uptimeSeconds) return 'Not running';
    
    const hours = Math.floor(uptimeSeconds / 3600);
    const minutes = Math.floor((uptimeSeconds % 3600) / 60);
    const seconds = uptimeSeconds % 60;
    
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
      return `${minutes}m ${seconds}s`;
    } else {
      return `${seconds}s`;
    }
  }

  function getCapabilitiesCount(server: McpServerCapabilities): { enabled: number; total: number } {
    const basicCapabilities = [
      server.capabilities.sampling,
      server.capabilities.elicitation,
      server.capabilities.tools,
      server.capabilities.resources,
      server.capabilities.prompts,
      server.capabilities.roots
    ];
    
    let enabledCount = basicCapabilities.filter(Boolean).length;
    let totalCount = basicCapabilities.length;
    
    // Note: notification_capabilities are only available in the details view
    // For the card view, we show basic capabilities only (6 total)
    // The details modal will show the full 10 capabilities including notifications
    
    return { enabled: enabledCount, total: totalCount };
  }

  $: typeInfo = getServerTypeInfo(server);
  $: statusInfo = getStatusInfo(server.is_running);
  $: capabilitiesCount = getCapabilitiesCount(server);
</script>

<div class="bg-white border border-gray-200 rounded-lg p-6 hover:shadow-md transition-shadow">
  <div class="flex items-start justify-between mb-4">
    <div class="flex items-start gap-4">
      <div class="text-2xl">{typeInfo.icon}</div>
      <div>
        <h3 class="text-lg font-semibold text-gray-900">{server.name}</h3>
        <div class="flex items-center gap-2 mt-1">
          <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {typeInfo.color}">
            {typeInfo.type}
          </span>
          <span class="inline-flex items-center gap-1 text-sm {statusInfo.color}">
            <span class="text-xs">{statusInfo.icon}</span>
            {statusInfo.text}
          </span>
        </div>
      </div>
    </div>
    
    <button
      on:click={() => dispatch('viewDetails')}
      class="px-3 py-1 text-sm text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded transition-colors"
    >
      View Details →
    </button>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
    <!-- Protocol Version -->
    <div>
      <p class="text-sm text-gray-600">Protocol Version</p>
      <p class="text-sm font-medium text-gray-900">{server.protocol_version}</p>
    </div>

    <!-- Uptime -->
    <div>
      <p class="text-sm text-gray-600">Uptime</p>
      <p class="text-sm font-medium text-gray-900">{formatUptime(server.uptime_seconds)}</p>
    </div>

    <!-- Available Tools -->
    <div>
      <p class="text-sm text-gray-600">Available Tools</p>
      <p class="text-sm font-medium text-gray-900">{server.tools_count}</p>
    </div>

    <!-- Capabilities Count -->
    <div>
      <p class="text-sm text-gray-600">Capabilities</p>
      <p class="text-sm font-medium text-gray-900">{capabilitiesCount.enabled}/{capabilitiesCount.total} supported</p>
    </div>
  </div>

  <!-- Server Info -->
  {#if server.server_info}
    <div class="mb-4">
      <p class="text-sm text-gray-600">Server Info</p>
      <p class="text-sm text-gray-900">{server.server_info}</p>
    </div>
  {/if}

  <!-- Capabilities Overview -->
  <div>
    <p class="text-sm text-gray-600 mb-2">Supported Capabilities</p>
    <div class="flex flex-wrap gap-2">
      {#if server.capabilities.sampling}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
          📊 Sampling
        </span>
      {/if}
      {#if server.capabilities.elicitation}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800">
          💬 Elicitation
        </span>
      {/if}
      {#if server.capabilities.tools}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-purple-100 text-purple-800">
          🔧 Tools
        </span>
      {/if}
      {#if server.capabilities.resources}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-orange-100 text-orange-800">
          📄 Resources
        </span>
      {/if}
      {#if server.capabilities.prompts}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-pink-100 text-pink-800">
          💭 Prompts
        </span>
      {/if}
      {#if server.capabilities.roots}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800">
          🌳 Roots
        </span>
      {/if}
      
      {#if capabilitiesCount.enabled === 0}
        <span class="text-sm text-gray-500">No capabilities detected</span>
      {/if}
    </div>
  </div>
</div>