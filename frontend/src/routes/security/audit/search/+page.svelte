<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { securityApi } from '$lib/api/security';
  import AuditEntryCard from '$lib/components/security/AuditEntryCard.svelte';
  import type { AuditEntry, AuditSearchFilters } from '$lib/types/security';
  
  // State management
  let entries: AuditEntry[] = [];
  let loading = false;
  let error = '';
  let totalCount = 0;
  let hasMore = false;
  
  // Search and filters
  let searchQuery = '';
  let filters: AuditSearchFilters = {
    eventTypes: [],
    severities: [],
    userIds: [],
    sourceIps: [],
    dateRange: {
      start: '',
      end: ''
    },
    limit: 50,
    offset: 0
  };
  
  // UI state
  let showAdvancedFilters = false;
  let autoRefresh = false;
  let refreshInterval: number | null = null;
  let selectedEntries = new Set<string>();
  let showBulkActions = false;
  
  // Available filter options
  let availableEventTypes: string[] = [];
  let availableUsers: string[] = [];
  let availableSeverities = ['critical', 'high', 'medium', 'low', 'info'];
  
  // Pagination
  let currentPage = 1;
  let itemsPerPage = 50;
  
  // Quick date range presets
  const dateRangePresets = [
    { label: 'Last Hour', hours: 1 },
    { label: 'Last 24 Hours', hours: 24 },
    { label: 'Last 7 Days', hours: 168 },
    { label: 'Last 30 Days', hours: 720 }
  ];
  
  // Load search data
  async function loadSearchData(append = false) {
    try {
      if (!append) {
        loading = true;
        entries = [];
      }
      error = '';
      
      // Build search parameters
      const searchParams = {
        query: searchQuery.trim() || undefined,
        ...filters,
        offset: append ? entries.length : 0
      };
      
      const result = await securityApi.searchAuditLogs(searchParams);
      
      if (append) {
        entries = [...entries, ...(result.entries || [])];
      } else {
        entries = result.entries || [];
      }
      
      totalCount = result.total || 0;
      hasMore = result.hasMore || false;
      
      // Load filter options if not loaded
      if (availableEventTypes.length === 0) {
        await loadFilterOptions();
      }
    } catch (err) {
      console.error('Failed to load audit entries:', err);
      error = `Failed to load audit entries: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Load available filter options
  async function loadFilterOptions() {
    try {
      const [eventTypes, users] = await Promise.all([
        securityApi.getAuditEventTypes(),
        securityApi.getAuditUsers()
      ]);
      
      availableEventTypes = eventTypes;
      availableUsers = users;
    } catch (err) {
      console.error('Failed to load filter options:', err);
    }
  }
  
  // Search with debouncing
  let searchTimeout: number | null = null;
  function debouncedSearch() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      currentPage = 1;
      filters.offset = 0;
      loadSearchData();
    }, 300);
  }
  
  // Filter management
  function toggleFilter(filterType: keyof AuditSearchFilters, value: string) {
    const filterArray = filters[filterType] as string[];
    if (filterArray.includes(value)) {
      filters[filterType] = filterArray.filter(v => v !== value);
    } else {
      filters[filterType] = [...filterArray, value];
    }
    applyFilters();
  }
  
  function applyFilters() {
    currentPage = 1;
    filters.offset = 0;
    loadSearchData();
  }
  
  function clearFilters() {
    filters = {
      eventTypes: [],
      severities: [],
      userIds: [],
      sourceIps: [],
      dateRange: { start: '', end: '' },
      limit: itemsPerPage,
      offset: 0
    };
    searchQuery = '';
    loadSearchData();
  }
  
  function setDateRange(hours: number) {
    const end = new Date();
    const start = new Date(end.getTime() - hours * 60 * 60 * 1000);
    
    filters.dateRange.start = start.toISOString().slice(0, 16);
    filters.dateRange.end = end.toISOString().slice(0, 16);
    applyFilters();
  }
  
  // Load more entries (infinite scroll)
  function loadMore() {
    if (!loading && hasMore) {
      loadSearchData(true);
    }
  }
  
  // Auto-refresh functionality
  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    
    if (autoRefresh) {
      refreshInterval = setInterval(() => {
        loadSearchData();
      }, 30000);
    } else if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }
  
  // Entry selection management
  function toggleEntrySelection(entryId: string) {
    const newSelected = new Set(selectedEntries);
    if (newSelected.has(entryId)) {
      newSelected.delete(entryId);
    } else {
      newSelected.add(entryId);
    }
    selectedEntries = newSelected;
    showBulkActions = selectedEntries.size > 0;
  }
  
  function selectAllVisible() {
    selectedEntries = new Set(entries.map(e => e.id));
    showBulkActions = true;
  }
  
  function clearSelection() {
    selectedEntries = new Set();
    showBulkActions = false;
  }
  
  // Bulk operations
  async function performBulkExport() {
    if (selectedEntries.size === 0) return;
    
    try {
      const result = await securityApi.exportAuditEntries({
        entryIds: Array.from(selectedEntries),
        format: 'csv'
      });
      
      // Trigger download
      const blob = new Blob([result.data], { type: 'text/csv' });
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `audit-entries-${new Date().toISOString().split('T')[0]}.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      window.URL.revokeObjectURL(url);
      
      clearSelection();
    } catch (err) {
      alert(`Failed to export entries: ${err}`);
    }
  }
  
  async function performBulkArchive() {
    if (selectedEntries.size === 0) return;
    
    if (!confirm(`Archive ${selectedEntries.size} selected entries?`)) return;
    
    try {
      await securityApi.bulkArchiveAuditEntries(Array.from(selectedEntries));
      loadSearchData(); // Reload to remove archived entries
      clearSelection();
    } catch (err) {
      alert(`Failed to archive entries: ${err}`);
    }
  }
  
  // Initialize from URL parameters
  function initializeFromUrl() {
    const urlParams = new URLSearchParams(window.location.search);
    
    // Set search query from URL
    const query = urlParams.get('q') || urlParams.get('query');
    if (query) {
      searchQuery = query;
    }
    
    // Set specific entry ID
    const entryId = urlParams.get('id');
    if (entryId) {
      searchQuery = `id:${entryId}`;
    }
    
    // Set filters from URL
    const eventTypes = urlParams.get('eventTypes');
    if (eventTypes) {
      filters.eventTypes = eventTypes.split(',');
    }
    
    const severities = urlParams.get('severities');
    if (severities) {
      filters.severities = severities.split(',');
    }
    
    const users = urlParams.get('users');
    if (users) {
      filters.userIds = users.split(',');
    }
  }
  
  onMount(() => {
    initializeFromUrl();
    loadSearchData();
    
    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
      }
    };
  });
  
  // Watch for search changes
  $: if (searchQuery !== undefined) {
    debouncedSearch();
  }
