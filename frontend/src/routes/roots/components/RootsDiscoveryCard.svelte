<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { rootsApi, type Root, type PaginatedRootsResponse } from '$lib/api/roots';

  export let rootsData: PaginatedRootsResponse | null;

  const dispatch = createEventDispatcher();

  // Local state
  let selectedFilter = 'all';
  let searchQuery = '';
  let viewMode = 'grid'; // 'grid' or 'list'
  let sortBy = 'path'; // 'path', 'type', 'discovered_at', 'accessible'
  let sortOrder = 'asc'; // 'asc' or 'desc'

  // Filter options
  const filterOptions = [
    { value: 'all', label: 'All Roots', icon: '📁' },
    { value: 'filesystem', label: 'File System', icon: '💾' },
    { value: 'uri', label: 'URI/URL', icon: '🌐' },
    { value: 'database', label: 'Database', icon: '🗄️' },
    { value: 'api', label: 'API Endpoint', icon: '🔌' },
    { value: 'cloud_storage', label: 'Cloud Storage', icon: '☁️' },
  ];

  // Sort options
  const sortOptions = [
    { value: 'path', label: 'Path' },
    { value: 'type', label: 'Type' },
    { value: 'discovered_at', label: 'Discovery Date' },
    { value: 'accessible', label: 'Accessibility' },
  ];

  // Computed filtered and sorted roots
  $: filteredRoots = rootsData?.roots ? filterAndSortRoots(rootsData.roots) : [];

  function filterAndSortRoots(roots: Root[]): Root[] {
    let filtered = roots;

    // Apply type filter
    if (selectedFilter !== 'all') {
      filtered = filtered.filter(root => root.root_type === selectedFilter);
    }

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(root => 
        root.path.toLowerCase().includes(query) ||
        (root.name && root.name.toLowerCase().includes(query)) ||
        root.root_type.toLowerCase().includes(query)
      );
    }

    // Apply sorting
    filtered.sort((a, b) => {
      let aValue: any, bValue: any;
      
      switch (sortBy) {
        case 'path':
          aValue = a.path.toLowerCase();
          bValue = b.path.toLowerCase();
          break;
        case 'type':
          aValue = a.root_type;
          bValue = b.root_type;
          break;
        case 'discovered_at':
          aValue = new Date(a.discovered_at);
          bValue = new Date(b.discovered_at);
          break;
        case 'accessible':
          aValue = a.accessible ? 1 : 0;
          bValue = b.accessible ? 1 : 0;
          break;
        default:
          return 0;
      }

      if (aValue < bValue) return sortOrder === 'asc' ? -1 : 1;
      if (aValue > bValue) return sortOrder === 'asc' ? 1 : -1;
      return 0;
    });

    return filtered;
  }

  function toggleSort(newSortBy: string) {
    if (sortBy === newSortBy) {
      sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = newSortBy;
      sortOrder = 'asc';
    }
  }

  function handleRefresh() {
    dispatch('refresh');
  }

  function handleDiscover() {
    dispatch('discover');
  }

  function getRootIcon(rootType: string): string {
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

  function formatDiscoveryDate(dateString: string): string {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays < 7) return `${diffDays}d ago`;
    return date.toLocaleDateString();
  }

  function getPermissionBadgeColor(permission: string): string {
    const colors: Record<string, string> = {
      read: 'blue',
      write: 'orange', 
      execute: 'green',
      create: 'purple',
      delete: 'red',
    };
    return colors[permission] || 'gray';
  }
</script>

