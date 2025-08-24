<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { securityApi } from '$lib/api/security';
  
  // State management
  let violations: any[] = [];
  let loading = true;
  let error = '';
  let selectedViolation: any = null;
  
  // Filters
  let filterSeverity: 'all' | 'critical' | 'high' | 'medium' | 'low' = 'all';
  let filterStatus: 'all' | 'open' | 'investigating' | 'resolved' | 'false_positive' = 'all';
  let filterTimeRange: '1h' | '24h' | '7d' | '30d' = '24h';
  let searchQuery = '';
  
  // UI state
  let showViolationDetails = false;
  let sortBy: 'timestamp' | 'severity' | 'policy_name' = 'timestamp';
  let sortDesc = true;
  
  // Pagination
  let currentPage = 1;
  let itemsPerPage = 25;
  let totalViolations = 0;
  let filteredCount = 0;
  
  // Auto-refresh
  let autoRefresh = false;
  let refreshInterval: number | null = null;
  
  onMount(async () => {
    await loadPolicyViolations();
  });
  
  async function loadPolicyViolations() {
    try {
      loading = true;
      error = '';
      
      const params: any = {
        limit: itemsPerPage,
        offset: (currentPage - 1) * itemsPerPage
      };
      
      if (filterSeverity !== 'all') {
        params.severity = filterSeverity;
      }
      if (filterStatus !== 'all') {
        params.status = filterStatus;
      }
      if (filterTimeRange !== '24h') {
        params.timeRange = filterTimeRange;
      }
      if (searchQuery.trim()) {
        params.search = searchQuery.trim();
      }
      
      const result = await securityApi.getPolicyViolations(params);
      violations = result.violations || [];
      totalViolations = result.total || 0;
      filteredCount = result.filtered || 0;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load policy violations';
      console.error('Error loading policy violations:', e);
      violations = [];
    } finally {
      loading = false;
    }
  }
  
  async function updateViolationStatus(violationId: string, newStatus: string) {
    try {
      await securityApi.updateViolationStatus(violationId, newStatus);
      await loadPolicyViolations();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to update violation status';
      console.error('Error updating violation status:', e);
    }
  }
  
  async function assignViolation(violationId: string, assignedTo: string) {
    try {
      await securityApi.assignViolation(violationId, assignedTo);
      await loadPolicyViolations();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to assign violation';
      console.error('Error assigning violation:', e);
    }
  }
  
  function viewViolationDetails(violation: any) {
    selectedViolation = violation;
    showViolationDetails = true;
  }
  
  function getSeverityBadgeColor(severity: string) {
    switch (severity?.toLowerCase()) {
      case 'critical': return 'bg-red-100 text-red-800 border-red-300';
      case 'high': return 'bg-orange-100 text-orange-800 border-orange-300';
      case 'medium': return 'bg-yellow-100 text-yellow-800 border-yellow-300';
      case 'low': return 'bg-blue-100 text-blue-800 border-blue-300';
      default: return 'bg-gray-100 text-gray-800 border-gray-300';
    }
  }
  
  function getStatusBadgeColor(status: string) {
    switch (status?.toLowerCase()) {
      case 'open': return 'bg-red-100 text-red-800 border-red-300';
      case 'investigating': return 'bg-yellow-100 text-yellow-800 border-yellow-300';
      case 'resolved': return 'bg-green-100 text-green-800 border-green-300';
      case 'false_positive': return 'bg-gray-100 text-gray-800 border-gray-300';
      default: return 'bg-blue-100 text-blue-800 border-blue-300';
    }
  }
  
  function formatTimestamp(timestamp: string) {
    try {
      return new Date(timestamp).toLocaleString();
    } catch {
      return timestamp || 'Unknown';
    }
  }
  
  function applyFilters() {
    currentPage = 1;
    loadPolicyViolations();
  }
  
  function nextPage() {
    if (currentPage * itemsPerPage < totalViolations) {
      currentPage++;
      loadPolicyViolations();
    }
  }
  
  function previousPage() {
    if (currentPage > 1) {
      currentPage--;
      loadPolicyViolations();
    }
  }
  
  // Auto-refresh toggle
  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    
    if (autoRefresh) {
      refreshInterval = setInterval(loadPolicyViolations, 30000);
    } else if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }
  
  // Cleanup on component destroy
  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  });
