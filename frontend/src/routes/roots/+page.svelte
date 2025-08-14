<script lang="ts">
  import { onMount } from 'svelte';
  import { rootsApi, type Root, type RootsServiceStatus, type PaginatedRootsResponse } from '$lib/api/roots';
  import RootsDiscoveryCard from './components/RootsDiscoveryCard.svelte';
  import SecurityConfigPanel from './components/SecurityConfigPanel.svelte';
  import ManualRootsManager from './components/ManualRootsManager.svelte';
  import RootsStatusMonitor from './components/RootsStatusMonitor.svelte';
  import PermissionsMatrix from './components/PermissionsMatrix.svelte';

  // Page state
  let activeTab = 'discovery';
  let loading = true;
  let error: string | null = null;

  // Roots data
  let rootsData: PaginatedRootsResponse | null = null;
  let serviceStatus: RootsServiceStatus | null = null;
  let refreshing = false;

  // Tab configuration
  const tabs = [
    { id: 'discovery', label: 'Discovery', icon: '🔍', description: 'Browse discovered filesystem and URI roots' },
    { id: 'security', label: 'Security', icon: '🔒', description: 'Configure access patterns and permissions' },
    { id: 'management', label: 'Management', icon: '⚙️', description: 'Manually add and manage custom roots' },
    { id: 'permissions', label: 'Permissions', icon: '👥', description: 'View and manage permission matrix' },
    { id: 'monitoring', label: 'Monitoring', icon: '📊', description: 'Service health and performance metrics' },
  ];

  onMount(async () => {
    await loadInitialData();
  });

  async function loadInitialData() {
    loading = true;
    error = null;

    try {
      // Load both roots and status in parallel
      const [rootsResult, statusResult] = await Promise.all([
        rootsApi.listRoots({ limit: 50, accessible_only: false }),
        rootsApi.getStatus(),
      ]);

      rootsData = rootsResult;
      serviceStatus = statusResult;
    } catch (err) {
      error = err instanceof Error ? err.message : 'Failed to load roots data';
      console.error('Failed to load roots data:', err);
    } finally {
      loading = false;
    }
  }

  async function refreshData() {
    if (refreshing) return;
    
    refreshing = true;
    try {
      await loadInitialData();
    } finally {
      refreshing = false;
    }
  }

  async function triggerDiscovery() {
    try {
      const result = await rootsApi.triggerDiscovery();
      console.log('Discovery completed:', result);
      
      // Refresh data after discovery
      await refreshData();
      
      // Show success message (you could add a toast notification here)
      alert(`Discovery completed! Found ${result.total_discovered} roots (${result.new_roots} new, ${result.updated_roots} updated) in ${rootsApi.formatDuration(result.duration_ms)}`);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Discovery failed';
      console.error('Discovery failed:', err);
      alert(`Discovery failed: ${errorMsg}`);
    }
  }

  function handleTabChange(tabId: string) {
    activeTab = tabId;
  }

  // Auto-refresh every 30 seconds when on monitoring tab
  let autoRefreshInterval: number | null = null;
  $: {
    if (autoRefreshInterval) {
      clearInterval(autoRefreshInterval);
      autoRefreshInterval = null;
    }
    
    if (activeTab === 'monitoring') {
      autoRefreshInterval = setInterval(async () => {
        try {
          serviceStatus = await rootsApi.getStatus();
        } catch (err) {
          console.error('Auto-refresh failed:', err);
        }
      }, 30000);
    }
  }
</script>

<svelte:head>
  <title>MCP Roots Management - MagicTunnel</title>
</svelte:head>

