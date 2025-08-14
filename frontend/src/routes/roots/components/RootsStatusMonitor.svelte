<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { rootsApi, type RootsServiceStatus, type PaginatedRootsResponse } from '$lib/api/roots';

  export let serviceStatus: RootsServiceStatus | null;
  export let rootsData: PaginatedRootsResponse | null;

  const dispatch = createEventDispatcher();

  // Calculate additional metrics
  $: metrics = calculateMetrics(serviceStatus, rootsData);

  function calculateMetrics(status: RootsServiceStatus | null, data: PaginatedRootsResponse | null) {
    if (!status || !data) {
      return {
        accessibilityRate: 0,
        manualRootRate: 0,
        averagePermissions: 0,
        rootTypeDistribution: {},
        permissionDistribution: {},
      };
    }

    const roots = data.roots;
    const accessibilityRate = status.total_roots > 0 ? (status.accessible_roots / status.total_roots) * 100 : 0;
    const manualRootRate = status.total_roots > 0 ? (status.manual_roots / status.total_roots) * 100 : 0;
    
    const totalPermissions = roots.reduce((sum, root) => sum + root.permissions.length, 0);
    const averagePermissions = roots.length > 0 ? totalPermissions / roots.length : 0;

    // Root type distribution
    const rootTypeDistribution = roots.reduce((acc, root) => {
      acc[root.root_type] = (acc[root.root_type] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    // Permission distribution
    const permissionDistribution = roots.reduce((acc, root) => {
      root.permissions.forEach(permission => {
        acc[permission] = (acc[permission] || 0) + 1;
      });
      return acc;
    }, {} as Record<string, number>);

    return {
      accessibilityRate: Math.round(accessibilityRate),
      manualRootRate: Math.round(manualRootRate),
      averagePermissions: Math.round(averagePermissions * 10) / 10,
      rootTypeDistribution,
      permissionDistribution,
    };
  }

  function formatLastDiscovery(dateString: string | undefined): string {
    if (!dateString) return 'Never';
    
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins} minutes ago`;
    if (diffHours < 24) return `${diffHours} hours ago`;
    return `${diffDays} days ago`;
  }

  function getHealthColor(healthy: boolean): string {
    return healthy ? '#10b981' : '#ef4444';
  }

  function getHealthIcon(healthy: boolean): string {
    return healthy ? '✅' : '❌';
  }

  function getCacheStatusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'active':
        return '#10b981';
      case 'inactive':
        return '#6b7280';
      default:
        return '#f59e0b';
    }
  }

  function handleRefresh() {
    dispatch('refresh');
  }
</script>

<div class="status-monitor">
  <div class="monitor-header">
    <div class="header-left">
      <h2 class="monitor-title">📊 Service Monitoring</h2>
      <p class="monitor-description">Real-time status and performance metrics</p>
    </div>
    <button class="refresh-btn" on:click={handleRefresh}>
      🔄 Refresh
    </button>
  </div>

  {#if serviceStatus}
    <!-- Service Health Overview -->
    <div class="health-overview">
      <div class="health-card" style="border-color: {getHealthColor(serviceStatus.healthy)}">
        <div class="health-header">
          <span class="health-icon">{getHealthIcon(serviceStatus.healthy)}</span>
          <span class="health-status" style="color: {getHealthColor(serviceStatus.healthy)}">
            {serviceStatus.healthy ? 'Healthy' : 'Issues Detected'}
          </span>
        </div>
        <div class="health-details">
          <div class="detail-item">
            <span class="detail-label">Cache Status:</span>
            <span class="detail-value" style="color: {getCacheStatusColor(serviceStatus.cache_status)}">
              {serviceStatus.cache_status}
            </span>
          </div>
          <div class="detail-item">
            <span class="detail-label">Last Discovery:</span>
            <span class="detail-value">{formatLastDiscovery(serviceStatus.last_discovery)}</span>
          </div>
          {#if serviceStatus.discovery_duration_ms}
            <div class="detail-item">
              <span class="detail-label">Discovery Duration:</span>
              <span class="detail-value">{rootsApi.formatDuration(serviceStatus.discovery_duration_ms)}</span>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Metrics Grid -->
    <div class="metrics-grid">
      <!-- Total Roots -->
      <div class="metric-card">
        <div class="metric-icon">📁</div>
        <div class="metric-content">
          <div class="metric-value">{serviceStatus.total_roots}</div>
          <div class="metric-label">Total Roots</div>
        </div>
      </div>

      <!-- Accessible Roots -->
      <div class="metric-card">
        <div class="metric-icon">✅</div>
        <div class="metric-content">
          <div class="metric-value">{serviceStatus.accessible_roots}</div>
          <div class="metric-label">Accessible</div>
          <div class="metric-percentage">{metrics.accessibilityRate}%</div>
        </div>
      </div>

      <!-- Manual Roots -->
      <div class="metric-card">
        <div class="metric-icon">⚙️</div>
        <div class="metric-content">
          <div class="metric-value">{serviceStatus.manual_roots}</div>
          <div class="metric-label">Manual</div>
          <div class="metric-percentage">{metrics.manualRootRate}%</div>
        </div>
      </div>

      <!-- Average Permissions -->
      <div class="metric-card">
        <div class="metric-icon">🔑</div>
        <div class="metric-content">
          <div class="metric-value">{metrics.averagePermissions}</div>
          <div class="metric-label">Avg Permissions</div>
        </div>
      </div>
    </div>

    <!-- Distribution Charts -->
    <div class="distribution-section">
      <div class="distribution-grid">
        <!-- Root Type Distribution -->
        <div class="distribution-card">
          <h3 class="distribution-title">Root Type Distribution</h3>
          <div class="distribution-list">
            {#each Object.entries(metrics.rootTypeDistribution) as [type, count]}
              {@const percentage = serviceStatus.total_roots > 0 ? Math.round((count / serviceStatus.total_roots) * 100) : 0}
              <div class="distribution-item">
                <div class="distribution-info">
                  <span class="distribution-type">{rootsApi.getRootTypeDisplayName(type)}</span>
                  <span class="distribution-count">{count}</span>
                </div>
                <div class="distribution-bar">
                  <div class="distribution-fill" style="width: {percentage}%"></div>
                </div>
                <div class="distribution-percentage">{percentage}%</div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Permission Distribution -->
        <div class="distribution-card">
          <h3 class="distribution-title">Permission Distribution</h3>
          <div class="distribution-list">
            {#each Object.entries(metrics.permissionDistribution) as [permission, count]}
              {@const percentage = serviceStatus.total_roots > 0 ? Math.round((count / serviceStatus.total_roots) * 100) : 0}
              <div class="distribution-item">
                <div class="distribution-info">
                  <span class="distribution-type">{rootsApi.getPermissionDisplayName(permission)}</span>
                  <span class="distribution-count">{count}</span>
                </div>
                <div class="distribution-bar">
                  <div class="distribution-fill" style="width: {percentage}%"></div>
                </div>
                <div class="distribution-percentage">{percentage}%</div>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {:else}
    <!-- Loading or Error State -->
    <div class="empty-state">
      <div class="empty-icon">📊</div>
      <h3>No Service Status Available</h3>
      <p>Unable to load service monitoring data</p>
      <button class="refresh-btn" on:click={handleRefresh}>
        Try Again
      </button>
    </div>
  {/if}
</div>

<style>
  .status-monitor {
    background: white;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    overflow: hidden;
  }

  .monitor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .header-left {
    flex: 1;
  }

  .monitor-title {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
    color: #1a1a1a;
  }

  .monitor-description {
    color: #6b7280;
    margin: 0;
    font-size: 0.875rem;
  }

  .refresh-btn {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.2s ease;
  }

  .refresh-btn:hover {
    background: #2563eb;
  }

  .health-overview {
    padding: 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
  }

  .health-card {
    background: white;
    border: 2px solid;
    border-radius: 8px;
    padding: 1.5rem;
  }

  .health-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .health-icon {
    font-size: 1.5rem;
  }

  .health-status {
    font-size: 1.125rem;
    font-weight: 600;
  }

  .health-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .detail-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .detail-label {
    color: #6b7280;
    font-size: 0.875rem;
  }

  .detail-value {
    font-weight: 500;
    color: #374151;
  }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1.5rem;
    padding: 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .metric-card {
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1.5rem;
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .metric-icon {
    font-size: 2rem;
    flex-shrink: 0;
  }

  .metric-content {
    flex: 1;
  }

  .metric-value {
    font-size: 1.875rem;
    font-weight: 700;
    color: #1a1a1a;
    line-height: 1;
  }

  .metric-label {
    font-size: 0.875rem;
    color: #6b7280;
    margin-top: 0.25rem;
  }

  .metric-percentage {
    font-size: 0.75rem;
    color: #3b82f6;
    font-weight: 500;
    margin-top: 0.125rem;
  }

  .distribution-section {
    padding: 1.5rem;
  }

  .distribution-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
  }

  .distribution-card {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    overflow: hidden;
  }

  .distribution-title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
    padding: 1rem 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
    color: #374151;
  }

  .distribution-list {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .distribution-item {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 1rem;
    align-items: center;
  }

  .distribution-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .distribution-type {
    font-weight: 500;
    color: #374151;
    font-size: 0.875rem;
  }

  .distribution-count {
    color: #6b7280;
    font-size: 0.875rem;
  }

  .distribution-bar {
    height: 6px;
    background: #e5e7eb;
    border-radius: 3px;
    overflow: hidden;
    grid-column: 1 / -1;
    margin: 0.25rem 0;
  }

  .distribution-fill {
    height: 100%;
    background: #3b82f6;
    transition: width 0.3s ease;
  }

  .distribution-percentage {
    color: #3b82f6;
    font-weight: 500;
    font-size: 0.875rem;
    text-align: right;
  }

  .empty-state {
    text-align: center;
    padding: 4rem 2rem;
    color: #6b7280;
  }

  .empty-icon {
    font-size: 4rem;
    margin-bottom: 1.5rem;
  }

  .empty-state h3 {
    margin-bottom: 0.5rem;
    color: #374151;
  }

  .empty-state p {
    margin-bottom: 2rem;
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .monitor-header {
      flex-direction: column;
      gap: 1rem;
      align-items: stretch;
    }

    .metrics-grid {
      grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      gap: 1rem;
    }

    .metric-card {
      padding: 1rem;
    }

    .metric-icon {
      font-size: 1.5rem;
    }

    .metric-value {
      font-size: 1.5rem;
    }

    .distribution-grid {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }

    .health-details {
      grid-template-columns: 1fr;
    }
  }
</style>