</script>

<svelte:head>
  <title>Policy Violations - Security Management</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <!-- Header -->
  <div class="bg-white border-b border-gray-200 mb-6">
    <div class="px-6 py-4">
      <div class="flex justify-between items-center">
        <div>
          <h1 class="text-2xl font-bold text-gray-900">Policy Violations</h1>
          <p class="text-gray-600 mt-1">Security policy violations detected by the Policy Engine</p>
        </div>
        <div class="flex items-center gap-3">
          <span class="text-sm text-gray-500">Alpha Feature</span>
          <div class="w-3 h-3 bg-orange-400 rounded-full" title="Alpha - Policy Engine"></div>
        </div>
      </div>
    </div>
  </div>

  <!-- Error Message -->
  {#if error}
    <div class="mx-6 mb-6 bg-red-50 border border-red-300 rounded-lg p-4">
      <div class="flex items-center">
        <div class="text-red-400 mr-3">⚠️</div>
        <div>
          <h3 class="text-sm font-medium text-red-800">Error Loading Violations</h3>
          <p class="text-red-700 text-sm mt-1">{error}</p>
        </div>
      </div>
    </div>
  {/if}

  <!-- Filters and Controls -->
  <div class="mx-6 mb-6 bg-white rounded-lg shadow-sm border border-gray-200 p-4">
    <div class="flex flex-wrap gap-4 items-center">
      <!-- Search -->
      <div class="flex-1 min-w-64">
        <input
          type="text"
          placeholder="Search violations by policy name, description..."
          bind:value={searchQuery}
          on:input={applyFilters}
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        />
      </div>
      
      <!-- Severity Filter -->
      <select 
        bind:value={filterSeverity}
        on:change={applyFilters}
        class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
      >
        <option value="all">All Severities</option>
        <option value="critical">Critical</option>
        <option value="high">High</option>
        <option value="medium">Medium</option>
        <option value="low">Low</option>
      </select>
      
      <!-- Status Filter -->
      <select 
        bind:value={filterStatus}
        on:change={applyFilters}
        class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
      >
        <option value="all">All Statuses</option>
        <option value="open">Open</option>
        <option value="investigating">Investigating</option>
        <option value="resolved">Resolved</option>
        <option value="false_positive">False Positive</option>
      </select>
      
      <!-- Time Range -->
      <select 
        bind:value={filterTimeRange}
        on:change={applyFilters}
        class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
      >
        <option value="1h">Last Hour</option>
        <option value="24h">Last 24 Hours</option>
        <option value="7d">Last 7 Days</option>
        <option value="30d">Last 30 Days</option>
      </select>
      
      <!-- Auto-refresh -->
      <button
        on:click={toggleAutoRefresh}
        class="px-3 py-2 border {autoRefresh ? 'border-green-300 bg-green-50 text-green-700' : 'border-gray-300'} rounded-lg hover:bg-gray-50 transition-colors"
      >
        {#if autoRefresh}
          🔄 Auto (30s)
        {:else}
          ⏸️ Manual
        {/if}
      </button>
      
      <!-- Refresh -->
      <button
        on:click={loadPolicyViolations}
        disabled={loading}
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 transition-colors"
      >
        {#if loading}
          ⏳ Loading...
        {:else}
          🔄 Refresh
        {/if}
      </button>
    </div>
    
    <!-- Results Summary -->
    <div class="mt-3 text-sm text-gray-600">
      {#if filteredCount !== totalViolations}
        Showing {filteredCount} filtered violations out of {totalViolations} total
      {:else}
        Showing {totalViolations} violations
      {/if}
    </div>
  </div>

  <!-- Violations List -->
  <div class="mx-6">
    {#if loading}
      <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
        <div class="text-gray-500">
          <div class="animate-spin w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full mx-auto mb-4"></div>
          Loading policy violations...
        </div>
      </div>
    {:else if violations.length === 0}
      <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-8 text-center">
        <div class="text-gray-500">
          <div class="text-4xl mb-4">🛡️</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">No Policy Violations</h3>
          <p>No policy violations found matching your criteria.</p>
          {#if filterSeverity !== 'all' || filterStatus !== 'all' || searchQuery.trim()}
            <button 
              on:click={() => { filterSeverity = 'all'; filterStatus = 'all'; searchQuery = ''; applyFilters(); }}
              class="mt-3 text-blue-600 hover:text-blue-700 underline"
            >
              Clear all filters
            </button>
          {/if}
        </div>
      </div>
    {:else}
      <div class="bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden">
        <!-- Table Header -->
        <div class="bg-gray-50 px-6 py-3 border-b border-gray-200">
          <div class="grid grid-cols-12 gap-4 items-center text-sm font-medium text-gray-700">
            <div class="col-span-3">Policy & Description</div>
            <div class="col-span-2">Severity</div>
            <div class="col-span-2">Status</div>
            <div class="col-span-2">Detected</div>
            <div class="col-span-2">Context</div>
            <div class="col-span-1">Actions</div>
          </div>
        </div>
        
        <!-- Violations -->
        <div class="divide-y divide-gray-200">
          {#each violations as violation}
            <div class="px-6 py-4 hover:bg-gray-50 transition-colors">
              <div class="grid grid-cols-12 gap-4 items-center">
                <!-- Policy Info -->
                <div class="col-span-3">
                  <div class="font-medium text-gray-900">{violation.policy_name}</div>
                  <div class="text-sm text-gray-600 mt-1">{violation.description}</div>
                  <div class="text-xs text-gray-500 mt-1">ID: {violation.policy_id}</div>
                </div>
                
                <!-- Severity -->
                <div class="col-span-2">
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border {getSeverityBadgeColor(violation.severity)}">
                    {violation.severity?.toUpperCase() || 'UNKNOWN'}
                  </span>
                </div>
                
                <!-- Status -->
                <div class="col-span-2">
                  <select
                    value={violation.status || 'open'}
                    on:change={(e) => updateViolationStatus(violation.id, e.target.value)}
                    class="text-xs px-2 py-1 border border-gray-300 rounded {getStatusBadgeColor(violation.status)}"
                  >
                    <option value="open">Open</option>
                    <option value="investigating">Investigating</option>
                    <option value="resolved">Resolved</option>
                    <option value="false_positive">False Positive</option>
                  </select>
                </div>
                
                <!-- Timestamp -->
                <div class="col-span-2">
                  <div class="text-sm text-gray-900">{formatTimestamp(violation.detected_at)}</div>
                </div>
                
                <!-- Context -->
                <div class="col-span-2">
                  {#if violation.context}
                    <div class="text-xs text-gray-600">
                      {#if violation.context.tool_name}
                        <div>Tool: {violation.context.tool_name}</div>
                      {/if}
                      {#if violation.context.user_id}
                        <div>User: {violation.context.user_id}</div>
                      {/if}
                    </div>
                  {:else}
                    <span class="text-xs text-gray-400">No context</span>
                  {/if}
                </div>
                
                <!-- Actions -->
                <div class="col-span-1">
                  <button
                    on:click={() => viewViolationDetails(violation)}
                    class="text-blue-600 hover:text-blue-700 text-sm"
                  >
                    View
                  </button>
                </div>
              </div>
            </div>
          {/each}
        </div>
        
        <!-- Pagination -->
        {#if totalViolations > itemsPerPage}
          <div class="bg-gray-50 px-6 py-3 border-t border-gray-200 flex justify-between items-center">
            <div class="text-sm text-gray-600">
              Page {currentPage} of {Math.ceil(totalViolations / itemsPerPage)}
            </div>
            <div class="flex gap-2">
              <button
                on:click={previousPage}
                disabled={currentPage <= 1}
                class="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-100 disabled:opacity-50"
              >
                Previous
              </button>
              <button
                on:click={nextPage}
                disabled={currentPage * itemsPerPage >= totalViolations}
                class="px-3 py-1 text-sm border border-gray-300 rounded hover:bg-gray-100 disabled:opacity-50"
              >
                Next
              </button>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<!-- Violation Details Modal -->
{#if showViolationDetails && selectedViolation}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
    <div class="bg-white rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-y-auto">
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-gray-200 flex justify-between items-center">
        <h2 class="text-xl font-semibold text-gray-900">Violation Details</h2>
        <button
          on:click={() => showViolationDetails = false}
          class="text-gray-400 hover:text-gray-600 text-2xl"
        >
          ×
        </button>
      </div>
      
      <!-- Modal Content -->
      <div class="px-6 py-4 space-y-6">
        <!-- Basic Info -->
        <div class="grid grid-cols-2 gap-6">
          <div>
            <h3 class="text-lg font-medium text-gray-900 mb-3">Violation Information</h3>
            <dl class="space-y-2">
              <div>
                <dt class="text-sm font-medium text-gray-500">Policy Name</dt>
                <dd class="text-sm text-gray-900">{selectedViolation.policy_name}</dd>
              </div>
              <div>
                <dt class="text-sm font-medium text-gray-500">Description</dt>
                <dd class="text-sm text-gray-900">{selectedViolation.description}</dd>
              </div>
              <div>
                <dt class="text-sm font-medium text-gray-500">Severity</dt>
                <dd>
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border {getSeverityBadgeColor(selectedViolation.severity)}">
                    {selectedViolation.severity?.toUpperCase() || 'UNKNOWN'}
                  </span>
                </dd>
              </div>
              <div>
                <dt class="text-sm font-medium text-gray-500">Detected At</dt>
                <dd class="text-sm text-gray-900">{formatTimestamp(selectedViolation.detected_at)}</dd>
              </div>
            </dl>
          </div>
          
          <div>
            <h3 class="text-lg font-medium text-gray-900 mb-3">Context Information</h3>
            {#if selectedViolation.context}
              <dl class="space-y-2">
                {#each Object.entries(selectedViolation.context) as [key, value]}
                  <div>
                    <dt class="text-sm font-medium text-gray-500 capitalize">{key.replace('_', ' ')}</dt>
                    <dd class="text-sm text-gray-900">{value}</dd>
                  </div>
                {/each}
              </dl>
            {:else}
              <p class="text-sm text-gray-500">No context information available</p>
            {/if}
          </div>
        </div>
        
        <!-- Actions -->
        <div class="border-t border-gray-200 pt-4">
          <h3 class="text-lg font-medium text-gray-900 mb-3">Actions</h3>
          <div class="flex gap-3">
            <select
              value={selectedViolation.status || 'open'}
              on:change={(e) => updateViolationStatus(selectedViolation.id, e.target.value)}
              class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
            >
              <option value="open">Open</option>
              <option value="investigating">Investigating</option>
              <option value="resolved">Resolved</option>
              <option value="false_positive">False Positive</option>
            </select>
            
            <input
              type="text"
              placeholder="Assign to user..."
              class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              on:keydown={(e) => {
                if (e.key === 'Enter' && e.target.value.trim()) {
                  assignViolation(selectedViolation.id, e.target.value.trim());
                  e.target.value = '';
                }
              }}
            />
          </div>
        </div>
      </div>
      
      <!-- Modal Footer -->
      <div class="px-6 py-4 border-t border-gray-200 flex justify-end">
        <button
          on:click={() => showViolationDetails = false}
          class="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .security-card {
    @apply bg-white border border-gray-200 rounded-lg shadow-sm;
  }
</style>