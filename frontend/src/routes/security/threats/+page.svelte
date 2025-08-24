<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  
  // Threat detection state
  let detectedThreats: any[] = [];
  let threatRules: any[] = [];
  let threatIntelligence: any = {};
  let loading = true;
  let error = '';
  
  // Modals
  let showCreateRuleModal = false;
  let showEditRuleModal = false;
  let showThreatDetailsModal = false;
  let showIntelligenceModal = false;
  
  let selectedRule: any = null;
  let selectedThreat: any = null;
  
  // Filters
  let filterSeverity: 'all' | 'critical' | 'high' | 'medium' | 'low' = 'all';
  let filterTimeRange: '1h' | '24h' | '7d' | '30d' = '24h';
  let searchQuery = '';
  
  // Statistics
  let threatStats: any = null;
  
  // New rule form
  let newRule = {
    name: '',
    priority: 50,
    enabled: true,
    indicators: [],
    severity: 'Medium'
  };
  
  // Available threat indicator types
  const indicatorTypes = [
    { value: 'MaliciousIp', label: 'Malicious IP', example: '192.168.1.100' },
    { value: 'SuspiciousUserAgent', label: 'Suspicious User Agent', example: 'sqlmap' },
    { value: 'BruteForcePattern', label: 'Brute Force Pattern', example: '5' },
    { value: 'SessionHijacking', label: 'Session Hijacking', example: 'ip_change' },
    { value: 'AnomalousBehavior', label: 'Anomalous Behavior', example: '100' },
    { value: 'AttackSignature', label: 'Attack Signature', example: '../' }
  ];
  
  const severityLevels = ['Low', 'Medium', 'High', 'Critical'];
  
  onMount(async () => {
    await Promise.all([
      loadDetectedThreats(),
      loadThreatRules(),
      loadThreatIntelligence(),
      loadThreatStatistics()
    ]);
  });
  
  async function loadDetectedThreats() {
    try {
      loading = true;
      const params = new URLSearchParams();
      if (filterSeverity !== 'all') {
        params.append('severity', filterSeverity);
      }
      params.append('time_range', filterTimeRange);
      if (searchQuery.trim()) {
        params.append('search', searchQuery.trim());
      }
      
      const response = await fetch(`/dashboard/api/security/threats/detected?${params}`);
      const result = await response.json();
      
      if (result.success) {
        detectedThreats = result.data.threats || [];
      } else {
        throw new Error(result.message || 'Failed to load detected threats');
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load detected threats';
      console.error('Error loading detected threats:', e);
    } finally {
      loading = false;
    }
  }
  
  async function loadThreatRules() {
    try {
      threatRules = await securityApi.getThreatDetectionRules();
    } catch (e) {
      console.error('Error loading threat rules:', e);
      threatRules = [];
    }
  }
  
  async function loadThreatIntelligence() {
    try {
      const response = await fetch('/dashboard/api/security/threats/intelligence');
      const result = await response.json();
      if (result.success) {
        threatIntelligence = result.data;
      }
    } catch (e) {
      console.error('Error loading threat intelligence:', e);
    }
  }
  
  async function loadThreatStatistics() {
    try {
      threatStats = await securityApi.getThreatDetectionStatistics();
    } catch (e) {
      console.error('Error loading threat statistics:', e);
    }
  }
  
  async function createThreatRule() {
    try {
      await securityApi.createThreatDetectionRule(newRule);
      showCreateRuleModal = false;
      resetNewRuleForm();
      await loadThreatRules();
      await loadThreatStatistics();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create threat rule';
      console.error('Error creating threat rule:', e);
    }
  }
  
  async function updateThreatRule() {
    if (!selectedRule) return;
    
    try {
      await securityApi.updateThreatDetectionRule(selectedRule.id, selectedRule);
      showEditRuleModal = false;
      selectedRule = null;
      await loadThreatRules();
      await loadThreatStatistics();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to update threat rule';
      console.error('Error updating threat rule:', e);
    }
  }
  
  async function deleteThreatRule(ruleId: string) {
    if (!confirm('Are you sure you want to delete this threat rule?')) return;
    
    try {
      await securityApi.deleteThreatDetectionRule(ruleId);
      await loadThreatRules();
      await loadThreatStatistics();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete threat rule';
      console.error('Error deleting threat rule:', e);
    }
  }
  
  function resetNewRuleForm() {
    newRule = {
      name: '',
      priority: 50,
      enabled: true,
      indicators: [],
      severity: 'Medium'
    };
  }
  
  function addIndicator(target: any) {
    target.indicators.push({
      indicator_type: 'MaliciousIp',
      value: '',
      confidence: 0.8
    });
  }
  
  function removeIndicator(target: any, index: number) {
    target.indicators.splice(index, 1);
  }
  
  function editRule(rule: any) {
    selectedRule = { ...rule };
    showEditRuleModal = true;
  }
  
  function viewThreatDetails(threat: any) {
    selectedThreat = threat;
    showThreatDetailsModal = true;
  }
  
  function getSeverityClass(severity: string): string {
    const severityLower = severity.toLowerCase();
    if (severityLower === 'critical') return 'bg-red-100 text-red-800';
    if (severityLower === 'high') return 'bg-orange-100 text-orange-800';
    if (severityLower === 'medium') return 'bg-yellow-100 text-yellow-800';
    return 'bg-gray-100 text-gray-800';
  }
  
  function getSeverityIcon(severity: string): string {
    const severityLower = severity.toLowerCase();
    if (severityLower === 'critical') return '🔴';
    if (severityLower === 'high') return '🟠';
    if (severityLower === 'medium') return '🟡';
    return '⚪';
  }
  
  function formatTimestamp(timestamp: string): string {
    return new Date(timestamp).toLocaleString();
  }
  
  // Reactive filtering
  $: filteredThreats = detectedThreats.filter(threat => {
    if (filterSeverity !== 'all' && threat.severity.toLowerCase() !== filterSeverity) {
      return false;
    }
    if (searchQuery.trim() && !threat.description.toLowerCase().includes(searchQuery.toLowerCase()) && 
        !threat.rule_id.toLowerCase().includes(searchQuery.toLowerCase())) {
      return false;
    }
    return true;
  });
</script>

<svelte:head>
  <title>Threat Detection - MagicTunnel</title>
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <!-- Alpha Warning -->
  <div class="alpha-warning">
    <div class="flex">
      <div class="flex-shrink-0">
        <svg class="h-5 w-5 text-orange-400" fill="currentColor" viewBox="0 0 20 20">
          <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
        </svg>
      </div>
      <div class="ml-3">
        <h3 class="text-sm font-medium text-orange-800">Alpha Service - Testing Required</h3>
        <div class="mt-2 text-sm text-orange-700">
          <p>The Threat Detection Engine is currently in alpha stage and requires thorough testing before production use. All threat detection is currently in monitor-only mode. Please validate all threat detection rules before enabling auto-response features.</p>
        </div>
      </div>
    </div>
  </div>

  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-2xl font-bold text-gray-900">
        Threat Detection 
        <span class="alpha-badge">Alpha</span>
      </h1>
      <p class="text-gray-600">Monitor and manage security threats</p>
    </div>
    <div class="flex space-x-2">
      <button
        on:click={() => showIntelligenceModal = true}
        class="bg-purple-600 text-white px-4 py-2 rounded-lg hover:bg-purple-700 transition-colors"
      >
        Threat Intel
      </button>
      <button
        on:click={() => showCreateRuleModal = true}
        class="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 transition-colors"
      >
        Create Rule
      </button>
    </div>
  </div>

  <!-- Error Display -->
  {#if error}
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">
      {error}
    </div>
  {/if}

  <!-- Statistics Cards -->
  {#if threatStats}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-red-100 rounded-lg">
            <svg class="w-6 h-6 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{threatStats.total_threats}</p>
            <p class="text-gray-600">Total Threats</p>
          </div>
        </div>
      </div>

      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-orange-100 rounded-lg">
            <svg class="w-6 h-6 text-orange-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L5.636 5.636" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{threatStats.blocked_requests}</p>
            <p class="text-gray-600">Blocked Requests</p>
          </div>
        </div>
      </div>

      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-blue-100 rounded-lg">
            <svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{threatStats.detection_rate?.today || 0}</p>
            <p class="text-gray-600">Today's Detections</p>
          </div>
        </div>
      </div>

      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-green-100 rounded-lg">
            <svg class="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{threatStats.false_positives}</p>
            <p class="text-gray-600">False Positives</p>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Threat Severity Breakdown -->
  {#if threatStats?.threats_by_severity}
    <div class="bg-white p-6 rounded-lg shadow border">
      <h3 class="text-lg font-medium text-gray-900 mb-4">Threats by Severity</h3>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
        {#each Object.entries(threatStats.threats_by_severity) as [severity, count]}
          <div class="text-center">
            <div class="text-2xl font-bold {getSeverityClass(severity).split(' ')[1]}">{count}</div>
            <div class="text-sm text-gray-600 capitalize">{severity}</div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Controls -->
  <div class="bg-white p-4 rounded-lg shadow border">
    <div class="flex flex-col sm:flex-row gap-4">
      <!-- Search -->
      <div class="flex-1">
        <input
          type="text"
          placeholder="Search threats..."
          bind:value={searchQuery}
          on:input={() => loadDetectedThreats()}
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        />
      </div>

      <!-- Filters -->
      <div class="flex gap-2">
        <select 
          bind:value={filterSeverity}
          on:change={() => loadDetectedThreats()}
          class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="all">All Severities</option>
          <option value="critical">Critical</option>
          <option value="high">High</option>
          <option value="medium">Medium</option>
          <option value="low">Low</option>
        </select>

        <select 
          bind:value={filterTimeRange}
          on:change={() => loadDetectedThreats()}
          class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="1h">Last Hour</option>
          <option value="24h">Last 24 Hours</option>
          <option value="7d">Last 7 Days</option>
          <option value="30d">Last 30 Days</option>
        </select>
      </div>
    </div>
  </div>

  <!-- Threats List -->
  <div class="bg-white rounded-lg shadow border">
    {#if loading}
      <div class="p-8 text-center text-gray-500">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-4"></div>
        Loading detected threats...
      </div>
    {:else if filteredThreats.length === 0}
      <div class="p-8 text-center text-gray-500">
        <svg class="w-12 h-12 mx-auto mb-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.618 5.984A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
        </svg>
        <p class="text-lg font-medium mb-2">No threats detected</p>
        <p class="text-gray-600">Your system is secure - no threats found in the selected time range</p>
      </div>
    {:else}
      <div class="overflow-hidden">
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-gray-50">
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Threat</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Type</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Severity</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Confidence</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Detected</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Actions</th>
            </tr>
          </thead>
          <tbody class="bg-white divide-y divide-gray-200">
            {#each filteredThreats as threat}
              <tr class="hover:bg-gray-50">
                <td class="px-6 py-4 whitespace-nowrap">
                  <div>
                    <div class="text-sm font-medium text-gray-900">
                      {getSeverityIcon(threat.severity)} {threat.description}
                    </div>
                    <div class="text-sm text-gray-500">Rule: {threat.rule_id}</div>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="text-sm text-gray-900">{threat.threat_type}</span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {getSeverityClass(threat.severity)}">
                    {threat.severity}
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="flex items-center">
                    <div class="w-full bg-gray-200 rounded-full h-2">
                      <div class="bg-blue-600 h-2 rounded-full" style="width: {threat.confidence * 100}%"></div>
                    </div>
                    <span class="ml-2 text-sm text-gray-600">{Math.round(threat.confidence * 100)}%</span>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                  {formatTimestamp(threat.detected_at)}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium">
                  <button
                    on:click={() => viewThreatDetails(threat)}
                    class="text-blue-600 hover:text-blue-900"
                  >
                    View Details
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>

  <!-- Threat Rules Section -->
  <div class="bg-white rounded-lg shadow border">
    <div class="p-6 border-b">
      <div class="flex justify-between items-center">
        <h3 class="text-lg font-medium text-gray-900">Threat Detection Rules</h3>
        <span class="text-sm text-gray-500">{threatRules.length} rules configured</span>
      </div>
    </div>
    
    {#if threatRules.length === 0}
      <div class="p-8 text-center text-gray-500">
        <p class="text-lg font-medium mb-2">No threat detection rules configured</p>
        <p class="text-gray-600">Create rules to detect specific threat patterns</p>
      </div>
    {:else}
      <div class="overflow-hidden">
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-gray-50">
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Rule</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Priority</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Indicators</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Severity</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Actions</th>
            </tr>
          </thead>
          <tbody class="bg-white divide-y divide-gray-200">
            {#each threatRules as rule}
              <tr class="hover:bg-gray-50">
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="text-sm font-medium text-gray-900">{rule.name}</div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="text-sm text-gray-900">{rule.priority}</span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="text-sm text-gray-900">{rule.indicators?.length || 0}</span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {getSeverityClass(rule.severity)}">
                    {rule.severity}
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {rule.enabled ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                    {rule.enabled ? 'Enabled' : 'Disabled'}
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium space-x-2">
                  <button
                    on:click={() => editRule(rule)}
                    class="text-indigo-600 hover:text-indigo-900"
                  >
                    Edit
                  </button>
                  <button
                    on:click={() => deleteThreatRule(rule.id)}
                    class="text-red-600 hover:text-red-900"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
</div>

<!-- Create Threat Rule Modal -->
{#if showCreateRuleModal}
  <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50" on:click={() => showCreateRuleModal = false}>
    <div class="relative top-20 mx-auto p-5 border w-11/12 max-w-2xl shadow-lg rounded-md bg-white" on:click|stopPropagation>
      <div class="mt-3">
        <h3 class="text-lg font-medium text-gray-900 mb-4">Create Threat Detection Rule</h3>
        
        <form on:submit|preventDefault={createThreatRule} class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
            <input
              type="text"
              bind:value={newRule.name}
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Priority (0-100)</label>
            <input
              type="number"
              bind:value={newRule.priority}
              min="0"
              max="100"
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Severity</label>
            <select
              bind:value={newRule.severity}
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            >
              {#each severityLevels as level}
                <option value={level}>{level}</option>
              {/each}
            </select>
          </div>
          
          <div>
            <label class="flex items-center">
              <input
                type="checkbox"
                bind:checked={newRule.enabled}
                class="rounded border-gray-300 text-blue-600 shadow-sm focus:border-blue-300 focus:ring focus:ring-blue-200 focus:ring-opacity-50"
              />
              <span class="ml-2 text-sm text-gray-700">Enabled</span>
            </label>
          </div>
          
          <!-- Indicators -->
          <div>
            <div class="flex justify-between items-center mb-2">
              <label class="block text-sm font-medium text-gray-700">Threat Indicators</label>
              <button
                type="button"
                on:click={() => addIndicator(newRule)}
                class="text-sm text-blue-600 hover:text-blue-700"
              >
                + Add Indicator
              </button>
            </div>
            {#each newRule.indicators as indicator, index}
              <div class="flex gap-2 mb-2 items-center">
                <select
                  bind:value={indicator.indicator_type}
                  class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                >
                  {#each indicatorTypes as indType}
                    <option value={indType.value}>{indType.label}</option>
                  {/each}
                </select>
                <input
                  type="text"
                  bind:value={indicator.value}
                  placeholder={indicatorTypes.find(i => i.value === indicator.indicator_type)?.example || ''}
                  class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                />
                <input
                  type="number"
                  bind:value={indicator.confidence}
                  min="0"
                  max="1"
                  step="0.1"
                  placeholder="0.8"
                  class="w-20 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                />
                <button
                  type="button"
                  on:click={() => removeIndicator(newRule, index)}
                  class="text-red-600 hover:text-red-700 px-2"
                >
                  ✕
                </button>
              </div>
            {/each}
          </div>
          
          <div class="flex justify-end space-x-2 pt-4">
            <button
              type="button"
              on:click={() => { showCreateRuleModal = false; resetNewRuleForm(); }}
              class="px-4 py-2 text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              Create Rule
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
{/if}

<!-- Threat Intelligence Modal -->
{#if showIntelligenceModal && threatIntelligence}
  <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50" on:click={() => showIntelligenceModal = false}>
    <div class="relative top-20 mx-auto p-5 border w-11/12 max-w-4xl shadow-lg rounded-md bg-white" on:click|stopPropagation>
      <div class="mt-3">
        <h3 class="text-lg font-medium text-gray-900 mb-4">Threat Intelligence</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
          <!-- Malicious IPs -->
          <div>
            <h4 class="text-md font-medium text-gray-900 mb-2">Malicious IPs ({threatIntelligence.malicious_ips?.length || 0})</h4>
            <div class="max-h-64 overflow-y-auto bg-gray-50 p-3 rounded-lg">
              {#if threatIntelligence.malicious_ips?.length > 0}
                {#each threatIntelligence.malicious_ips as ip}
                  <div class="text-sm text-gray-700 mb-1 font-mono">{ip}</div>
                {/each}
              {:else}
                <div class="text-sm text-gray-500">No malicious IPs loaded</div>
              {/if}
            </div>
          </div>
          
          <!-- Suspicious User Agents -->
          <div>
            <h4 class="text-md font-medium text-gray-900 mb-2">Suspicious User Agents ({threatIntelligence.suspicious_user_agents?.length || 0})</h4>
            <div class="max-h-64 overflow-y-auto bg-gray-50 p-3 rounded-lg">
              {#if threatIntelligence.suspicious_user_agents?.length > 0}
                {#each threatIntelligence.suspicious_user_agents as ua}
                  <div class="text-sm text-gray-700 mb-1 font-mono">{ua}</div>
                {/each}
              {:else}
                <div class="text-sm text-gray-500">No suspicious user agents loaded</div>
              {/if}
            </div>
          </div>
          
          <!-- Attack Signatures -->
          <div>
            <h4 class="text-md font-medium text-gray-900 mb-2">Attack Signatures ({threatIntelligence.attack_signatures?.length || 0})</h4>
            <div class="max-h-64 overflow-y-auto bg-gray-50 p-3 rounded-lg">
              {#if threatIntelligence.attack_signatures?.length > 0}
                {#each threatIntelligence.attack_signatures as signature}
                  <div class="text-sm text-gray-700 mb-1 font-mono">{signature}</div>
                {/each}
              {:else}
                <div class="text-sm text-gray-500">No attack signatures loaded</div>
              {/if}
            </div>
          </div>
        </div>
        
        {#if threatIntelligence.last_updated}
          <div class="mt-4 text-sm text-gray-500">
            Last updated: {formatTimestamp(threatIntelligence.last_updated)}
          </div>
        {/if}
        
        <div class="flex justify-end space-x-2 pt-4">
          <button
            on:click={() => showIntelligenceModal = false}
            class="px-4 py-2 text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Threat Details Modal -->
{#if showThreatDetailsModal && selectedThreat}
  <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50" on:click={() => showThreatDetailsModal = false}>
    <div class="relative top-20 mx-auto p-5 border w-11/12 max-w-3xl shadow-lg rounded-md bg-white" on:click|stopPropagation>
      <div class="mt-3">
        <h3 class="text-lg font-medium text-gray-900 mb-4">Threat Details</h3>
        
        <div class="grid grid-cols-2 gap-4 mb-6">
          <div>
            <label class="block text-sm font-medium text-gray-700">Threat ID</label>
            <div class="text-sm text-gray-900">{selectedThreat.id}</div>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700">Rule ID</label>
            <div class="text-sm text-gray-900">{selectedThreat.rule_id}</div>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700">Type</label>
            <div class="text-sm text-gray-900">{selectedThreat.threat_type}</div>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700">Severity</label>
            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {getSeverityClass(selectedThreat.severity)}">
              {selectedThreat.severity}
            </span>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700">Confidence</label>
            <div class="text-sm text-gray-900">{Math.round(selectedThreat.confidence * 100)}%</div>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700">Detected At</label>
            <div class="text-sm text-gray-900">{formatTimestamp(selectedThreat.detected_at)}</div>
          </div>
        </div>
        
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-700 mb-2">Description</label>
          <div class="text-sm text-gray-900 bg-gray-50 p-3 rounded-lg">{selectedThreat.description}</div>
        </div>
        
        {#if selectedThreat.indicators?.length > 0}
          <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 mb-2">Indicators</label>
            <div class="bg-gray-50 p-3 rounded-lg">
              {#each selectedThreat.indicators as indicator}
                <div class="text-sm text-gray-700 mb-1 font-mono">{indicator}</div>
              {/each}
            </div>
          </div>
        {/if}
        
        {#if selectedThreat.source}
          <div class="mb-4">
            <label class="block text-sm font-medium text-gray-700 mb-2">Source Information</label>
            <div class="bg-gray-50 p-3 rounded-lg">
              <div class="grid grid-cols-2 gap-4 text-sm">
                {#if selectedThreat.source.ip}
                  <div>
                    <span class="font-medium text-gray-700">IP:</span>
                    <span class="text-gray-900 font-mono">{selectedThreat.source.ip}</span>
                  </div>
                {/if}
                {#if selectedThreat.source.user_agent}
                  <div>
                    <span class="font-medium text-gray-700">User Agent:</span>
                    <span class="text-gray-900 font-mono">{selectedThreat.source.user_agent}</span>
                  </div>
                {/if}
                {#if selectedThreat.source.request_path}
                  <div class="col-span-2">
                    <span class="font-medium text-gray-700">Request Path:</span>
                    <span class="text-gray-900 font-mono">{selectedThreat.source.request_path}</span>
                  </div>
                {/if}
              </div>
            </div>
          </div>
        {/if}
        
        <div class="flex justify-end space-x-2 pt-4">
          <button
            on:click={() => { showThreatDetailsModal = false; selectedThreat = null; }}
            class="px-4 py-2 text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}