<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { rootsApi, type Root, type Permission } from '$lib/api/roots';

  export let roots: Root[];

  const dispatch = createEventDispatcher();

  const permissions: Permission[] = ['read', 'write', 'execute', 'create', 'delete'];
  const rootTypes = ['filesystem', 'uri', 'database', 'api', 'cloud_storage', 'container', 'network_share', 'custom'];

  // Group roots by type
  $: rootsByType = roots.reduce((acc, root) => {
    if (!acc[root.root_type]) {
      acc[root.root_type] = [];
    }
    acc[root.root_type].push(root);
    return acc;
  }, {} as Record<string, Root[]>);

  // Calculate permission statistics
  $: permissionStats = permissions.map(permission => {
    const total = roots.length;
    const granted = roots.filter(root => root.permissions.includes(permission)).length;
    return {
      permission,
      granted,
      total,
      percentage: total > 0 ? Math.round((granted / total) * 100) : 0
    };
  });

  function getPermissionColor(permission: Permission): string {
    const colors: Record<Permission, string> = {
      read: '#3b82f6',
      write: '#f59e0b', 
      execute: '#10b981',
      create: '#8b5cf6',
      delete: '#ef4444',
    };
    return colors[permission];
  }

  function getRootTypeIcon(rootType: string): string {
    const icons: Record<string, string> = {
      filesystem: '💾',
      uri: '🌐',
      database: '🗄️',
      api: '🔌',
      cloud_storage: '☁️',
      container: '📦',
      network_share: '🌐',
      custom: '⚙️',
    };
    return icons[rootType] || '📁';
  }

  function hasPermission(root: Root, permission: Permission): boolean {
    return root.permissions.includes(permission);
  }
</script>