</script>

<div class="space-y-6">
  <!-- Header Section -->
  <div class="security-card">
    <div class="security-card-header">
      <div>
        <h2 class="security-card-title">Audit Log Search</h2>
        <p class="text-sm text-gray-600 mt-1">
          Search and filter audit entries with advanced criteria
        </p>
      </div>
      
      <div class="flex items-center gap-3">
        <button
          class="btn-sm {autoRefresh ? 'btn-primary' : 'btn-secondary'}"
          on:click={toggleAutoRefresh}
        >
          {autoRefresh ? '⏸️ Stop' : '🔄 Auto'} Refresh
        </button>
        
        <button
          class="btn-sm btn-secondary"
          on:click={() => showAdvancedFilters = !showAdvancedFilters}
        >
          {showAdvancedFilters ? '📊 Simple' : '⚙️ Advanced'} Filters
        </button>
        
        <button class="btn-primary" on:click={loadSearchData}>
          🔍 Search
        </button>
      </div>
    </div>
    
    <!-- Search Results Summary -->
    <div class="flex items-center justify-between text-sm text-gray-600">
      <div>
        {#if loading}
          <span>Searching...</span>
        {:else if totalCount > 0}
          <span>Found {totalCount.toLocaleString()} entries</span>
          {#if hasMore}
            <span>• Showing {entries.length.toLocaleString()}</span>
          {/if}
        {:else if searchQuery || filters.eventTypes.length > 0 || filters.severities.length > 0}
          <span>No entries match your search criteria</span>
        {:else}
          <span>Enter search criteria to find audit entries</span>
        {/if}
      </div>
      
      <div class="text-xs">
        {#if autoRefresh}
          <span class="text-green-600">🔄 Auto-refreshing every 30s</span>
        {/if}
      </div>
    </div>
  </div>

  <!-- Search and Basic Filters -->
  <div class="security-card">
    <div class="space-y-4">
      <!-- Main Search Bar -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Search Query</label>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search by message, user, IP address, event type... (e.g., user:admin, ip:192.168.1.1, type:auth_failure)"
          class="w-full px-4 py-3 border border-gray-300 rounded-md text-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
        <p class="text-xs text-gray-500 mt-1">
          Use prefixes: user:, ip:, type:, severity:, message: • Supports wildcards: * and ?
        </p>
      </div>
      
      <!-- Quick Date Range -->
      <div class="flex items-center gap-2 flex-wrap">
        <span class="text-sm text-gray-700 mr-2">Quick ranges:</span>
        {#each dateRangePresets as preset}
          <button
            class="btn-xs btn-secondary"
            on:click={() => setDateRange(preset.hours)}
          >
            {preset.label}
          </button>
        {/each}
        
        <button class="btn-xs btn-secondary" on:click={clearFilters}>
          🗑️ Clear All
        </button>
      </div>
    </div>
  </div>

  <!-- Advanced Filters -->
  {#if showAdvancedFilters}
    <div class="security-card">
      <h3 class="text-lg font-medium text-gray-900 mb-4">Advanced Filters</h3>
      
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <!-- Event Types -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Event Types</label>
          <div class="space-y-2 max-h-32 overflow-y-auto">
            {#each availableEventTypes as eventType}
              <label class="flex items-center">
                <input
                  type="checkbox"
                  checked={filters.eventTypes.includes(eventType)}
                  on:change={() => toggleFilter('eventTypes', eventType)}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <span class="ml-2 text-sm text-gray-700">{eventType}</span>
              </label>
            {/each}
          </div>
        </div>
        
        <!-- Severities -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Severities</label>
          <div class="space-y-2">
            {#each availableSeverities as severity}
              <label class="flex items-center">
                <input
                  type="checkbox"
                  checked={filters.severities.includes(severity)}
                  on:change={() => toggleFilter('severities', severity)}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <span class="ml-2 text-sm text-gray-700 capitalize">{severity}</span>
              </label>
            {/each}
          </div>
        </div>
        
        <!-- Date Range -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Custom Date Range</label>
          <div class="space-y-2">
            <input
              type="datetime-local"
              bind:value={filters.dateRange.start}
              class="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
            <input
              type="datetime-local"
              bind:value={filters.dateRange.end}
              class="w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
        </div>
      </div>
      
      <div class="flex items-center justify-end gap-3 mt-6">
        <button class="btn-secondary" on:click={clearFilters}>
          Clear Filters
        </button>
        <button class="btn-primary" on:click={applyFilters}>
          Apply Filters
        </button>
      </div>
    </div>
  {/if}

  <!-- Bulk Actions Bar -->
  {#if showBulkActions}
    <div class="security-card bg-blue-50 border-blue-200">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <span class="text-sm font-medium text-blue-900">
            {selectedEntries.size} entries selected
          </span>
          
          <div class="flex items-center gap-2">
            <button
              class="text-sm text-blue-700 hover:text-blue-900 underline"
              on:click={selectAllVisible}
            >
              Select All Visible ({entries.length})
            </button>
            
            <span class="text-blue-500">|</span>
            
            <button
              class="text-sm text-blue-700 hover:text-blue-900 underline"
              on:click={clearSelection}
            >
              Clear Selection
            </button>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <button
            class="btn-sm btn-secondary"
            on:click={performBulkExport}
          >
            📤 Export Selected
          </button>
          
          <button
            class="btn-sm btn-secondary"
            on:click={performBulkArchive}
          >
            📦 Archive Selected
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Search Results -->
  <div class="space-y-4">
    {#if loading && entries.length === 0}
      <div class="security-card">
        <div class="flex items-center justify-center py-12">
          <div class="flex items-center gap-3 text-gray-600">
            <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
            <span>Searching audit entries...</span>
          </div>
        </div>
      </div>
    {:else if error}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-red-600 mb-4">❌</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">Search Error</h3>
          <p class="text-gray-600 mb-4">{error}</p>
          <button class="btn-primary" on:click={loadSearchData}>
            🔄 Retry Search
          </button>
        </div>
      </div>
    {:else if entries.length === 0 && !loading}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-gray-400 mb-4 text-4xl">🔍</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">
            {searchQuery || filters.eventTypes.length > 0 || filters.severities.length > 0
              ? 'No Matching Entries'
              : 'Start Your Search'}
          </h3>
          <p class="text-gray-600 mb-4">
            {searchQuery || filters.eventTypes.length > 0 || filters.severities.length > 0
              ? 'Try adjusting your search criteria or filters'
              : 'Enter a search query or apply filters to find audit entries'}
          </p>
          {#if searchQuery || filters.eventTypes.length > 0 || filters.severities.length > 0}
            <button class="btn-primary" on:click={clearFilters}>
              🗑️ Clear Search
            </button>
          {/if}
        </div>
      </div>
    {:else}
      <!-- Entry Results -->
      <div class="space-y-3">
        {#each entries as entry}
          <AuditEntryCard
            {entry}
            selected={selectedEntries.has(entry.id)}
            searchQuery={searchQuery}
            on:select={() => toggleEntrySelection(entry.id)}
          />
        {/each}
      </div>

      <!-- Load More / Pagination -->
      {#if hasMore}
        <div class="security-card">
          <div class="text-center py-6">
            <button
              class="btn-primary"
              on:click={loadMore}
              disabled={loading}
            >
              {#if loading}
                <span class="flex items-center gap-2">
                  <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                  Loading More...
                </span>
              {:else}
                📋 Load More Entries
              {/if}
            </button>
            
            <div class="text-sm text-gray-600 mt-2">
              Showing {entries.length} of {totalCount.toLocaleString()} entries
            </div>
          </div>
        </div>
      {:else if entries.length > 0}
        <div class="security-card">
          <div class="text-center py-4">
            <div class="text-sm text-gray-600">
              ✅ All {totalCount.toLocaleString()} matching entries loaded
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>