<div class="discovery-card">
  <!-- Header -->
  <div class="card-header">
    <div class="header-left">
      <h2 class="card-title">
        🔍 Root Discovery
      </h2>
      <p class="card-description">
        {filteredRoots.length} of {rootsData?.total_count || 0} roots
      </p>
    </div>
    
    <div class="header-actions">
      <button class="view-toggle" class:active={viewMode === 'grid'} on:click={() => viewMode = 'grid'}>
        <span>⊞</span>
      </button>
      <button class="view-toggle" class:active={viewMode === 'list'} on:click={() => viewMode = 'list'}>
        <span>☰</span>
      </button>
    </div>
  </div>

  <!-- Filters and Search -->
  <div class="filters-section">
    <div class="search-box">
      <span class="search-icon">🔍</span>
      <input
        type="text"
        placeholder="Search roots by path, name, or type..."
        bind:value={searchQuery}
        class="search-input"
      />
      {#if searchQuery}
        <button class="clear-search" on:click={() => searchQuery = ''}>
          ✕
        </button>
      {/if}
    </div>

    <div class="filter-tabs">
      {#each filterOptions as option}
        <button
          class="filter-tab"
          class:active={selectedFilter === option.value}
          on:click={() => selectedFilter = option.value}
        >
          <span class="filter-icon">{option.icon}</span>
          <span class="filter-label">{option.label}</span>
        </button>
      {/each}
    </div>

    <div class="sort-controls">
      <label class="sort-label">Sort by:</label>
      <select bind:value={sortBy} class="sort-select">
        {#each sortOptions as option}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
      <button class="sort-order-btn" on:click={() => toggleSort(sortBy)}>
        {sortOrder === 'asc' ? '↑' : '↓'}
      </button>
    </div>
  </div>

  <!-- Content -->
  <div class="content-area">
    {#if filteredRoots.length === 0}
      <div class="empty-state">
        {#if searchQuery || selectedFilter !== 'all'}
          <div class="empty-icon">🔍</div>
          <h3>No matching roots found</h3>
          <p>Try adjusting your filters or search query</p>
          <button class="clear-filters-btn" on:click={() => { searchQuery = ''; selectedFilter = 'all'; }}>
            Clear Filters
          </button>
        {:else}
          <div class="empty-icon">📁</div>
          <h3>No roots discovered yet</h3>
          <p>Click "Discover Roots" to scan for accessible filesystem and URI boundaries</p>
          <button class="discover-btn" on:click={handleDiscover}>
            🔍 Discover Roots
          </button>
        {/if}
      </div>
    {:else}
      <div class="roots-container" class:grid-view={viewMode === 'grid'} class:list-view={viewMode === 'list'}>
        {#each filteredRoots as root (root.id)}
          <div class="root-item" class:accessible={root.accessible} class:inaccessible={!root.accessible}>
            <div class="root-header">
              <div class="root-icon">
                {getRootIcon(root.root_type)}
              </div>
              <div class="root-info">
                <div class="root-path" title={root.path}>
                  {root.name || root.path}
                </div>
                <div class="root-type">
                  {rootsApi.getRootTypeDisplayName(root.root_type)}
                </div>
              </div>
              <div class="root-status">
                <span class="accessibility-badge" class:accessible={root.accessible} class:inaccessible={!root.accessible}>
                  {root.accessible ? '✅' : '❌'}
                </span>
              </div>
            </div>

            <div class="root-details">
              <div class="root-path-full" title={root.path}>
                {root.path}
              </div>
              
              {#if root.permissions.length > 0}
                <div class="permissions">
                  {#each root.permissions as permission}
                    <span class="permission-badge {getPermissionBadgeColor(permission)}">
                      {rootsApi.getPermissionDisplayName(permission)}
                    </span>
                  {/each}
                </div>
              {/if}

              <div class="root-metadata">
                <div class="metadata-item">
                  <span class="metadata-label">Discovered:</span>
                  <span class="metadata-value">{formatDiscoveryDate(root.discovered_at)}</span>
                </div>
                
                {#if root.manual}
                  <div class="metadata-item">
                    <span class="manual-badge">Manual</span>
                  </div>
                {/if}

                {#if Object.keys(root.metadata).length > 0}
                  {#each Object.entries(root.metadata) as [key, value]}
                    <div class="metadata-item">
                      <span class="metadata-label">{key}:</span>
                      <span class="metadata-value">{value}</span>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>

      <!-- Pagination (if needed) -->
      {#if rootsData?.has_more}
        <div class="pagination">
          <button class="load-more-btn" on:click={handleRefresh}>
            Load More Roots
          </button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .discovery-card {
    background: white;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    overflow: hidden;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .header-left {
    flex: 1;
  }

  .card-title {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
    color: #1a1a1a;
  }

  .card-description {
    color: #6b7280;
    margin: 0;
    font-size: 0.875rem;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .view-toggle {
    padding: 0.5rem;
    background: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .view-toggle.active {
    background: #3b82f6;
    color: white;
    border-color: #3b82f6;
  }

  .filters-section {
    padding: 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 1rem;
    color: #9ca3af;
  }

  .search-input {
    width: 100%;
    padding: 0.75rem 1rem 0.75rem 2.5rem;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 0.875rem;
  }

  .clear-search {
    position: absolute;
    right: 1rem;
    background: none;
    border: none;
    color: #9ca3af;
    cursor: pointer;
    padding: 0.25rem;
  }

  .filter-tabs {
    display: flex;
    gap: 0.5rem;
    overflow-x: auto;
    padding-bottom: 0.25rem;
  }

  .filter-tab {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    background: white;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s ease;
    white-space: nowrap;
    font-size: 0.875rem;
  }

  .filter-tab.active {
    background: #3b82f6;
    color: white;
    border-color: #3b82f6;
  }

  .sort-controls {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
  }

  .sort-label {
    color: #6b7280;
  }

  .sort-select {
    padding: 0.25rem 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    font-size: 0.875rem;
  }

  .sort-order-btn {
    padding: 0.25rem 0.5rem;
    background: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .content-area {
    padding: 1.5rem;
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

  .empty-state p {
    margin-bottom: 2rem;
  }

  .clear-filters-btn,
  .discover-btn {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
  }

  .roots-container {
    display: grid;
    gap: 1rem;
  }

  .roots-container.grid-view {
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  }

  .roots-container.list-view {
    grid-template-columns: 1fr;
  }

  .root-item {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
    transition: all 0.2s ease;
  }

  .root-item:hover {
    border-color: #3b82f6;
    box-shadow: 0 2px 8px rgba(59, 130, 246, 0.1);
  }

  .root-item.inaccessible {
    background: #fef2f2;
    border-color: #fecaca;
  }

  .root-header {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .root-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .root-info {
    flex: 1;
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

  .accessibility-badge {
    font-size: 1rem;
  }

  .root-details {
    space-y: 0.75rem;
  }

  .root-path-full {
    font-family: monospace;
    font-size: 0.875rem;
    color: #4b5563;
    background: #f3f4f6;
    padding: 0.5rem;
    border-radius: 4px;
    overflow-wrap: break-word;
    margin-bottom: 0.75rem;
  }

  .permissions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-bottom: 0.75rem;
  }

  .permission-badge {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-weight: 500;
  }

  .permission-badge.blue { background: #dbeafe; color: #1e40af; }
  .permission-badge.orange { background: #fed7aa; color: #c2410c; }
  .permission-badge.green { background: #dcfce7; color: #166534; }
  .permission-badge.purple { background: #e9d5ff; color: #7c3aed; }
  .permission-badge.red { background: #fecaca; color: #dc2626; }
  .permission-badge.gray { background: #f3f4f6; color: #4b5563; }

  .root-metadata {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    font-size: 0.875rem;
  }

  .metadata-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .metadata-label {
    color: #6b7280;
  }

  .metadata-value {
    color: #374151;
  }

  .manual-badge {
    background: #f3e8ff;
    color: #7c3aed;
    padding: 0.125rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .pagination {
    margin-top: 2rem;
    text-align: center;
  }

  .load-more-btn {
    padding: 0.75rem 1.5rem;
    background: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
  }

  .load-more-btn:hover {
    background: #e5e7eb;
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .roots-container.grid-view {
      grid-template-columns: 1fr;
    }

    .filters-section {
      padding: 1rem;
    }

    .filter-tabs {
      gap: 0.25rem;
    }

    .filter-tab {
      padding: 0.375rem 0.75rem;
      font-size: 0.8125rem;
    }

    .sort-controls {
      flex-wrap: wrap;
    }
  }
</style>