<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { McpServerCapabilities } from '$lib/api';
  import { 
    getServerStatusColor, 
    getServerStatusIcon, 
    getServerStatusLabel,
    serverNeedsOAuth,
    serverIsAuthenticating,
    serverHasFailed,
    formatServerTimestamp
  } from '$lib/utils/serverStatus';

  export let server: McpServerCapabilities;
  export let loading = false;

  const dispatch = createEventDispatcher();

  function handleInitiateOAuth() {
    dispatch('initiate-oauth', { serverName: server.name });
  }

  function handleOpenAuthUrl() {
    if (server.oauth_auth_url) {
      window.open(server.oauth_auth_url, '_blank');
    }
  }

  $: statusColor = getServerStatusColor(server.status);
  $: statusIcon = getServerStatusIcon(server.status);
  $: statusLabel = getServerStatusLabel(server.status);
  $: needsOAuth = serverNeedsOAuth(server);
  $: isAuthenticating = serverIsAuthenticating(server);
  $: hasFailed = serverHasFailed(server);
</script>

<div class="bg-white border border-gray-200 rounded-lg p-4">
  <!-- Header -->
  <div class="flex items-center justify-between mb-3">
    <div class="flex items-center gap-2">
      <span class="text-lg">🔐</span>
      <h4 class="font-semibold text-gray-900">OAuth Authentication</h4>
    </div>
    
    <!-- Status Badge -->
    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border {statusColor}">
      {statusIcon} {statusLabel}
    </span>
  </div>

  <!-- Status Information -->
  <div class="space-y-3">
    <!-- Current Status Details -->
    <div class="text-sm">
      <div class="flex items-center justify-between">
        <span class="text-gray-600">Status:</span>
        <span class="font-medium">{statusLabel}</span>
      </div>
      <div class="flex items-center justify-between mt-1">
        <span class="text-gray-600">Last Updated:</span>
        <span class="text-xs text-gray-500">{formatServerTimestamp(server.last_updated)}</span>
      </div>
      {#if server.error_message}
        <div class="mt-2 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700">
          <strong>Error:</strong> {server.error_message}
        </div>
      {/if}
    </div>

    <!-- OAuth Configuration -->
    {#if server.config?.oauth_termination_here !== undefined}
      <div class="text-sm">
        <div class="flex items-center justify-between">
          <span class="text-gray-600">OAuth Mode:</span>
          <span class="text-xs font-mono bg-gray-100 px-2 py-1 rounded">
            {server.config.oauth_termination_here ? 'Server-Terminated' : 'Client-Terminated'}
          </span>
        </div>
      </div>
    {/if}

    <!-- Action Buttons -->
    <div class="flex flex-col gap-2 pt-2 border-t">
      {#if needsOAuth}
        <!-- OAuth Required Actions -->
        <button
          class="w-full px-3 py-2 bg-blue-600 text-white text-sm rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          on:click={handleInitiateOAuth}
          disabled={loading}
        >
          {#if loading}
            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
          {:else}
            🔐
          {/if}
          Initiate OAuth
        </button>
        
        {#if server.oauth_auth_url}
          <button
            class="w-full px-3 py-2 bg-green-600 text-white text-sm rounded-md hover:bg-green-700 flex items-center justify-center gap-2"
            on:click={handleOpenAuthUrl}
          >
            🌐 Open Auth URL
          </button>
        {/if}
      {:else if isAuthenticating}
        <!-- In Progress -->
        <div class="w-full px-3 py-2 bg-blue-100 text-blue-700 text-sm rounded-md text-center flex items-center justify-center gap-2">
          <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
          Authentication in progress...
        </div>
      {:else if hasFailed}
        <!-- Failed - Retry Option -->
        <button
          class="w-full px-3 py-2 bg-orange-600 text-white text-sm rounded-md hover:bg-orange-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          on:click={handleInitiateOAuth}
          disabled={loading}
        >
          {#if loading}
            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
          {:else}
            🔄
          {/if}
          Retry OAuth
        </button>
      {:else if server.status === 'connected'}
        <!-- Connected - Show Success -->
        <div class="w-full px-3 py-2 bg-green-100 text-green-700 text-sm rounded-md text-center flex items-center justify-center gap-2">
          ✅ Successfully Connected
        </div>
      {:else}
        <!-- Other States -->
        <div class="w-full px-3 py-2 bg-gray-100 text-gray-600 text-sm rounded-md text-center">
          {statusLabel}
        </div>
      {/if}
    </div>

    <!-- Help Text -->
    {#if needsOAuth}
      <div class="text-xs text-gray-500 bg-blue-50 p-2 rounded">
        💡 This server requires OAuth authentication. Click "Initiate OAuth" to start the authentication flow.
      </div>
    {:else if server.status === 'connected'}
      <div class="text-xs text-gray-500 bg-green-50 p-2 rounded">
        ✅ OAuth authentication successful. Server is ready to use.
      </div>
    {/if}
  </div>
</div>