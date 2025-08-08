<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { AuditEntry, AuditStatistics, SecurityAlert } from '$lib/types/security';
  
  // State management
  let auditStats: AuditStatistics | null = null;
  let recentEntries: AuditEntry[] = [];
  let securityAlerts: SecurityAlert[] = [];
  let loading = true;
  let error = '';
  let lastUpdated: Date | null = null;
  
  // Time range selector
  let selectedTimeRange: '1h' | '24h' | '7d' | '30d' = '24h';
  
  // Load audit dashboard data
  async function loadAuditData() {
    try {
      loading = true;
      error = '';
      
      const [statsData, entriesData, alertsData] = await Promise.all([
        securityApi.getAuditStatistics(selectedTimeRange),
        securityApi.getAuditEntries({ limit: 10, orderBy: 'timestamp', order: 'desc' }),
        securityApi.getSecurityAlerts({ severity: 'high', limit: 5, status: 'active' })
      ]);
      
      auditStats = statsData;
      recentEntries = entriesData.entries || [];
      securityAlerts = alertsData.alerts || [];
      lastUpdated = new Date();
    } catch (err) {
      console.error('Failed to load audit data:', err);
      error = `Failed to load audit data: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Calculate derived metrics
  $: derivedMetrics = calculateDerivedMetrics(auditStats);
  
  function calculateDerivedMetrics(stats: AuditStatistics | null) {
    if (!stats) return null;
    
    return {
      violationRate: stats.totalEntries > 0 ? (stats.violations / stats.totalEntries * 100) : 0,
      topEventType: stats.eventTypes && stats.eventTypes.length > 0 ? stats.eventTypes[0] : null,
      averageEntriesPerHour: stats.totalEntries / (selectedTimeRange === '1h' ? 1 : selectedTimeRange === '24h' ? 24 : selectedTimeRange === '7d' ? 168 : 720),
      criticalIssues: securityAlerts.filter(a => a.severity === 'critical').length,
      highIssues: securityAlerts.filter(a => a.severity === 'high').length
    };
  }
  
  // Format numbers for display
  function formatNumber(num: number): string {
    if (num >= 1000000) {
      return (num / 1000000).toFixed(1) + 'M';
    } else if (num >= 1000) {
      return (num / 1000).toFixed(1) + 'K';
    }
    return num.toString();
  }
  
  // Format date display
  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    
    if (minutes < 1) {
      return 'Just now';
    } else if (minutes < 60) {
      return `${minutes}m ago`;
    } else if (hours < 24) {
      return `${hours}h ago`;
    } else {
      return date.toLocaleDateString();
    }
  }
  
  // Get event type display properties
  function getEventTypeProps(eventType: string) {
    const eventProps = {
      'auth_success': { color: 'bg-green-100 text-green-800', icon: '✅' },
      'auth_failure': { color: 'bg-red-100 text-red-800', icon: '🚫' },
      'permission_granted': { color: 'bg-blue-100 text-blue-800', icon: '🔓' },
      'permission_denied': { color: 'bg-red-100 text-red-800', icon: '🔒' },
      'role_assigned': { color: 'bg-purple-100 text-purple-800', icon: '👤' },
      'role_removed': { color: 'bg-orange-100 text-orange-800', icon: '👤' },
      'policy_violation': { color: 'bg-red-100 text-red-800', icon: '⚠️' },
      'config_changed': { color: 'bg-yellow-100 text-yellow-800', icon: '⚙️' },
      'api_access': { color: 'bg-cyan-100 text-cyan-800', icon: '🔌' },
      'data_access': { color: 'bg-indigo-100 text-indigo-800', icon: '📊' }
    };
    
    return eventProps[eventType] || { color: 'bg-gray-100 text-gray-800', icon: '📝' };
  }
  
  // Get severity display properties
  function getSeverityProps(severity: string) {
    const severityProps = {
      'critical': { color: 'bg-red-100 text-red-800', icon: '🚨' },
      'high': { color: 'bg-orange-100 text-orange-800', icon: '⚠️' },
      'medium': { color: 'bg-yellow-100 text-yellow-800', icon: '🔶' },
      'low': { color: 'bg-green-100 text-green-800', icon: 'ℹ️' },
      'info': { color: 'bg-blue-100 text-blue-800', icon: '📘' }
    };
    
    return severityProps[severity] || { color: 'bg-gray-100 text-gray-800', icon: '📝' };
  }
  
  // Navigation helpers
  function navigateToSearch() {
    window.location.href = '/security/audit/search';
  }
  
  function navigateToViolations() {
    window.location.href = '/security/audit/violations';
  }
  
  function navigateToEntry(entryId: string) {
    window.location.href = `/security/audit/search?id=${entryId}`;
  }
  
  // Quick actions
  async function exportAuditLog() {
    try {
      const result = await securityApi.exportAuditLog({
        timeRange: selectedTimeRange,
        format: 'csv'
      });
      
      // Trigger download
      const blob = new Blob([result.data], { type: 'text/csv' });
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `audit-log-${selectedTimeRange}-${new Date().toISOString().split('T')[0]}.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      window.URL.revokeObjectURL(url);
    } catch (err) {
      alert(`Failed to export audit log: ${err}`);
    }
  }
  
  // Auto-refresh functionality
  onMount(() => {
    loadAuditData();
    
    // Auto-refresh every 30 seconds
    const interval = setInterval(loadAuditData, 30000);
    
    return () => clearInterval(interval);
  });
  
  // Reload when time range changes
  $: if (selectedTimeRange) {
    loadAuditData();
  }
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading audit data...</span>
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
        <h3 class="text-lg font-medium text-gray-900 mb-2">Audit Data Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadAuditData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Audit Logging Dashboard</h2>
          <p class="text-sm text-gray-600 mt-1">
            Monitor security events, violations, and system activity
          </p>
        </div>
        
        <div class="flex items-center gap-3">
          <!-- Time Range Selector -->
          <select 
            bind:value={selectedTimeRange}
            class="px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="1h">Last Hour</option>
            <option value="24h">Last 24 Hours</option>
            <option value="7d">Last 7 Days</option>
            <option value="30d">Last 30 Days</option>
          </select>
          
          <button 
            class="btn-secondary"
            on:click={exportAuditLog}
          >
            📤 Export
          </button>
          
          <button 
            class="btn-secondary"
            on:click={navigateToViolations}
          >
            🚨 Violations
          </button>
          
          <button 
            class="btn-primary"
            on:click={navigateToSearch}
          >
            🔍 Search Logs
          </button>
        </div>
      </div>
      
      <!-- Last Updated -->
      <div class="text-xs text-gray-500">
        Last updated: {lastUpdated?.toLocaleString() || 'Never'} • Auto-refreshing every 30s
      </div>
    </div>

    <!-- Statistics Overview -->
    {#if auditStats}
      <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
        <!-- Total Entries -->
        <div class="bg-blue-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-blue-700">{formatNumber(auditStats.totalEntries)}</div>
          <div class="text-sm text-blue-600">Total Events</div>
        </div>
        
        <!-- Violations -->
        <div class="bg-red-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-red-700">{formatNumber(auditStats.violations)}</div>
          <div class="text-sm text-red-600">Violations</div>
          {#if derivedMetrics?.violationRate}
            <div class="text-xs text-red-500">{derivedMetrics.violationRate.toFixed(1)}% rate</div>
          {/if}
        </div>
        
        <!-- Auth Events -->
        <div class="bg-green-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-green-700">{formatNumber(auditStats.authEvents)}</div>
          <div class="text-sm text-green-600">Auth Events</div>
        </div>
        
        <!-- Failed Auth -->
        <div class="bg-orange-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-orange-700">{formatNumber(auditStats.failedAuth)}</div>
          <div class="text-sm text-orange-600">Auth Failures</div>
        </div>
        
        <!-- Unique Users -->
        <div class="bg-purple-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-purple-700">{formatNumber(auditStats.uniqueUsers)}</div>
          <div class="text-sm text-purple-600">Unique Users</div>
        </div>
        
        <!-- Critical Alerts -->
        <div class="bg-gray-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-gray-700">{derivedMetrics?.criticalIssues || 0}</div>
          <div class="text-sm text-gray-600">Critical Issues</div>
        </div>
      </div>
    {/if}

    <!-- Main Content Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Recent Activity -->
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Recent Activity</h3>
          <button class="btn-sm btn-secondary" on:click={navigateToSearch}>
            View All →
          </button>
        </div>

        {#if recentEntries.length > 0}
          <div class="space-y-3">
            {#each recentEntries as entry}
              {@const eventProps = getEventTypeProps(entry.eventType)}
              {@const severityProps = getSeverityProps(entry.severity)}
              
              <button
                class="w-full text-left p-3 bg-gray-50 hover:bg-gray-100 rounded-lg transition-colors"
                on:click={() => navigateToEntry(entry.id)}
              >
                <div class="flex items-start justify-between">
                  <div class="flex items-start gap-3">
                    <span class="text-lg mt-0.5">{eventProps.icon}</span>
                    
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-1">
                        <span class="text-sm font-medium text-gray-900">{entry.eventType}</span>
                        <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {severityProps.color}">
                          {severityProps.icon} {entry.severity}
                        </span>
                      </div>
                      
                      <p class="text-sm text-gray-600 mb-1">{entry.message}</p>
                      
                      <div class="flex items-center gap-4 text-xs text-gray-500">
                        <span>👤 {entry.userId || 'System'}</span>
                        {#if entry.sourceIp}
                          <span>🌐 {entry.sourceIp}</span>
                        {/if}
                        <span>⏰ {formatDate(entry.timestamp)}</span>
                      </div>
                    </div>
                  </div>
                  
                  <span class="text-gray-400 ml-2">→</span>
                </div>
              </button>
            {/each}
          </div>
        {:else}
          <div class="text-center py-8">
            <div class="text-gray-400 mb-2 text-4xl">📊</div>
            <p class="text-gray-600">No recent audit entries</p>
          </div>
        {/if}
      </div>

      <!-- Security Alerts -->
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Security Alerts</h3>
          <button class="btn-sm btn-secondary" on:click={navigateToViolations}>
            View All →
          </button>
        </div>

        {#if securityAlerts.length > 0}
          <div class="space-y-3">
            {#each securityAlerts as alert}
              {@const severityProps = getSeverityProps(alert.severity)}
              
              <div class="p-3 border border-gray-200 rounded-lg">
                <div class="flex items-start justify-between mb-2">
                  <div class="flex items-center gap-2">
                    <span class="text-lg">{severityProps.icon}</span>
                    <span class="font-medium text-gray-900">{alert.title}</span>
                  </div>
                  
                  <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {severityProps.color}">
                    {alert.severity}
                  </span>
                </div>
                
                <p class="text-sm text-gray-600 mb-2">{alert.description}</p>
                
                <div class="flex items-center justify-between text-xs text-gray-500">
                  <span>⏰ {formatDate(alert.timestamp)}</span>
                  {#if alert.count > 1}
                    <span class="bg-red-100 text-red-700 px-2 py-0.5 rounded-full">
                      {alert.count} occurrences
                    </span>
                  {/if}
                </div>
                
                {#if alert.actionRequired}
                  <div class="mt-2 pt-2 border-t border-gray-100">
                    <div class="text-xs text-orange-700 font-medium">Action Required:</div>
                    <div class="text-xs text-gray-600 mt-1">{alert.actionRequired}</div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-center py-8">
            <div class="text-green-400 mb-2 text-4xl">✅</div>
            <p class="text-gray-600">No active security alerts</p>
          </div>
        {/if}
      </div>
    </div>

    <!-- Event Analytics -->
    {#if auditStats?.eventTypes && auditStats.eventTypes.length > 0}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Event Type Distribution</h3>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {#each auditStats.eventTypes.slice(0, 6) as eventTypeData}
            {@const eventProps = getEventTypeProps(eventTypeData.type)}
            {@const percentage = (eventTypeData.count / auditStats.totalEntries * 100).toFixed(1)}
            
            <div class="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
              <div class="flex items-center gap-3">
                <span class="text-xl">{eventProps.icon}</span>
                <div>
                  <div class="font-medium text-gray-900">{eventTypeData.type}</div>
                  <div class="text-sm text-gray-600">{percentage}% of events</div>
                </div>
              </div>
              
              <div class="text-lg font-bold text-gray-700">
                {formatNumber(eventTypeData.count)}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Performance Metrics -->
    {#if derivedMetrics}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Performance Insights</h3>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <!-- Average Events per Hour -->
          <div class="text-center p-4 bg-blue-50 rounded-lg">
            <div class="text-2xl font-bold text-blue-700">
              {derivedMetrics.averageEntriesPerHour.toFixed(1)}
            </div>
            <div class="text-sm text-blue-600">Events/Hour</div>
          </div>
          
          <!-- Violation Rate -->
          <div class="text-center p-4 bg-red-50 rounded-lg">
            <div class="text-2xl font-bold text-red-700">
              {derivedMetrics.violationRate.toFixed(1)}%
            </div>
            <div class="text-sm text-red-600">Violation Rate</div>
          </div>
          
          <!-- Top Event Type -->
          {#if derivedMetrics.topEventType}
            <div class="text-center p-4 bg-green-50 rounded-lg">
              <div class="text-lg font-bold text-green-700 mb-1">
                {getEventTypeProps(derivedMetrics.topEventType.type).icon}
              </div>
              <div class="text-sm text-green-600">Most Common</div>
              <div class="text-xs text-green-500 mt-1">{derivedMetrics.topEventType.type}</div>
            </div>
          {/if}
          
          <!-- Critical Issues -->
          <div class="text-center p-4 bg-orange-50 rounded-lg">
            <div class="text-2xl font-bold text-orange-700">
              {derivedMetrics.criticalIssues + derivedMetrics.highIssues}
            </div>
            <div class="text-sm text-orange-600">High+ Issues</div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Quick Actions -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Quick Actions</h3>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <button class="btn-secondary" on:click={navigateToSearch}>
          🔍 Advanced Search
        </button>
        
        <button class="btn-secondary" on:click={navigateToViolations}>
          🚨 View Violations
        </button>
        
        <button class="btn-secondary" on:click={exportAuditLog}>
          📤 Export Logs
        </button>
        
        <button class="btn-secondary" on:click={loadAuditData}>
          🔄 Refresh Data
        </button>
      </div>
    </div>
  {/if}
</div>