<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { SanitizationPolicy, SanitizationStatistics, SanitizationEvent } from '$lib/types/security';
  
  // State management
  let sanitizationStats: SanitizationStatistics | null = null;
  let recentPolicies: SanitizationPolicy[] = [];
  let recentEvents: SanitizationEvent[] = [];
  let loading = true;
  let error = '';
  let lastUpdated: Date | null = null;
  
  // Time range selector
  let selectedTimeRange: '1h' | '24h' | '7d' | '30d' = '24h';
  
  // Load sanitization dashboard data
  async function loadSanitizationData() {
    try {
      loading = true;
      error = '';
      
      const [statsData, policiesData, eventsData] = await Promise.all([
        securityApi.getSanitizationStatistics(selectedTimeRange),
        securityApi.getSanitizationPolicies({ limit: 8, orderBy: 'modified', order: 'desc' }),
        securityApi.getSanitizationEvents({ limit: 10, orderBy: 'timestamp', order: 'desc' })
      ]);
      
      sanitizationStats = statsData;
      recentPolicies = policiesData.policies || [];
      recentEvents = eventsData.events || [];
      lastUpdated = new Date();
    } catch (err) {
      console.error('Failed to load sanitization data:', err);
      error = `Failed to load sanitization data: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Calculate derived metrics
  $: derivedMetrics = calculateDerivedMetrics(sanitizationStats, recentEvents);
  
  function calculateDerivedMetrics(stats: SanitizationStatistics | null, events: SanitizationEvent[]) {
    if (!stats) return null;
    
    const recentBlocks = events.filter(e => e.action === 'block').length;
    const recentWarnings = events.filter(e => e.action === 'warn').length;
    const recentSanitized = events.filter(e => e.action === 'sanitize').length;
    
    return {
      detectionRate: stats.totalRequests > 0 ? (stats.detectedThreats / stats.totalRequests * 100) : 0,
      blockRate: stats.totalRequests > 0 ? (stats.blockedRequests / stats.totalRequests * 100) : 0,
      sanitizationRate: stats.totalRequests > 0 ? (stats.sanitizedRequests / stats.totalRequests * 100) : 0,
      avgProcessingTime: stats.avgProcessingTime || 0,
      recentBlocks,
      recentWarnings,
      recentSanitized,
      topThreatType: stats.threatTypes && stats.threatTypes.length > 0 ? stats.threatTypes[0] : null
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
  
  // Get policy type display properties
  function getPolicyTypeProps(policyType: string) {
    const typeProps = {
      'content_filter': { color: 'bg-blue-100 text-blue-800', icon: '🔍', label: 'Content Filter' },
      'secret_detection': { color: 'bg-red-100 text-red-800', icon: '🔐', label: 'Secret Detection' },
      'input_validation': { color: 'bg-green-100 text-green-800', icon: '✅', label: 'Input Validation' },
      'output_sanitization': { color: 'bg-purple-100 text-purple-800', icon: '🧹', label: 'Output Sanitization' },
      'pii_detection': { color: 'bg-orange-100 text-orange-800', icon: '👤', label: 'PII Detection' },
      'malware_scan': { color: 'bg-yellow-100 text-yellow-800', icon: '🛡️', label: 'Malware Scan' },
      'rate_limiting': { color: 'bg-cyan-100 text-cyan-800', icon: '⏱️', label: 'Rate Limiting' },
      'custom_rule': { color: 'bg-gray-100 text-gray-800', icon: '⚙️', label: 'Custom Rule' }
    };
    
    return typeProps[policyType] || { color: 'bg-gray-100 text-gray-800', icon: '📝', label: policyType };
  }
  
  // Get action display properties
  function getActionProps(action: string) {
    const actionProps = {
      'block': { color: 'bg-red-100 text-red-800', icon: '🚫', label: 'Blocked' },
      'warn': { color: 'bg-yellow-100 text-yellow-800', icon: '⚠️', label: 'Warning' },
      'sanitize': { color: 'bg-green-100 text-green-800', icon: '🧹', label: 'Sanitized' },
      'log': { color: 'bg-blue-100 text-blue-800', icon: '📝', label: 'Logged' },
      'allow': { color: 'bg-gray-100 text-gray-800', icon: '✅', label: 'Allowed' }
    };
    
    return actionProps[action] || { color: 'bg-gray-100 text-gray-800', icon: '📝', label: action };
  }
  
  // Navigation helpers
  function navigateToPolicies() {
    window.location.href = '/security/sanitization/policies';
  }
  
  function navigateToTesting() {
    window.location.href = '/security/sanitization/testing';
  }
  
  function navigateToEvents() {
    window.location.href = '/security/sanitization/events';
  }
  
  function navigateToPolicy(policyId: string) {
    window.location.href = `/security/sanitization/policies/${policyId}`;
  }
  
  // Quick actions
  async function runSecurityScan() {
    try {
      const result = await securityApi.runSanitizationScan({
        scanType: 'comprehensive',
        timeRange: selectedTimeRange
      });
      
      alert(`Security Scan Complete:\n\n` +
            `• Requests Analyzed: ${result.requestsAnalyzed}\n` +
            `• Threats Detected: ${result.threatsDetected}\n` +
            `• Policies Triggered: ${result.policiesTriggered}\n` +
            `• Actions Taken: ${result.actionsTaken}\n\n` +
            `Scan completed in ${result.duration}ms`);
    } catch (err) {
      alert(`Security scan failed: ${err}`);
    }
  }
  
  async function testAllPolicies() {
    try {
      const result = await securityApi.testAllSanitizationPolicies();
      
      const passed = result.results.filter(r => r.status === 'pass').length;
      const failed = result.results.filter(r => r.status === 'fail').length;
      const warnings = result.results.filter(r => r.status === 'warning').length;
      
      alert(`Policy Test Results:\n\n` +
            `✅ Passed: ${passed}\n` +
            `❌ Failed: ${failed}\n` +
            `⚠️ Warnings: ${warnings}\n\n` +
            `Total Policies Tested: ${result.results.length}`);
    } catch (err) {
      alert(`Policy testing failed: ${err}`);
    }
  }
  
  // Auto-refresh functionality
  onMount(() => {
    loadSanitizationData();
    
    // Auto-refresh every 30 seconds
    const interval = setInterval(loadSanitizationData, 30000);
    
    return () => clearInterval(interval);
  });
  
  // Reload when time range changes
  $: if (selectedTimeRange) {
    loadSanitizationData();
  }
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading sanitization data...</span>
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
        <h3 class="text-lg font-medium text-gray-900 mb-2">Sanitization Data Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadSanitizationData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Request Sanitization Dashboard</h2>
          <p class="text-sm text-gray-600 mt-1">
            Monitor content filtering, secret detection, and request sanitization
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
            on:click={testAllPolicies}
          >
            🧪 Test Policies
          </button>
          
          <button 
            class="btn-secondary"
            on:click={runSecurityScan}
          >
            🔍 Run Scan
          </button>
          
          <button 
            class="btn-primary"
            on:click={navigateToPolicies}
          >
            📝 Manage Policies
          </button>
        </div>
      </div>
      
      <!-- Last Updated -->
      <div class="text-xs text-gray-500">
        Last updated: {lastUpdated?.toLocaleString() || 'Never'} • Auto-refreshing every 30s
      </div>
    </div>

    <!-- Statistics Overview -->
    {#if sanitizationStats}
      <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
        <!-- Total Requests -->
        <div class="bg-blue-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-blue-700">{formatNumber(sanitizationStats.totalRequests)}</div>
          <div class="text-sm text-blue-600">Total Requests</div>
        </div>
        
        <!-- Detected Threats -->
        <div class="bg-red-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-red-700">{formatNumber(sanitizationStats.detectedThreats)}</div>
          <div class="text-sm text-red-600">Threats Detected</div>
          {#if derivedMetrics?.detectionRate}
            <div class="text-xs text-red-500">{derivedMetrics.detectionRate.toFixed(1)}% rate</div>
          {/if}
        </div>
        
        <!-- Blocked Requests -->
        <div class="bg-orange-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-orange-700">{formatNumber(sanitizationStats.blockedRequests)}</div>
          <div class="text-sm text-orange-600">Blocked</div>
          {#if derivedMetrics?.blockRate}
            <div class="text-xs text-orange-500">{derivedMetrics.blockRate.toFixed(1)}% rate</div>
          {/if}
        </div>
        
        <!-- Sanitized Requests -->
        <div class="bg-green-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-green-700">{formatNumber(sanitizationStats.sanitizedRequests)}</div>
          <div class="text-sm text-green-600">Sanitized</div>
          {#if derivedMetrics?.sanitizationRate}
            <div class="text-xs text-green-500">{derivedMetrics.sanitizationRate.toFixed(1)}% rate</div>
          {/if}
        </div>
        
        <!-- Active Policies -->
        <div class="bg-purple-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-purple-700">{formatNumber(sanitizationStats.activePolicies)}</div>
          <div class="text-sm text-purple-600">Active Policies</div>
        </div>
        
        <!-- Avg Processing Time -->
        <div class="bg-gray-50 p-4 rounded-lg">
          <div class="text-2xl font-bold text-gray-700">
            {derivedMetrics?.avgProcessingTime ? derivedMetrics.avgProcessingTime.toFixed(1) : '--'}
          </div>
          <div class="text-sm text-gray-600">Avg Time (ms)</div>
        </div>
      </div>
    {/if}

    <!-- Main Content Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Recent Policies -->
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Active Policies</h3>
          <button class="btn-sm btn-secondary" on:click={navigateToPolicies}>
            View All →
          </button>
        </div>

        {#if recentPolicies.length > 0}
          <div class="space-y-3">
            {#each recentPolicies as policy}
              {@const typeProps = getPolicyTypeProps(policy.type)}
              
              <button
                class="w-full text-left p-3 bg-gray-50 hover:bg-gray-100 rounded-lg transition-colors"
                on:click={() => navigateToPolicy(policy.id)}
              >
                <div class="flex items-start justify-between">
                  <div class="flex items-start gap-3">
                    <span class="text-lg mt-0.5">{typeProps.icon}</span>
                    
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2 mb-1">
                        <span class="text-sm font-medium text-gray-900">{policy.name}</span>
                        <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {typeProps.color}">
                          {typeProps.label}
                        </span>
                        
                        {#if !policy.active}
                          <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-600">
                            ⏸️ Inactive
                          </span>
                        {/if}
                      </div>
                      
                      <p class="text-sm text-gray-600 mb-1">{policy.description}</p>
                      
                      <div class="flex items-center gap-4 text-xs text-gray-500">
                        <span>🎯 {policy.patterns?.length || 0} patterns</span>
                        {#if policy.triggerCount}
                          <span>⚡ {formatNumber(policy.triggerCount)} triggers</span>
                        {/if}
                        <span>⏰ {formatDate(policy.modifiedAt)}</span>
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
            <div class="text-gray-400 mb-2 text-4xl">📝</div>
            <p class="text-gray-600">No policies configured yet</p>
            <button class="btn-primary mt-3" on:click={navigateToPolicies}>
              Create First Policy
            </button>
          </div>
        {/if}
      </div>

      <!-- Recent Events -->
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Recent Activity</h3>
          <button class="btn-sm btn-secondary" on:click={navigateToEvents}>
            View All →
          </button>
        </div>

        {#if recentEvents.length > 0}
          <div class="space-y-3">
            {#each recentEvents as event}
              {@const actionProps = getActionProps(event.action)}
              
              <div class="p-3 bg-gray-50 rounded-lg">
                <div class="flex items-start justify-between mb-2">
                  <div class="flex items-center gap-2">
                    <span class="text-lg">{actionProps.icon}</span>
                    <span class="font-medium text-gray-900">{event.policyName}</span>
                  </div>
                  
                  <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {actionProps.color}">
                    {actionProps.label}
                  </span>
                </div>
                
                <p class="text-sm text-gray-600 mb-2">{event.description}</p>
                
                <div class="flex items-center justify-between text-xs text-gray-500">
                  <div class="flex items-center gap-4">
                    {#if event.userId}
                      <span>👤 {event.userId}</span>
                    {/if}
                    {#if event.sourceIp}
                      <span>🌐 {event.sourceIp}</span>
                    {/if}
                    <span>⏰ {formatDate(event.timestamp)}</span>
                  </div>
                  
                  {#if event.riskLevel}
                    <span class="inline-flex items-center px-1 py-0.5 rounded text-xs font-medium {
                      event.riskLevel === 'high' ? 'bg-red-100 text-red-700' :
                      event.riskLevel === 'medium' ? 'bg-yellow-100 text-yellow-700' :
                      'bg-green-100 text-green-700'
                    }">
                      🎯 {event.riskLevel}
                    </span>
                  {/if}
                </div>
                
                <!-- Detection Details -->
                {#if event.detectionDetails}
                  <details class="mt-2">
                    <summary class="cursor-pointer text-xs text-blue-600 hover:text-blue-800">
                      View detection details
                    </summary>
                    
                    <div class="mt-2 p-2 bg-white rounded border text-xs">
                      {#if event.detectionDetails.matchedPattern}
                        <div class="mb-1">
                          <span class="font-medium">Pattern:</span> 
                          <code class="bg-gray-100 px-1 rounded">{event.detectionDetails.matchedPattern}</code>
                        </div>
                      {/if}
                      
                      {#if event.detectionDetails.confidence}
                        <div class="mb-1">
                          <span class="font-medium">Confidence:</span> 
                          {Math.round(event.detectionDetails.confidence * 100)}%
                        </div>
                      {/if}
                      
                      {#if event.detectionDetails.threatType}
                        <div class="mb-1">
                          <span class="font-medium">Threat Type:</span> 
                          {event.detectionDetails.threatType}
                        </div>
                      {/if}
                    </div>
                  </details>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-center py-8">
            <div class="text-green-400 mb-2 text-4xl">✅</div>
            <p class="text-gray-600">No recent sanitization events</p>
          </div>
        {/if}
      </div>
    </div>

    <!-- Threat Analytics -->
    {#if sanitizationStats?.threatTypes && sanitizationStats.threatTypes.length > 0}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Threat Type Distribution</h3>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {#each sanitizationStats.threatTypes.slice(0, 6) as threatTypeData}
            {@const percentage = (threatTypeData.count / sanitizationStats.detectedThreats * 100).toFixed(1)}
            
            <div class="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
              <div class="flex items-center gap-3">
                <span class="text-xl">
                  {threatTypeData.type === 'secret' ? '🔐' :
                   threatTypeData.type === 'malware' ? '🦠' :
                   threatTypeData.type === 'pii' ? '👤' :
                   threatTypeData.type === 'injection' ? '💉' :
                   threatTypeData.type === 'xss' ? '🕷️' : '⚠️'}
                </span>
                <div>
                  <div class="font-medium text-gray-900 capitalize">{threatTypeData.type}</div>
                  <div class="text-sm text-gray-600">{percentage}% of threats</div>
                </div>
              </div>
              
              <div class="text-lg font-bold text-gray-700">
                {formatNumber(threatTypeData.count)}
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
          <!-- Detection Rate -->
          <div class="text-center p-4 bg-red-50 rounded-lg">
            <div class="text-2xl font-bold text-red-700">
              {derivedMetrics.detectionRate.toFixed(1)}%
            </div>
            <div class="text-sm text-red-600">Detection Rate</div>
          </div>
          
          <!-- Block Rate -->
          <div class="text-center p-4 bg-orange-50 rounded-lg">
            <div class="text-2xl font-bold text-orange-700">
              {derivedMetrics.blockRate.toFixed(1)}%
            </div>
            <div class="text-sm text-orange-600">Block Rate</div>
          </div>
          
          <!-- Processing Time -->
          <div class="text-center p-4 bg-blue-50 rounded-lg">
            <div class="text-2xl font-bold text-blue-700">
              {derivedMetrics.avgProcessingTime.toFixed(1)}
            </div>
            <div class="text-sm text-blue-600">Avg Time (ms)</div>
          </div>
          
          <!-- Top Threat -->
          {#if derivedMetrics.topThreatType}
            <div class="text-center p-4 bg-purple-50 rounded-lg">
              <div class="text-lg font-bold text-purple-700 mb-1">
                {derivedMetrics.topThreatType.type === 'secret' ? '🔐' :
                 derivedMetrics.topThreatType.type === 'malware' ? '🦠' :
                 derivedMetrics.topThreatType.type === 'pii' ? '👤' : '⚠️'}
              </div>
              <div class="text-sm text-purple-600">Most Common</div>
              <div class="text-xs text-purple-500 mt-1 capitalize">{derivedMetrics.topThreatType.type}</div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Policy Effectiveness -->
    {#if recentPolicies.length > 0}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Policy Effectiveness</h3>
        </div>

        <div class="space-y-3">
          {#each recentPolicies.slice(0, 5) as policy}
            {@const effectiveness = policy.triggerCount > 0 ? Math.min(100, (policy.triggerCount / (sanitizationStats?.totalRequests || 1)) * 1000) : 0}
            
            <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
              <div class="flex items-center gap-3">
                <span class="text-lg">{getPolicyTypeProps(policy.type).icon}</span>
                <div>
                  <div class="font-medium text-gray-900">{policy.name}</div>
                  <div class="text-sm text-gray-600">{policy.triggerCount || 0} triggers</div>
                </div>
              </div>
              
              <div class="text-right">
                <div class="text-sm font-bold text-gray-700">{effectiveness.toFixed(1)}%</div>
                <div class="text-xs text-gray-500">effectiveness</div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Quick Actions -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Quick Actions</h3>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <button class="btn-secondary" on:click={navigateToPolicies}>
          📝 Manage Policies
        </button>
        
        <button class="btn-secondary" on:click={navigateToTesting}>
          🧪 Test Sanitization
        </button>
        
        <button class="btn-secondary" on:click={runSecurityScan}>
          🔍 Run Security Scan
        </button>
        
        <button class="btn-secondary" on:click={loadSanitizationData}>
          🔄 Refresh Data
        </button>
      </div>
    </div>
  {/if}
</div>