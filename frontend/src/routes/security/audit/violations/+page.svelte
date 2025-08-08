<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { SecurityViolation, ViolationStatistics, AuditEntry } from '$lib/types/security';
  
  // State management
  let violations: SecurityViolation[] = [];
  let violationStats: ViolationStatistics | null = null;
  let relatedEntries: AuditEntry[] = [];
  let loading = true;
  let error = '';
  let selectedViolation: SecurityViolation | null = null;
  
  // Filters
  let filterSeverity: 'all' | 'critical' | 'high' | 'medium' | 'low' = 'all';
  let filterStatus: 'all' | 'active' | 'investigating' | 'resolved' | 'false_positive' = 'all';
  let filterTimeRange: '1h' | '24h' | '7d' | '30d' = '24h';
  let searchQuery = '';
  
  // UI state
  let showStatistics = true;
  let autoRefresh = true;
  let refreshInterval: number | null = null;
  
  // Pagination
  let currentPage = 1;
  let itemsPerPage = 20;
  
  // Load violations data
  async function loadViolationsData() {
    try {
      loading = true;
      error = '';
      
      const [violationsData, statsData] = await Promise.all([
        securityApi.getSecurityViolations({
          severity: filterSeverity !== 'all' ? filterSeverity : undefined,
          status: filterStatus !== 'all' ? filterStatus : undefined,
          timeRange: filterTimeRange,
          search: searchQuery.trim() || undefined,
          limit: itemsPerPage,
          offset: (currentPage - 1) * itemsPerPage
        }),
        securityApi.getViolationStatistics(filterTimeRange)
      ]);
      
      violations = violationsData.violations || [];
      violationStats = statsData;
    } catch (err) {
      console.error('Failed to load violations data:', err);
      error = `Failed to load violations data: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Load related entries for a violation
  async function loadRelatedEntries(violationId: string) {
    try {
      const result = await securityApi.getViolationRelatedEntries(violationId);
      relatedEntries = result.entries || [];
    } catch (err) {
      console.error('Failed to load related entries:', err);
      relatedEntries = [];
    }
  }
  
  // Violation management
  async function updateViolationStatus(violationId: string, status: string, notes?: string) {
    try {
      await securityApi.updateViolationStatus(violationId, {
        status,
        notes,
        updatedBy: 'current_user' // Would come from auth context
      });
      
      await loadViolationsData();
    } catch (err) {
      alert(`Failed to update violation status: ${err}`);
    }
  }
  
  async function assignViolation(violationId: string, assigneeId: string) {
    try {
      await securityApi.assignViolation(violationId, {
        assigneeId,
        assignedBy: 'current_user'
      });
      
      await loadViolationsData();
    } catch (err) {
      alert(`Failed to assign violation: ${err}`);
    }
  }
  
  async function addViolationNote(violationId: string, note: string) {
    try {
      await securityApi.addViolationNote(violationId, {
        note,
        addedBy: 'current_user'
      });
      
      await loadViolationsData();
    } catch (err) {
      alert(`Failed to add note: ${err}`);
    }
  }
  
  // Get severity display properties
  function getSeverityProps(severity: string) {
    const severityProps = {
      'critical': { 
        color: 'bg-red-100 text-red-800 border-red-300', 
        icon: '🚨', 
        bgColor: 'bg-red-50',
        textColor: 'text-red-700'
      },
      'high': { 
        color: 'bg-orange-100 text-orange-800 border-orange-300', 
        icon: '⚠️', 
        bgColor: 'bg-orange-50',
        textColor: 'text-orange-700'
      },
      'medium': { 
        color: 'bg-yellow-100 text-yellow-800 border-yellow-300', 
        icon: '🔶', 
        bgColor: 'bg-yellow-50',
        textColor: 'text-yellow-700'
      },
      'low': { 
        color: 'bg-green-100 text-green-800 border-green-300', 
        icon: 'ℹ️', 
        bgColor: 'bg-green-50',
        textColor: 'text-green-700'
      }
    };
    
    return severityProps[severity] || severityProps['medium'];
  }
  
  // Get status display properties
  function getStatusProps(status: string) {
    const statusProps = {
      'active': { color: 'bg-red-100 text-red-800', icon: '🔴', label: 'Active' },
      'investigating': { color: 'bg-blue-100 text-blue-800', icon: '🔵', label: 'Investigating' },
      'resolved': { color: 'bg-green-100 text-green-800', icon: '✅', label: 'Resolved' },
      'false_positive': { color: 'bg-gray-100 text-gray-800', icon: '❌', label: 'False Positive' }
    };
    
    return statusProps[status] || statusProps['active'];
  }
  
  // Format date display
  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    
    if (minutes < 1) {
      return 'Just now';
    } else if (minutes < 60) {
      return `${minutes}m ago`;
    } else if (hours < 24) {
      return `${hours}h ago`;
    } else if (days < 7) {
      return `${days}d ago`;
    } else {
      return date.toLocaleDateString();
    }
  }
  
  // Handle violation selection
  async function selectViolation(violation: SecurityViolation) {
    selectedViolation = violation;
    await loadRelatedEntries(violation.id);
  }
  
  // Filter and search
  function applyFilters() {
    currentPage = 1;
    loadViolationsData();
  }
  
  // Quick actions
  async function resolveViolation(violationId: string) {
    const notes = prompt('Resolution notes (optional):');
    await updateViolationStatus(violationId, 'resolved', notes || undefined);
  }
  
  async function markFalsePositive(violationId: string) {
    const notes = prompt('Please explain why this is a false positive:');
    if (notes) {
      await updateViolationStatus(violationId, 'false_positive', notes);
    }
  }
  
  async function startInvestigation(violationId: string) {
    const assignee = prompt('Assign to (user ID):');
    if (assignee) {
      await Promise.all([
        updateViolationStatus(violationId, 'investigating'),
        assignViolation(violationId, assignee)
      ]);
    } else {
      await updateViolationStatus(violationId, 'investigating');
    }
  }
  
  // Auto-refresh functionality
  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    
    if (autoRefresh) {
      refreshInterval = setInterval(loadViolationsData, 30000);
    } else if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }
  
  onMount(() => {
    loadViolationsData();
    
    if (autoRefresh) {
      refreshInterval = setInterval(loadViolationsData, 30000);
    }
    
    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
      }
    };
  });
  
  // Reactive filtering (only in browser, not during SSR)
  $: if (typeof window !== 'undefined' && (filterSeverity || filterStatus || filterTimeRange)) {
    applyFilters();
  }
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading security violations...</span>
        </div>
      </div>
    </div>
  {:else if error}
    <!-- Error State -->
    <div class="security-card">
      <div class="text-center py-12">
        <div class="text-red-600 mb-4">
          <svg class="h-12 w-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.502 0L4.314 15.5c-.77.833.192 2.5 1.732 2.5z" />
          </svg>
        </div>
        <h3 class="text-lg font-medium text-gray-900 mb-2">Violations Data Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadViolationsData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Security Violations</h2>
          <p class="text-sm text-gray-600 mt-1">
            Track and manage security incidents and policy violations
          </p>
        </div>
        
        <div class="flex items-center gap-3">
          <button
            class="btn-sm {showStatistics ? 'btn-primary' : 'btn-secondary'}"
            on:click={() => showStatistics = !showStatistics}
          >
            {showStatistics ? '📊 Hide' : '📊 Show'} Stats
          </button>
          
          <button
            class="btn-sm {autoRefresh ? 'btn-primary' : 'btn-secondary'}"
            on:click={toggleAutoRefresh}
          >
            {autoRefresh ? '⏸️ Stop' : '🔄 Auto'} Refresh
          </button>
          
          <button class="btn-primary" on:click={loadViolationsData}>
            🔄 Refresh
          </button>
        </div>
      </div>
      
      <!-- Auto-refresh indicator -->
      {#if autoRefresh}
        <div class="text-xs text-green-600">
          🔄 Auto-refreshing every 30 seconds
        </div>
      {/if}
    </div>

    <!-- Statistics -->
    {#if showStatistics && violationStats}
      <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
        <!-- Total Violations -->
        <div class="bg-red-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-red-700">{violationStats.total}</div>
          <div class="text-sm text-red-600">Total Violations</div>
        </div>
        
        <!-- Active Violations -->
        <div class="bg-orange-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-orange-700">{violationStats.active}</div>
          <div class="text-sm text-orange-600">Active</div>
        </div>
        
        <!-- Critical Violations -->
        <div class="bg-purple-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-purple-700">{violationStats.critical}</div>
          <div class="text-sm text-purple-600">Critical</div>
        </div>
        
        <!-- Under Investigation -->
        <div class="bg-blue-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-blue-700">{violationStats.investigating}</div>
          <div class="text-sm text-blue-600">Investigating</div>
        </div>
        
        <!-- Resolved -->
        <div class="bg-green-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-green-700">{violationStats.resolved}</div>
          <div class="text-sm text-green-600">Resolved</div>
        </div>
        
        <!-- Average Resolution Time -->
        <div class="bg-gray-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-gray-700">
            {violationStats.avgResolutionTime ? Math.round(violationStats.avgResolutionTime / 60) : '--'}
          </div>
          <div class="text-sm text-gray-600">Avg Resolution (min)</div>
        </div>
      </div>
    {/if}

    <!-- Filters -->
    <div class="security-card">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
        <!-- Search -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Search</label>
          <input
            type="text"
            bind:value={searchQuery}
            on:input={applyFilters}
            placeholder="Search violations..."
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <!-- Severity Filter -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Severity</label>
          <select bind:value={filterSeverity} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="all">All Severities</option>
            <option value="critical">Critical</option>
            <option value="high">High</option>
            <option value="medium">Medium</option>
            <option value="low">Low</option>
          </select>
        </div>

        <!-- Status Filter -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Status</label>
          <select bind:value={filterStatus} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="all">All Statuses</option>
            <option value="active">Active</option>
            <option value="investigating">Investigating</option>
            <option value="resolved">Resolved</option>
            <option value="false_positive">False Positive</option>
          </select>
        </div>

        <!-- Time Range -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Time Range</label>
          <select bind:value={filterTimeRange} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="1h">Last Hour</option>
            <option value="24h">Last 24 Hours</option>
            <option value="7d">Last 7 Days</option>
            <option value="30d">Last 30 Days</option>
          </select>
        </div>
        
        <!-- Actions -->
        <div class="flex items-end">
          <button class="btn-secondary w-full" on:click={applyFilters}>
            🔍 Apply Filters
          </button>
        </div>
      </div>
    </div>

    <!-- Violations List -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Violations List -->
      <div class="space-y-4">
        {#if violations.length === 0}
          <div class="security-card">
            <div class="text-center py-12">
              <div class="text-green-400 mb-4 text-4xl">✅</div>
              <h3 class="text-lg font-medium text-gray-900 mb-2">No Security Violations</h3>
              <p class="text-gray-600">
                {filterSeverity !== 'all' || filterStatus !== 'all' || searchQuery
                  ? 'No violations match your current filters'
                  : 'No security violations found in the selected time range'}
              </p>
            </div>
          </div>
        {:else}
          {#each violations as violation}
            {@const severityProps = getSeverityProps(violation.severity)}
            {@const statusProps = getStatusProps(violation.status)}
            
            <button
              class="w-full text-left security-card hover:shadow-lg transition-all duration-200 {
                selectedViolation?.id === violation.id ? 'ring-2 ring-blue-500 bg-blue-50' : ''
              }"
              on:click={() => selectViolation(violation)}
            >
              <div class="flex items-start justify-between mb-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-2">
                    <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {severityProps.color}">
                      {severityProps.icon} {violation.severity.toUpperCase()}
                    </span>
                    
                    <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {statusProps.color}">
                      {statusProps.icon} {statusProps.label}
                    </span>
                    
                    {#if violation.riskScore}
                      <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-purple-100 text-purple-800">
                        🎯 Risk: {violation.riskScore}
                      </span>
                    {/if}
                  </div>
                  
                  <h4 class="text-sm font-medium text-gray-900 mb-1">{violation.title}</h4>
                  <p class="text-sm text-gray-600 mb-2">{violation.description}</p>
                  
                  <div class="flex items-center gap-4 text-xs text-gray-500">
                    {#if violation.userId}
                      <span>👤 {violation.userId}</span>
                    {/if}
                    
                    {#if violation.sourceIp}
                      <span>🌐 {violation.sourceIp}</span>
                    {/if}
                    
                    <span>⏰ {formatDate(violation.detectedAt)}</span>
                    
                    {#if violation.count > 1}
                      <span class="bg-red-100 text-red-700 px-1 py-0.5 rounded">
                        {violation.count}× occurrences
                      </span>
                    {/if}
                  </div>
                </div>
                
                <div class="ml-4 flex-shrink-0">
                  <span class="text-gray-400">→</span>
                </div>
              </div>
              
              {#if violation.assigneeId || violation.lastUpdated}
                <div class="border-t border-gray-100 pt-2 text-xs text-gray-500">
                  {#if violation.assigneeId}
                    <span>👤 Assigned to: {violation.assigneeId}</span>
                  {/if}
                  
                  {#if violation.lastUpdated}
                    <span class="ml-4">📝 Updated: {formatDate(violation.lastUpdated)}</span>
                  {/if}
                </div>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
      
      <!-- Violation Details Panel -->
      <div class="space-y-4">
        {#if selectedViolation}
          {@const severityProps = getSeverityProps(selectedViolation.severity)}
          {@const statusProps = getStatusProps(selectedViolation.status)}
          
          <!-- Violation Details -->
          <div class="security-card {severityProps.bgColor}">
            <div class="flex items-start justify-between mb-4">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-2">
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {severityProps.color}">
                    {severityProps.icon} {selectedViolation.severity.toUpperCase()}
                  </span>
                  
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {statusProps.color}">
                    {statusProps.icon} {statusProps.label}
                  </span>
                </div>
                
                <h3 class="text-lg font-medium {severityProps.textColor}">{selectedViolation.title}</h3>
                <p class="text-sm text-gray-700 mt-1">{selectedViolation.description}</p>
              </div>
            </div>
            
            <!-- Violation Metadata -->
            <div class="grid grid-cols-2 gap-4 text-sm">
              <div>
                <span class="font-medium text-gray-700">Detected:</span>
                <div class="text-gray-600">{formatDate(selectedViolation.detectedAt)}</div>
              </div>
              
              {#if selectedViolation.userId}
                <div>
                  <span class="font-medium text-gray-700">User:</span>
                  <div class="text-gray-600">{selectedViolation.userId}</div>
                </div>
              {/if}
              
              {#if selectedViolation.sourceIp}
                <div>
                  <span class="font-medium text-gray-700">Source IP:</span>
                  <div class="text-gray-600 font-mono">{selectedViolation.sourceIp}</div>
                </div>
              {/if}
              
              {#if selectedViolation.riskScore}
                <div>
                  <span class="font-medium text-gray-700">Risk Score:</span>
                  <div class="text-gray-600">{selectedViolation.riskScore}/100</div>
                </div>
              {/if}
              
              {#if selectedViolation.count > 1}
                <div>
                  <span class="font-medium text-gray-700">Occurrences:</span>
                  <div class="text-gray-600">{selectedViolation.count}</div>
                </div>
              {/if}
              
              {#if selectedViolation.assigneeId}
                <div>
                  <span class="font-medium text-gray-700">Assigned to:</span>
                  <div class="text-gray-600">{selectedViolation.assigneeId}</div>
                </div>
              {/if}
            </div>
          </div>
          
          <!-- Quick Actions -->
          <div class="security-card">
            <h4 class="text-sm font-medium text-gray-900 mb-3">Quick Actions</h4>
            
            <div class="grid grid-cols-2 gap-2">
              {#if selectedViolation.status === 'active'}
                <button
                  class="btn-sm btn-secondary"
                  on:click={() => startInvestigation(selectedViolation.id)}
                >
                  🔍 Investigate
                </button>
              {/if}
              
              {#if selectedViolation.status !== 'resolved'}
                <button
                  class="btn-sm btn-primary"
                  on:click={() => resolveViolation(selectedViolation.id)}
                >
                  ✅ Resolve
                </button>
              {/if}
              
              {#if selectedViolation.status !== 'false_positive'}
                <button
                  class="btn-sm btn-secondary"
                  on:click={() => markFalsePositive(selectedViolation.id)}
                >
                  ❌ False Positive
                </button>
              {/if}
              
              <button
                class="btn-sm btn-secondary"
                on:click={() => {
                  const note = prompt('Add investigation note:');
                  if (note) addViolationNote(selectedViolation.id, note);
                }}
              >
                📝 Add Note
              </button>
            </div>
          </div>
          
          <!-- Related Audit Entries -->
          {#if relatedEntries.length > 0}
            <div class="security-card">
              <h4 class="text-sm font-medium text-gray-900 mb-3">
                Related Audit Entries ({relatedEntries.length})
              </h4>
              
              <div class="space-y-2 max-h-64 overflow-y-auto">
                {#each relatedEntries as entry}
                  <div class="p-2 bg-gray-50 rounded text-xs">
                    <div class="flex items-center justify-between mb-1">
                      <span class="font-medium text-gray-900">{entry.eventType}</span>
                      <span class="text-gray-500">{formatDate(entry.timestamp)}</span>
                    </div>
                    <div class="text-gray-600">{entry.message}</div>
                    {#if entry.userId || entry.sourceIp}
                      <div class="flex items-center gap-2 mt-1 text-gray-500">
                        {#if entry.userId}<span>👤 {entry.userId}</span>{/if}
                        {#if entry.sourceIp}<span>🌐 {entry.sourceIp}</span>{/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
          
          <!-- Investigation Timeline -->
          {#if selectedViolation.investigationTimeline && selectedViolation.investigationTimeline.length > 0}
            <div class="security-card">
              <h4 class="text-sm font-medium text-gray-900 mb-3">Investigation Timeline</h4>
              
              <div class="space-y-3">
                {#each selectedViolation.investigationTimeline as event}
                  <div class="flex items-start gap-3">
                    <div class="w-2 h-2 bg-blue-500 rounded-full mt-2"></div>
                    <div class="flex-1">
                      <div class="text-sm font-medium text-gray-900">{event.action}</div>
                      <div class="text-xs text-gray-600">{event.notes}</div>
                      <div class="text-xs text-gray-500 mt-1">
                        {event.performedBy} • {formatDate(event.timestamp)}
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        {:else}
          <div class="security-card">
            <div class="text-center py-12">
              <div class="text-gray-400 mb-4 text-4xl">👆</div>
              <h3 class="text-lg font-medium text-gray-900 mb-2">Select a Violation</h3>
              <p class="text-gray-600">Click on a violation from the list to view detailed information and take action</p>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>