<div class="roots-page">
  <!-- Page Header -->
  <div class="page-header">
    <div class="header-content">
      <div class="title-section">
        <h1 class="page-title">
          🗂️ MCP Roots Management
        </h1>
        <p class="page-description">
          Manage filesystem and URI boundaries for MCP tool access
        </p>
      </div>
      
      <div class="header-actions">
        <button 
          class="action-btn secondary" 
          on:click={refreshData}
          disabled={refreshing}
        >
          {refreshing ? '🔄' : '↻'} 
          {refreshing ? 'Refreshing...' : 'Refresh'}
        </button>
        
        <button 
          class="action-btn primary" 
          on:click={triggerDiscovery}
          disabled={loading || refreshing}
        >
          🔍 Discover Roots
        </button>
      </div>
    </div>

    <!-- Status Bar -->
    {#if serviceStatus}
      <div class="status-bar" class:healthy={serviceStatus.healthy} class:unhealthy={!serviceStatus.healthy}>
        <div class="status-info">
          <span class="status-indicator" class:healthy={serviceStatus.healthy} class:unhealthy={!serviceStatus.healthy}>
            {serviceStatus.healthy ? '✅' : '❌'}
          </span>
          <span class="status-text">
            {serviceStatus.healthy ? 'Service Healthy' : 'Service Issues Detected'}
          </span>
          <span class="status-details">
            {serviceStatus.total_roots} roots ({serviceStatus.accessible_roots} accessible, {serviceStatus.manual_roots} manual)
          </span>
        </div>
        <div class="cache-status">
          Cache: {serviceStatus.cache_status}
        </div>
      </div>
    {/if}
  </div>

  <!-- Navigation Tabs -->
  <div class="tab-navigation">
    {#each tabs as tab}
      <button
        class="tab-button"
        class:active={activeTab === tab.id}
        on:click={() => handleTabChange(tab.id)}
        title={tab.description}
      >
        <span class="tab-icon">{tab.icon}</span>
        <span class="tab-label">{tab.label}</span>
      </button>
    {/each}
  </div>

  <!-- Content Area -->
  <div class="content-area">
    {#if loading}
      <div class="loading-state">
        <div class="loading-spinner"></div>
        <p>Loading roots data...</p>
      </div>
    {:else if error}
      <div class="error-state">
        <div class="error-icon">❌</div>
        <h3>Failed to Load Roots</h3>
        <p>{error}</p>
        <button class="action-btn primary" on:click={loadInitialData}>
          Try Again
        </button>
      </div>
    {:else}
      <!-- Tab Content -->
      {#if activeTab === 'discovery'}
        <RootsDiscoveryCard 
          {rootsData} 
          on:refresh={refreshData}
          on:discover={triggerDiscovery}
        />
      {:else if activeTab === 'security'}
        <SecurityConfigPanel 
          on:configUpdated={refreshData}
        />
      {:else if activeTab === 'management'}
        <ManualRootsManager 
          manualRoots={rootsData?.roots.filter(r => r.manual) || []}
          on:rootAdded={refreshData}
          on:rootRemoved={refreshData}
        />
      {:else if activeTab === 'permissions'}
        <PermissionsMatrix 
          roots={rootsData?.roots || []}
          on:permissionsUpdated={refreshData}
        />
      {:else if activeTab === 'monitoring'}
        <RootsStatusMonitor 
          {serviceStatus}
          {rootsData}
          on:refresh={refreshData}
        />
      {/if}
    {/if}
  </div>
</div>

<style>
  .roots-page {
    padding: 1.5rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .page-header {
    margin-bottom: 2rem;
  }

  .header-content {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 2rem;
    margin-bottom: 1rem;
  }

  .title-section {
    flex: 1;
  }

  .page-title {
    font-size: 2rem;
    font-weight: 600;
    color: #1a1a1a;
    margin: 0 0 0.5rem 0;
  }

  .page-description {
    font-size: 1rem;
    color: #666;
    margin: 0;
  }

  .header-actions {
    display: flex;
    gap: 1rem;
  }

  .action-btn {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 8px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .action-btn.primary {
    background: #3b82f6;
    color: white;
  }

  .action-btn.primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .action-btn.secondary {
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #d1d5db;
  }

  .action-btn.secondary:hover:not(:disabled) {
    background: #e5e7eb;
  }

  .action-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .status-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    font-size: 0.875rem;
  }

  .status-bar.healthy {
    background: #f0f9ff;
    border-color: #bae6fd;
  }

  .status-bar.unhealthy {
    background: #fef2f2;
    border-color: #fecaca;
  }

  .status-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .status-indicator {
    font-size: 1rem;
  }

  .status-text {
    font-weight: 500;
  }

  .status-details {
    color: #6b7280;
  }

  .cache-status {
    color: #6b7280;
    text-transform: capitalize;
  }

  .tab-navigation {
    display: flex;
    gap: 0.25rem;
    background: #f9fafb;
    padding: 0.25rem;
    border-radius: 10px;
    margin-bottom: 2rem;
    overflow-x: auto;
  }

  .tab-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    background: transparent;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
    color: #6b7280;
    font-weight: 500;
  }

  .tab-button:hover {
    background: #e5e7eb;
    color: #374151;
  }

  .tab-button.active {
    background: white;
    color: #3b82f6;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .tab-icon {
    font-size: 1rem;
  }

  .content-area {
    min-height: 500px;
  }

  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    color: #6b7280;
  }

  .loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 2px solid #e5e7eb;
    border-top: 2px solid #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    text-align: center;
  }

  .error-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .error-state h3 {
    color: #dc2626;
    margin-bottom: 0.5rem;
  }

  .error-state p {
    color: #6b7280;
    margin-bottom: 2rem;
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .roots-page {
      padding: 1rem;
    }

    .header-content {
      flex-direction: column;
      gap: 1rem;
    }

    .header-actions {
      width: 100%;
      justify-content: stretch;
    }

    .action-btn {
      flex: 1;
      justify-content: center;
    }

    .tab-navigation {
      overflow-x: auto;
      scrollbar-width: none;
      -ms-overflow-style: none;
    }

    .tab-navigation::-webkit-scrollbar {
      display: none;
    }

    .status-bar {
      flex-direction: column;
      gap: 0.5rem;
      align-items: flex-start;
    }
  }
</style>