<script lang="ts">
  import type { McpServerCapabilities } from '$lib/api';
  import { 
    getServerStatusColor, 
    getServerStatusIcon, 
    getServerStatusLabel,
    getServerTypeIcon,
    getServerTypeLabel,
    formatServerTimestamp,
    serverNeedsOAuth,
    serverIsHealthy,
    serverHasFailed
  } from '$lib/utils/serverStatus';

  export let server: McpServerCapabilities;

  $: statusColor = getServerStatusColor(server.status);
  $: statusIcon = getServerStatusIcon(server.status);
  $: statusLabel = getServerStatusLabel(server.status);
  $: typeIcon = getServerTypeIcon(server.type);
  $: typeLabel = getServerTypeLabel(server.type);
  $: needsOAuth = serverNeedsOAuth(server);
  $: isHealthy = serverIsHealthy(server);
  $: hasFailed = serverHasFailed(server);

  function formatUptime(uptimeSeconds: number | null | undefined): string {
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
</script>

<div class="bg-white border border-gray-200 rounded-lg p-4">
  <!-- Header -->
  <div class="flex items-center justify-between mb-4">
    <div class="flex items-center gap-3">
      <span class="text-2xl">{typeIcon}</span>
      <div>
        <h3 class="text-lg font-semibold text-gray-900">{server.name}</h3>
        <p class="text-sm text-gray-600">{typeLabel}</p>
      </div>
    </div>
    
    <!-- Status Badge -->
    <div class="flex items-center gap-3">
      <span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium border {statusColor}">
        {statusIcon} {statusLabel}
      </span>
      {#if server.enabled}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-700 border border-green-200">
          Enabled
        </span>
      {:else}
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-gray-100 text-gray-700 border border-gray-200">
          Disabled
        </span>
      {/if}
    </div>
  </div>

  <!-- Server Information Grid -->
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-4">
    <div>
      <p class="text-xs text-gray-500 mb-1">Type</p>
      <p class="text-sm font-medium">{typeLabel}</p>
    </div>
    
    <div>
      <p class="text-xs text-gray-500 mb-1">Status</p>
      <div class="flex items-center gap-1">
        <span class="text-sm">{statusIcon}</span>
        <span class="text-sm font-medium">{statusLabel}</span>
      </div>
    </div>
    
    <div>
      <p class="text-xs text-gray-500 mb-1">Tools</p>
      <p class="text-sm font-medium">{server.tools_count} available</p>
    </div>
    
    <div>
      <p class="text-xs text-gray-500 mb-1">Last Updated</p>
      <p class="text-xs text-gray-600">{formatServerTimestamp(server.last_updated)}</p>
    </div>
  </div>

  <!-- Additional Information -->
  {#if server.config?.base_url}
    <div class="mb-3">
      <p class="text-xs text-gray-500 mb-1">Base URL</p>
      <p class="text-sm font-mono bg-gray-50 px-2 py-1 rounded text-gray-700 break-all">
        {server.config.base_url}
      </p>
    </div>
  {/if}

  {#if server.config?.command}
    <div class="mb-3">
      <p class="text-xs text-gray-500 mb-1">Command</p>
      <p class="text-sm font-mono bg-gray-50 px-2 py-1 rounded text-gray-700">
        {Array.isArray(server.config.command) ? server.config.command.join(' ') : server.config.command}
      </p>
    </div>
  {/if}

  <!-- Runtime Information (if connected) -->
  {#if isHealthy && (server.uptime_seconds !== undefined || server.protocol_version)}
    <div class="border-t pt-3 mt-3">
      <h4 class="text-sm font-medium text-gray-900 mb-2">Runtime Information</h4>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
        {#if server.uptime_seconds !== undefined}
          <div>
            <span class="text-gray-600">Uptime:</span>
            <span class="font-medium ml-2">{formatUptime(server.uptime_seconds)}</span>
          </div>
        {/if}
        {#if server.protocol_version}
          <div>
            <span class="text-gray-600">Protocol:</span>
            <span class="font-medium ml-2">{server.protocol_version}</span>
          </div>
        {/if}
        {#if server.server_info}
          <div class="md:col-span-2">
            <span class="text-gray-600">Server Info:</span>
            <span class="font-medium ml-2">{server.server_info}</span>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Capabilities (if available) -->
  {#if server.capabilities}
    <div class="border-t pt-3 mt-3">
      <h4 class="text-sm font-medium text-gray-900 mb-2">Capabilities</h4>
      <div class="flex flex-wrap gap-2">
        {#each Object.entries(server.capabilities) as [capability, supported]}
          <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {supported ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'}">
            {supported ? '✅' : '❌'} {capability}
          </span>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Error Information (if failed) -->
  {#if hasFailed && server.error_message}
    <div class="border-t pt-3 mt-3">
      <div class="bg-red-50 border border-red-200 rounded-lg p-3">
        <div class="flex items-start gap-2">
          <span class="text-red-500 mt-0.5">⚠️</span>
          <div>
            <h4 class="text-sm font-medium text-red-800 mb-1">Error Details</h4>
            <p class="text-sm text-red-700">{server.error_message}</p>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- OAuth Notice (if OAuth server) -->
  {#if server.type === 'oauth' && needsOAuth}
    <div class="border-t pt-3 mt-3">
      <div class="bg-blue-50 border border-blue-200 rounded-lg p-3">
        <div class="flex items-start gap-2">
          <span class="text-blue-500 mt-0.5">🔐</span>
          <div>
            <h4 class="text-sm font-medium text-blue-800 mb-1">OAuth Required</h4>
            <p class="text-sm text-blue-700">This server requires OAuth authentication before it can be used.</p>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>