<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import SanitizationPolicyCard from '$lib/components/security/SanitizationPolicyCard.svelte';
  import SanitizationPolicyEditor from '$lib/components/security/SanitizationPolicyEditor.svelte';
  import type { SanitizationPolicy, CreateSanitizationPolicy } from '$lib/types/security';
  
  // State management
  let policies: SanitizationPolicy[] = [];
  let loading = true;
  let error = '';
  let searchQuery = '';
  let filterType: 'all' | 'content_filter' | 'secret_detection' | 'input_validation' | 'output_sanitization' | 'pii_detection' | 'malware_scan' | 'rate_limiting' | 'custom_rule' = 'all';
  let filterStatus: 'all' | 'active' | 'inactive' = 'all';
  let sortBy: 'name' | 'type' | 'created' | 'modified' | 'triggers' = 'name';
  let sortOrder: 'asc' | 'desc' = 'asc';
  
  // Modal states
  let showPolicyEditor = false;
  let editingPolicy: SanitizationPolicy | null = null;
  
  // Selection and bulk operations
  let selectedPolicies = new Set<string>();
  let showBulkActions = false;
  let bulkOperationInProgress = false;
  
  // Pagination
  let currentPage = 1;
  let itemsPerPage = 12;
  
  // Policy templates
  const policyTemplates = [
    {
      name: 'API Key Detection',
      type: 'secret_detection',
      description: 'Detects and blocks API keys in requests',
      patterns: [
        { pattern: 'api[_-]?key[\\s]*[:=][\\s]*[\'"]?([a-zA-Z0-9]{20,})', type: 'regex', name: 'Generic API Key' },
        { pattern: 'sk-[a-zA-Z0-9]{48}', type: 'regex', name: 'OpenAI API Key' },
        { pattern: 'ghp_[a-zA-Z0-9]{36}', type: 'regex', name: 'GitHub Token' }
      ],
      action: 'block',
      severity: 'high'
    },
    {
      name: 'PII Detection',
      type: 'pii_detection',
      description: 'Detects personally identifiable information',
      patterns: [
        { pattern: '\\b\\d{3}-\\d{2}-\\d{4}\\b', type: 'regex', name: 'Social Security Number' },
        { pattern: '\\b4[0-9]{12}(?:[0-9]{3})?\\b', type: 'regex', name: 'Visa Credit Card' },
        { pattern: '\\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Z|a-z]{2,}\\b', type: 'regex', name: 'Email Address' }
      ],
      action: 'sanitize',
      severity: 'medium'
    },
    {
      name: 'SQL Injection Prevention',
      type: 'input_validation',
      description: 'Prevents SQL injection attacks',
      patterns: [
        { pattern: '(union|select|insert|delete|update|drop|create|alter)\\s+.*\\s+(from|into|table|database)', type: 'regex', name: 'SQL Keywords' },
        { pattern: '\\s*(;|--|#|/\\*|\\*/)', type: 'regex', name: 'SQL Comments' },
        { pattern: '\\s*(or|and)\\s+1\\s*=\\s*1', type: 'regex', name: 'Boolean Injection' }
      ],
      action: 'block',
      severity: 'high'
    },
    {
      name: 'XSS Prevention',
      type: 'input_validation',
      description: 'Prevents cross-site scripting attacks',
      patterns: [
        { pattern: '<script[^>]*>.*?<\\/script>', type: 'regex', name: 'Script Tags' },
        { pattern: 'javascript:\\s*', type: 'regex', name: 'JavaScript Protocol' },
        { pattern: 'on(click|load|error|focus|blur)\\s*=', type: 'regex', name: 'Event Handlers' }
      ],
      action: 'sanitize',
      severity: 'high'
    },
    {
      name: 'Rate Limiting',
      type: 'rate_limiting',
      description: 'Limits request frequency per user/IP',
      patterns: [],
      action: 'block',
      severity: 'medium',
      rateLimit: {
        requests: 100,
        windowMs: 60000,
        keyBy: 'ip'
      }
    }
  ];
  
  // Statistics
  $: policyStats = calculatePolicyStats(policies);
  $: filteredPolicies = filterAndSortPolicies(policies, searchQuery, filterType, filterStatus, sortBy, sortOrder);
  $: paginatedPolicies = paginatePolicies(filteredPolicies, currentPage, itemsPerPage);
  $: totalPages = Math.ceil(filteredPolicies.length / itemsPerPage);
  
  function calculatePolicyStats(policies: SanitizationPolicy[]) {
    return {
      total: policies.length,
      active: policies.filter(p => p.active).length,
      inactive: policies.filter(p => !p.active).length,
      byType: {
        content_filter: policies.filter(p => p.type === 'content_filter').length,
        secret_detection: policies.filter(p => p.type === 'secret_detection').length,
        input_validation: policies.filter(p => p.type === 'input_validation').length,
        output_sanitization: policies.filter(p => p.type === 'output_sanitization').length,
        pii_detection: policies.filter(p => p.type === 'pii_detection').length,
        malware_scan: policies.filter(p => p.type === 'malware_scan').length,
        rate_limiting: policies.filter(p => p.type === 'rate_limiting').length,
        custom_rule: policies.filter(p => p.type === 'custom_rule').length
      }
    };
  }
  
  function filterAndSortPolicies(
    policies: SanitizationPolicy[],
    query: string,
    type: string,
    status: string,
    sortBy: string,
    sortOrder: string
  ): SanitizationPolicy[] {
    let filtered = [...policies];
    
    // Apply search filter
    if (query.trim()) {
      const lowerQuery = query.toLowerCase();
      filtered = filtered.filter(policy =>
        policy.name.toLowerCase().includes(lowerQuery) ||
        policy.description?.toLowerCase().includes(lowerQuery) ||
        policy.patterns?.some(p => p.pattern.toLowerCase().includes(lowerQuery))
      );
    }
    
    // Apply type filter
    if (type !== 'all') {
      filtered = filtered.filter(policy => policy.type === type);
    }
    
    // Apply status filter
    if (status !== 'all') {
      filtered = filtered.filter(policy => 
        status === 'active' ? policy.active : !policy.active
      );
    }
    
    // Apply sorting
    filtered.sort((a, b) => {
      let comparison = 0;
      
      switch (sortBy) {
        case 'name':
          comparison = a.name.localeCompare(b.name);
          break;
        case 'type':
          comparison = a.type.localeCompare(b.type);
          break;
        case 'created':
          comparison = new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
          break;
        case 'modified':
          comparison = new Date(a.modifiedAt).getTime() - new Date(b.modifiedAt).getTime();
          break;
        case 'triggers':
          comparison = (a.triggerCount || 0) - (b.triggerCount || 0);
          break;
      }
      
      return sortOrder === 'asc' ? comparison : -comparison;
    });
    
    return filtered;
  }
  
  function paginatePolicies(policies: SanitizationPolicy[], page: number, itemsPerPage: number): SanitizationPolicy[] {
    const startIndex = (page - 1) * itemsPerPage;
    return policies.slice(startIndex, startIndex + itemsPerPage);
  }
  
  // Load policies
  async function loadPolicies() {
    try {
      loading = true;
      error = '';
      policies = await securityApi.getSanitizationPolicies().then(result => result.policies || []);
    } catch (err) {
      console.error('Failed to load policies:', err);
      error = `Failed to load policies: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Policy operations
  async function deletePolicy(policyId: string) {
    const policy = policies.find(p => p.id === policyId);
    
    if (!confirm(`Are you sure you want to delete the policy "${policy?.name}"?\n\nThis action cannot be undone.`)) {
      return;
    }
    
    try {
      await securityApi.deleteSanitizationPolicy(policyId);
      await loadPolicies();
    } catch (err) {
      alert(`Failed to delete policy: ${err}`);
    }
  }
  
  async function togglePolicyStatus(policy: SanitizationPolicy) {
    try {
      await securityApi.updateSanitizationPolicy(policy.id, {
        active: !policy.active
      });
      await loadPolicies();
    } catch (err) {
      alert(`Failed to update policy status: ${err}`);
    }
  }
  
  async function duplicatePolicy(policy: SanitizationPolicy) {
    const duplicatedPolicy: CreateSanitizationPolicy = {
      name: `${policy.name} (Copy)`,
      type: policy.type,
      description: policy.description ? `${policy.description} (Copy)` : '',
      patterns: [...(policy.patterns || [])],
      action: policy.action,
      severity: policy.severity,
      active: false, // Start inactive
      conditions: policy.conditions ? { ...policy.conditions } : undefined,
      rateLimit: policy.rateLimit ? { ...policy.rateLimit } : undefined
    };
    
    try {
      await securityApi.createSanitizationPolicy(duplicatedPolicy);
      await loadPolicies();
    } catch (err) {
      alert(`Failed to duplicate policy: ${err}`);
    }
  }
  
  async function testPolicy(policy: SanitizationPolicy) {
    const testContent = prompt('Enter test content to check against this policy:');
    if (!testContent) return;
    
    try {
      const result = await securityApi.testSanitizationPolicy(policy.id, {
        content: testContent,
        context: {
          userId: 'test_user',
          sourceIp: '192.168.1.100',
          userAgent: 'Test Browser'
        }
      });
      
      let message = `Policy Test Results:\n\n`;
      message += `Policy: ${policy.name}\n`;
      message += `Action: ${result.action}\n`;
      message += `Matched: ${result.matched ? 'Yes' : 'No'}\n`;
      
      if (result.matches && result.matches.length > 0) {
        message += `\nMatches:\n`;
        result.matches.forEach(match => {
          message += `• Pattern: ${match.pattern}\n`;
          message += `  Match: "${match.matchedText}"\n`;
          message += `  Confidence: ${Math.round(match.confidence * 100)}%\n`;
        });
      }
      
      if (result.sanitizedContent && result.sanitizedContent !== testContent) {
        message += `\nSanitized Content:\n${result.sanitizedContent}`;
      }
      
      alert(message);
    } catch (err) {
      alert(`Policy test failed: ${err}`);
    }
  }
  
  // Modal handlers
  function openPolicyEditor(policy?: SanitizationPolicy) {
    editingPolicy = policy || null;
    showPolicyEditor = true;
  }
  
  function closePolicyEditor() {
    showPolicyEditor = false;
    editingPolicy = null;
  }
  
  async function handlePolicySave(event: CustomEvent<CreateSanitizationPolicy>) {
    try {
      if (editingPolicy) {
        await securityApi.updateSanitizationPolicy(editingPolicy.id, event.detail);
      } else {
        await securityApi.createSanitizationPolicy(event.detail);
      }
      
      await loadPolicies();
      closePolicyEditor();
    } catch (err) {
      alert(`Failed to save policy: ${err}`);
    }
  }
  
  // Template operations
  async function createFromTemplate(template: any) {
    const policy: CreateSanitizationPolicy = {
      name: template.name,
      type: template.type,
      description: template.description,
      patterns: template.patterns,
      action: template.action,
      severity: template.severity,
      active: true,
      rateLimit: template.rateLimit
    };
    
    try {
      await securityApi.createSanitizationPolicy(policy);
      await loadPolicies();
    } catch (err) {
      alert(`Failed to create policy from template: ${err}`);
    }
  }
  
  // Selection management
  function togglePolicySelection(policyId: string) {
    const newSelected = new Set(selectedPolicies);
    if (newSelected.has(policyId)) {
      newSelected.delete(policyId);
    } else {
      newSelected.add(policyId);
    }
    selectedPolicies = newSelected;
    showBulkActions = selectedPolicies.size > 0;
  }
  
  function selectAllPolicies() {
    selectedPolicies = new Set(filteredPolicies.map(p => p.id));
    showBulkActions = true;
  }
  
  function clearSelection() {
    selectedPolicies = new Set();
    showBulkActions = false;
  }
  
  // Bulk operations
  async function performBulkOperation(operation: 'activate' | 'deactivate' | 'delete' | 'test') {
    if (selectedPolicies.size === 0) return;
    
    const policyIds = Array.from(selectedPolicies);
    const confirmMessage = 
      operation === 'delete' 
        ? `Are you sure you want to delete ${policyIds.length} policies? This action cannot be undone.`
        : `Are you sure you want to ${operation} ${policyIds.length} policies?`;
    
    if (!confirm(confirmMessage)) return;
    
    try {
      bulkOperationInProgress = true;
      
      if (operation === 'test') {
        const result = await securityApi.testMultipleSanitizationPolicies(policyIds);
        alert(`Bulk Test Results:\n\n${result.results.map(r => `${r.policyName}: ${r.status}`).join('\n')}`);
      } else {
        const operations: any = {};
        if (operation === 'activate') operations.activate = policyIds;
        else if (operation === 'deactivate') operations.deactivate = policyIds;
        else if (operation === 'delete') operations.delete = policyIds;
        
        const result = await securityApi.bulkUpdateSanitizationPolicies(operations);
        
        if (result.failed > 0) {
          alert(`Bulk operation completed with ${result.failed} failures:\n${result.errors.join('\n')}`);
        }
        
        await loadPolicies();
      }
      
      clearSelection();
    } catch (err) {
      alert(`Bulk operation failed: ${err}`);
    } finally {
      bulkOperationInProgress = false;
    }
  }
  
  // Pagination
  function changePage(page: number) {
    currentPage = Math.max(1, Math.min(page, totalPages));
  }
  
  onMount(() => {
    loadPolicies();
  });
</script>

<div class="space-y-6">
  <!-- Header Section -->
  <div class="security-card">
    <div class="security-card-header">
      <div>
        <h2 class="security-card-title">Sanitization Policies</h2>
        <p class="text-sm text-gray-600 mt-1">
          Create and manage content filtering, secret detection, and input validation policies
        </p>
      </div>
      
      <div class="flex items-center gap-3">
        <button 
          class="btn-secondary"
          on:click={() => {
            const template = policyTemplates[Math.floor(Math.random() * policyTemplates.length)];
            createFromTemplate(template);
          }}
        >
          🎯 Quick Template
        </button>
        
        <button 
          class="btn-primary"
          on:click={() => openPolicyEditor()}
        >
          ➕ Create Policy
        </button>
      </div>
    </div>

    <!-- Statistics Cards -->
    <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-8 gap-4 mt-6">
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-900">{policyStats.total}</div>
        <div class="text-sm text-gray-600">Total</div>
      </div>
      
      <div class="bg-green-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-green-700">{policyStats.active}</div>
        <div class="text-sm text-green-600">Active</div>
      </div>
      
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-600">{policyStats.inactive}</div>
        <div class="text-sm text-gray-600">Inactive</div>
      </div>
      
      <div class="bg-red-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-red-700">{policyStats.byType.secret_detection}</div>
        <div class="text-sm text-red-600">Secrets</div>
      </div>
      
      <div class="bg-orange-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-orange-700">{policyStats.byType.pii_detection}</div>
        <div class="text-sm text-orange-600">PII</div>
      </div>
      
      <div class="bg-blue-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-blue-700">{policyStats.byType.input_validation}</div>
        <div class="text-sm text-blue-600">Validation</div>
      </div>
      
      <div class="bg-purple-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-purple-700">{policyStats.byType.content_filter}</div>
        <div class="text-sm text-purple-600">Content</div>
      </div>
      
      <div class="bg-cyan-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-cyan-700">{policyStats.byType.rate_limiting}</div>
        <div class="text-sm text-cyan-600">Rate Limit</div>
      </div>
    </div>
  </div>

  <!-- Quick Templates -->
  <div class="security-card">
    <div class="security-card-header">
      <h3 class="security-card-title">Quick Start Templates</h3>
    </div>
    
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each policyTemplates as template}
        <button
          class="text-left p-4 border border-gray-200 rounded-lg hover:bg-gray-50 hover:border-gray-300 transition-colors"
          on:click={() => createFromTemplate(template)}
        >
          <div class="flex items-center justify-between mb-2">
            <h4 class="font-medium text-gray-900">{template.name}</h4>
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {
              template.type === 'secret_detection' ? 'bg-red-100 text-red-800' :
              template.type === 'pii_detection' ? 'bg-orange-100 text-orange-800' :
              template.type === 'input_validation' ? 'bg-blue-100 text-blue-800' :
              template.type === 'rate_limiting' ? 'bg-cyan-100 text-cyan-800' :
              'bg-gray-100 text-gray-800'
            }">
              {template.type.replace(/_/g, ' ')}
            </span>
          </div>
          
          <p class="text-sm text-gray-600 mb-2">{template.description}</p>
          
          <div class="flex items-center justify-between text-xs text-gray-500">
            <span>{template.patterns.length} patterns</span>
            <span class="capitalize">{template.severity} severity</span>
          </div>
        </button>
      {/each}
    </div>
  </div>

  <!-- Filters and Search -->
  <div class="security-card">
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
      <!-- Search -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Search Policies</label>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search by name, description, patterns..."
          class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      <!-- Type Filter -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Type</label>
        <select bind:value={filterType} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="all">All Types</option>
          <option value="content_filter">Content Filter</option>
          <option value="secret_detection">Secret Detection</option>
          <option value="input_validation">Input Validation</option>
          <option value="output_sanitization">Output Sanitization</option>
          <option value="pii_detection">PII Detection</option>
          <option value="malware_scan">Malware Scan</option>
          <option value="rate_limiting">Rate Limiting</option>
          <option value="custom_rule">Custom Rule</option>
        </select>
      </div>

      <!-- Status Filter -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Status</label>
        <select bind:value={filterStatus} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="all">All Statuses</option>
          <option value="active">Active Only</option>
          <option value="inactive">Inactive Only</option>
        </select>
      </div>

      <!-- Sort Options -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Sort By</label>
        <select bind:value={sortBy} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="name">Name</option>
          <option value="type">Type</option>
          <option value="created">Created</option>
          <option value="modified">Modified</option>
          <option value="triggers">Trigger Count</option>
        </select>
      </div>

      <!-- Sort Direction -->
      <div class="flex items-end">
        <button
          class="flex items-center justify-center w-full px-3 py-2 border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
          on:click={() => sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'}
        >
          {sortOrder === 'asc' ? '↑ Ascending' : '↓ Descending'}
        </button>
      </div>
    </div>
  </div>

  <!-- Bulk Actions Bar -->
  {#if showBulkActions}
    <div class="security-card bg-blue-50 border-blue-200">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <span class="text-sm font-medium text-blue-900">
            {selectedPolicies.size} policies selected
          </span>
          
          <div class="flex items-center gap-2">
            <button
              class="text-sm text-blue-700 hover:text-blue-900 underline"
              on:click={selectAllPolicies}
            >
              Select All ({filteredPolicies.length})
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
            disabled={bulkOperationInProgress}
            on:click={() => performBulkOperation('activate')}
          >
            ✅ Activate
          </button>
          
          <button
            class="btn-sm btn-secondary"
            disabled={bulkOperationInProgress}
            on:click={() => performBulkOperation('deactivate')}
          >
            ⏸️ Deactivate
          </button>
          
          <button
            class="btn-sm btn-secondary"
            disabled={bulkOperationInProgress}
            on:click={() => performBulkOperation('test')}
          >
            🧪 Test
          </button>
          
          <button
            class="btn-sm btn-danger"
            disabled={bulkOperationInProgress}
            on:click={() => performBulkOperation('delete')}
          >
            🗑️ Delete
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Policies List -->
  <div class="space-y-4">
    {#if loading}
      <div class="security-card">
        <div class="flex items-center justify-center py-12">
          <div class="flex items-center gap-3 text-gray-600">
            <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
            <span>Loading policies...</span>
          </div>
        </div>
      </div>
    {:else if error}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-red-600 mb-4">❌</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">Failed to Load Policies</h3>
          <p class="text-gray-600 mb-4">{error}</p>
          <button class="btn-primary" on:click={loadPolicies}>
            🔄 Retry
          </button>
        </div>
      </div>
    {:else if paginatedPolicies.length === 0}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-gray-400 mb-4 text-4xl">📝</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">
            {searchQuery || filterType !== 'all' || filterStatus !== 'all'
              ? 'No Policies Match Your Filters' 
              : 'No Policies Created Yet'}
          </h3>
          <p class="text-gray-600 mb-4">
            {searchQuery || filterType !== 'all' || filterStatus !== 'all'
              ? 'Try adjusting your search criteria or filters'
              : 'Get started by creating your first sanitization policy'}
          </p>
          <button class="btn-primary" on:click={() => openPolicyEditor()}>
            ➕ Create First Policy
          </button>
        </div>
      </div>
    {:else}
      <!-- Policy Cards Grid -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {#each paginatedPolicies as policy}
          <SanitizationPolicyCard
            {policy}
            selected={selectedPolicies.has(policy.id)}
            on:select={() => togglePolicySelection(policy.id)}
            on:edit={() => openPolicyEditor(policy)}
            on:delete={() => deletePolicy(policy.id)}
            on:toggle={() => togglePolicyStatus(policy)}
            on:duplicate={() => duplicatePolicy(policy)}
            on:test={() => testPolicy(policy)}
          />
        {/each}
      </div>

      <!-- Pagination -->
      {#if totalPages > 1}
        <div class="security-card">
          <div class="flex items-center justify-between">
            <div class="text-sm text-gray-600">
              Showing {((currentPage - 1) * itemsPerPage) + 1} to {Math.min(currentPage * itemsPerPage, filteredPolicies.length)} of {filteredPolicies.length} policies
            </div>

            <div class="flex items-center gap-2">
              <button
                class="btn-sm btn-secondary"
                disabled={currentPage <= 1}
                on:click={() => changePage(currentPage - 1)}
              >
                ← Previous
              </button>

              {#each Array.from({length: Math.min(5, totalPages)}, (_, i) => i + Math.max(1, currentPage - 2)) as page}
                {#if page <= totalPages}
                  <button
                    class="btn-sm {page === currentPage ? 'btn-primary' : 'btn-secondary'}"
                    on:click={() => changePage(page)}
                  >
                    {page}
                  </button>
                {/if}
              {/each}

              <button
                class="btn-sm btn-secondary"
                disabled={currentPage >= totalPages}
                on:click={() => changePage(currentPage + 1)}
              >
                Next →
              </button>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<!-- Policy Editor Modal -->
{#if showPolicyEditor}
  <SanitizationPolicyEditor
    policy={editingPolicy}
    on:save={handlePolicySave}
    on:cancel={closePolicyEditor}
  />
{/if}