<div class="permissions-matrix">
  <div class="matrix-header">
    <h2 class="matrix-title">👥 Permissions Matrix</h2>
    <p class="matrix-description">Overview of permissions across all roots</p>
  </div>

  <!-- Permission Statistics -->
  <div class="permission-stats">
    <h3 class="stats-title">Permission Coverage</h3>
    <div class="stats-grid">
      {#each permissionStats as stat}
        <div class="stat-card">
          <div class="stat-header">
            <span class="stat-name" style="color: {getPermissionColor(stat.permission)}">
              {rootsApi.getPermissionDisplayName(stat.permission)}
            </span>
            <span class="stat-percentage">{stat.percentage}%</span>
          </div>
          <div class="stat-bar">
            <div 
              class="stat-fill" 
              style="width: {stat.percentage}%; background: {getPermissionColor(stat.permission)}"
            ></div>
          </div>
          <div class="stat-details">
            {stat.granted} of {stat.total} roots
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Matrix Table -->
  <div class="matrix-table-container">
    <h3 class="table-title">Detailed Permissions by Root Type</h3>
    
    <div class="matrix-table">
      <div class="table-header">
        <div class="header-cell type-header">Root Type</div>
        {#each permissions as permission}
          <div class="header-cell permission-header" style="color: {getPermissionColor(permission)}">
            {rootsApi.getPermissionDisplayName(permission)}
          </div>
        {/each}
        <div class="header-cell count-header">Count</div>
      </div>

      {#each Object.entries(rootsByType) as [rootType, typeRoots]}
        <div class="table-row">
          <div class="type-cell">
            <span class="type-icon">{getRootTypeIcon(rootType)}</span>
            <span class="type-name">{rootsApi.getRootTypeDisplayName(rootType)}</span>
          </div>
          
          {#each permissions as permission}
            {@const grantedCount = typeRoots.filter(root => hasPermission(root, permission)).length}
            {@const percentage = Math.round((grantedCount / typeRoots.length) * 100)}
            <div class="permission-cell">
              <div class="permission-indicator" style="background: {getPermissionColor(permission)}; opacity: {percentage / 100}">
                {grantedCount}/{typeRoots.length}
              </div>
            </div>
          {/each}
          
          <div class="count-cell">
            {typeRoots.length}
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Individual Roots -->
  <div class="individual-roots">
    <h3 class="roots-title">Individual Root Permissions</h3>
    
    <div class="roots-list">
      {#each roots as root}
        <div class="root-item" class:accessible={root.accessible} class:inaccessible={!root.accessible}>
          <div class="root-info">
            <div class="root-header">
              <span class="root-icon">{getRootTypeIcon(root.root_type)}</span>
              <div class="root-details">
                <div class="root-path">{root.name || root.path}</div>
                <div class="root-type">{rootsApi.getRootTypeDisplayName(root.root_type)}</div>
                {#if !root.accessible}
                  <div class="inaccessible-warning">⚠️ Not accessible</div>
                {/if}
              </div>
            </div>
          </div>
          
          <div class="root-permissions">
            {#each permissions as permission}
              <div 
                class="permission-badge" 
                class:granted={hasPermission(root, permission)}
                class:denied={!hasPermission(root, permission)}
                style="border-color: {getPermissionColor(permission)}"
                title="{rootsApi.getPermissionDisplayName(permission)}: {hasPermission(root, permission) ? 'Granted' : 'Denied'}"
              >
                <span class="permission-icon">
                  {hasPermission(root, permission) ? '✓' : '✗'}
                </span>
                <span class="permission-label">{permission[0].toUpperCase()}</span>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="empty-state">
          <div class="empty-icon">👥</div>
          <h3>No Roots Found</h3>
          <p>Discover roots to see their permission matrix</p>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .permissions-matrix {
    background: white;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    overflow: hidden;
  }

  .matrix-header {
    padding: 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .matrix-title {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
    color: #1a1a1a;
  }

  .matrix-description {
    color: #6b7280;
    margin: 0;
    font-size: 0.875rem;
  }

  .permission-stats {
    padding: 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
  }

  .stats-title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 1rem 0;
    color: #374151;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .stat-card {
    background: white;
    padding: 1rem;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
  }

  .stat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .stat-name {
    font-weight: 500;
    font-size: 0.875rem;
  }

  .stat-percentage {
    font-weight: 600;
    color: #1f2937;
  }

  .stat-bar {
    height: 4px;
    background: #e5e7eb;
    border-radius: 2px;
    margin-bottom: 0.5rem;
    overflow: hidden;
  }

  .stat-fill {
    height: 100%;
    transition: width 0.3s ease;
  }

  .stat-details {
    font-size: 0.75rem;
    color: #6b7280;
  }

  .matrix-table-container {
    padding: 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .table-title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 1rem 0;
    color: #374151;
  }

  .matrix-table {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    overflow: hidden;
  }

  .table-header {
    display: grid;
    grid-template-columns: 200px repeat(5, 1fr) 80px;
    background: #f3f4f6;
  }

  .table-row {
    display: grid;
    grid-template-columns: 200px repeat(5, 1fr) 80px;
    border-top: 1px solid #e5e7eb;
  }

  .header-cell {
    padding: 0.75rem;
    font-weight: 600;
    font-size: 0.875rem;
    border-right: 1px solid #e5e7eb;
  }

  .type-cell {
    padding: 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-right: 1px solid #e5e7eb;
  }

  .type-icon {
    font-size: 1.125rem;
  }

  .type-name {
    font-weight: 500;
    font-size: 0.875rem;
  }

  .permission-cell {
    padding: 0.75rem;
    display: flex;
    justify-content: center;
    align-items: center;
    border-right: 1px solid #e5e7eb;
  }

  .permission-indicator {
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    color: white;
    font-size: 0.75rem;
    font-weight: 500;
    text-align: center;
    min-width: 40px;
  }

  .count-cell {
    padding: 0.75rem;
    text-align: center;
    font-weight: 500;
    color: #374151;
  }

  .individual-roots {
    padding: 1.5rem;
  }

  .roots-title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 1rem 0;
    color: #374151;
  }

  .roots-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .root-item {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .root-item.inaccessible {
    background: #fef2f2;
    border-color: #fecaca;
  }

  .root-info {
    flex: 1;
    min-width: 0;
  }

  .root-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .root-icon {
    font-size: 1.25rem;
    flex-shrink: 0;
  }

  .root-details {
    min-width: 0;
  }

  .root-path {
    font-weight: 500;
    color: #1a1a1a;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .root-type {
    font-size: 0.875rem;
    color: #6b7280;
  }

  .inaccessible-warning {
    font-size: 0.75rem;
    color: #dc2626;
    font-weight: 500;
  }

  .root-permissions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .permission-badge {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .permission-badge.granted {
    background: rgba(16, 185, 129, 0.1);
    color: #059669;
    border-color: #10b981;
  }

  .permission-badge.denied {
    background: rgba(239, 68, 68, 0.1);
    color: #dc2626;
    border-color: #ef4444;
  }

  .permission-icon {
    font-size: 0.75rem;
  }

  .permission-label {
    font-weight: 600;
  }

  .empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: #6b7280;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .empty-state h3 {
    margin-bottom: 0.5rem;
    color: #374151;
  }

  /* Responsive design */
  @media (max-width: 1024px) {
    .table-header,
    .table-row {
      grid-template-columns: 150px repeat(5, minmax(60px, 1fr)) 60px;
    }

    .header-cell,
    .type-cell,
    .permission-cell,
    .count-cell {
      padding: 0.5rem 0.25rem;
      font-size: 0.75rem;
    }

    .permission-indicator {
      font-size: 0.625rem;
      min-width: 30px;
    }
  }

  @media (max-width: 768px) {
    .root-item {
      flex-direction: column;
      align-items: stretch;
      gap: 1rem;
    }

    .root-permissions {
      justify-content: center;
      flex-wrap: wrap;
    }

    .matrix-table-container {
      overflow-x: auto;
    }

    .matrix-table {
      min-width: 600px;
    }

    .stats-grid {
      grid-template-columns: 1fr;
    }
  }
</style>