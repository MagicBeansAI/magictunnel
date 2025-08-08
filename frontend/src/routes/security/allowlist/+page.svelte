<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import AllowlistRuleCard from '$lib/components/security/AllowlistRuleCard.svelte';
  import AllowlistRuleEditor from '$lib/components/security/AllowlistRuleEditor.svelte';
  import AllowlistRuleTester from '$lib/components/security/AllowlistRuleTester.svelte';
  import type { AllowlistRule, CreateAllowlistRule } from '$lib/types/security';

  // State management
  let rules: AllowlistRule[] = [];
  let loading = true;
  let error = '';
  let searchQuery = '';
  let filterType: 'all' | 'tool' | 'resource' | 'global' = 'all';
  let filterAction: 'all' | 'allow' | 'deny' | 'require_approval' = 'all';
  let sortBy: 'name' | 'priority' | 'created' | 'modified' = 'priority';
  let sortOrder: 'asc' | 'desc' = 'desc';

  // Modal states
  let showRuleEditor = false;
  let showRuleTester = false;
  let editingRule: AllowlistRule | null = null;

  // Selection and bulk operations
  let selectedRules = new Set<string>();
  let showBulkActions = false;
  let bulkOperationInProgress = false;

  // Pagination
  let currentPage = 1;
  let itemsPerPage = 20;

  // Statistics
  $: ruleStats = calculateRuleStats(rules);
  $: filteredRules = filterAndSortRules(rules, searchQuery, filterType, filterAction, sortBy, sortOrder);
  $: paginatedRules = paginateRules(filteredRules, currentPage, itemsPerPage);
  $: totalPages = Math.ceil(filteredRules.length / itemsPerPage);

  function calculateRuleStats(rules: AllowlistRule[]) {
    const stats = {
      total: rules.length,
      active: rules.filter(r => r.active).length,
      inactive: rules.filter(r => !r.active).length,
      byType: {
        tool: rules.filter(r => r.type === 'tool').length,
        resource: rules.filter(r => r.type === 'resource').length,
        global: rules.filter(r => r.type === 'global').length,
      },
      byAction: {
        allow: rules.filter(r => r.action === 'allow').length,
        deny: rules.filter(r => r.action === 'deny').length,
        require_approval: rules.filter(r => r.action === 'require_approval').length,
      }
    };
    return stats;
  }

  function filterAndSortRules(
    rules: AllowlistRule[],
    query: string,
    type: string,
    action: string,
    sortBy: string,
    sortOrder: string
  ): AllowlistRule[] {
    let filtered = [...rules];

    // Apply search filter
    if (query.trim()) {
      const lowerQuery = query.toLowerCase();
      filtered = filtered.filter(rule =>
        rule.name.toLowerCase().includes(lowerQuery) ||
        rule.pattern.toLowerCase().includes(lowerQuery)
      );
    }

    // Apply type filter
    if (type !== 'all') {
      filtered = filtered.filter(rule => rule.type === type);
    }

    // Apply action filter
    if (action !== 'all') {
      filtered = filtered.filter(rule => rule.action === action);
    }

    // Apply sorting
    filtered.sort((a, b) => {
      let comparison = 0;
      
      switch (sortBy) {
        case 'name':
          comparison = a.name.localeCompare(b.name);
          break;
        case 'priority':
          comparison = a.priority - b.priority;
          break;
        case 'created':
          comparison = new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
          break;
        case 'modified':
          comparison = new Date(a.modifiedAt).getTime() - new Date(b.modifiedAt).getTime();
          break;
      }

      return sortOrder === 'asc' ? comparison : -comparison;
    });

    return filtered;
  }

  function paginateRules(rules: AllowlistRule[], page: number, itemsPerPage: number): AllowlistRule[] {
    const startIndex = (page - 1) * itemsPerPage;
    return rules.slice(startIndex, startIndex + itemsPerPage);
  }

  // Load allowlist rules
  async function loadRules() {
    try {
      loading = true;
      error = '';
      rules = await securityApi.getAllowlistRules();
    } catch (err) {
      console.error('Failed to load allowlist rules:', err);
      error = `Failed to load rules: ${err}`;
    } finally {
      loading = false;
    }
  }

  // Rule operations
  async function deleteRule(ruleId: string) {
    if (!confirm('Are you sure you want to delete this rule? This action cannot be undone.')) {
      return;
    }

    try {
      await securityApi.deleteAllowlistRule(ruleId);
      await loadRules();
      // Show success message
    } catch (err) {
      alert(`Failed to delete rule: ${err}`);
    }
  }

  async function toggleRuleStatus(rule: AllowlistRule) {
    try {
      await securityApi.updateAllowlistRule(rule.id, {
        active: !rule.active
      });
      await loadRules();
    } catch (err) {
      alert(`Failed to update rule status: ${err}`);
    }
  }

  // Modal handlers
  function openRuleEditor(rule?: AllowlistRule) {
    editingRule = rule || null;
    showRuleEditor = true;
  }

  function closeRuleEditor() {
    showRuleEditor = false;
    editingRule = null;
  }

  async function handleRuleSave(event: CustomEvent<CreateAllowlistRule>) {
    try {
      if (editingRule) {
        await securityApi.updateAllowlistRule(editingRule.id, event.detail);
      } else {
        await securityApi.createAllowlistRule(event.detail);
      }
      
      await loadRules();
      closeRuleEditor();
    } catch (err) {
      alert(`Failed to save rule: ${err}`);
    }
  }

  // Selection management
  function toggleRuleSelection(ruleId: string) {
    const newSelected = new Set(selectedRules);
    if (newSelected.has(ruleId)) {
      newSelected.delete(ruleId);
    } else {
      newSelected.add(ruleId);
    }
    selectedRules = newSelected;
    showBulkActions = selectedRules.size > 0;
  }

  function selectAllRules() {
    selectedRules = new Set(filteredRules.map(r => r.id));
    showBulkActions = true;
  }

  function clearSelection() {
    selectedRules = new Set();
    showBulkActions = false;
  }

  // Bulk operations
  async function performBulkOperation(operation: 'enable' | 'disable' | 'delete') {
    if (selectedRules.size === 0) return;

    const ruleIds = Array.from(selectedRules);
    const confirmMessage = 
      operation === 'delete' 
        ? `Are you sure you want to delete ${ruleIds.length} rules? This action cannot be undone.`
        : `Are you sure you want to ${operation} ${ruleIds.length} rules?`;

    if (!confirm(confirmMessage)) return;

    try {
      bulkOperationInProgress = true;
      
      const operations: any = {};
      if (operation === 'enable') operations.enable = ruleIds;
      else if (operation === 'disable') operations.disable = ruleIds;
      else if (operation === 'delete') operations.delete = ruleIds;

      const result = await securityApi.bulkUpdateAllowlistRules(operations);
      
      if (result.failed > 0) {
        alert(`Bulk operation completed with ${result.failed} failures:\n${result.errors.join('\n')}`);
      }
      
      await loadRules();
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
    loadRules();
  });
</script>

<div class="space-y-6">
  <!-- Header Section -->
  <div class="security-card">
    <div class="security-card-header">
      <div>
        <h2 class="security-card-title">Tool Allowlisting Rules</h2>
        <p class="text-sm text-gray-600 mt-1">
          Control which tools and resources can be accessed by users and API keys
        </p>
      </div>
      
      <div class="flex items-center gap-3">
        <button 
          class="btn-secondary"
          on:click={() => showRuleTester = true}
        >
          🧪 Test Rules
        </button>
        
        <button 
          class="btn-primary"
          on:click={() => openRuleEditor()}
        >
          ➕ Add Rule
        </button>
      </div>
    </div>

    <!-- Statistics Cards -->
    <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4 mt-6">
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-900">{ruleStats.total}</div>
        <div class="text-sm text-gray-600">Total Rules</div>
      </div>
      
      <div class="bg-green-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-green-700">{ruleStats.active}</div>
        <div class="text-sm text-green-600">Active</div>
      </div>
      
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-600">{ruleStats.inactive}</div>
        <div class="text-sm text-gray-600">Inactive</div>
      </div>
      
      <div class="bg-blue-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-blue-700">{ruleStats.byType.tool}</div>
        <div class="text-sm text-blue-600">Tool Rules</div>
      </div>
      
      <div class="bg-purple-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-purple-700">{ruleStats.byType.resource}</div>
        <div class="text-sm text-purple-600">Resource Rules</div>
      </div>
      
      <div class="bg-orange-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-orange-700">{ruleStats.byType.global}</div>
        <div class="text-sm text-orange-600">Global Rules</div>
      </div>
    </div>
  </div>

  <!-- Filters and Search -->
  <div class="security-card">
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
      <!-- Search -->
      <div class="lg:col-span-2">
        <label class="block text-sm font-medium text-gray-700 mb-2">Search Rules</label>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search by name or pattern..."
          class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      <!-- Type Filter -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Type</label>
        <select bind:value={filterType} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="all">All Types</option>
          <option value="tool">Tool Rules</option>
          <option value="resource">Resource Rules</option>
          <option value="global">Global Rules</option>
        </select>
      </div>

      <!-- Action Filter -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Action</label>
        <select bind:value={filterAction} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="all">All Actions</option>
          <option value="allow">Allow</option>
          <option value="deny">Deny</option>
          <option value="require_approval">Require Approval</option>
        </select>
      </div>

      <!-- Sort Options -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Sort By</label>
        <div class="flex gap-2">
          <select bind:value={sortBy} class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
            <option value="priority">Priority</option>
            <option value="name">Name</option>
            <option value="created">Created</option>
            <option value="modified">Modified</option>
          </select>
          
          <button
            class="px-3 py-2 border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
            on:click={() => sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'}
          >
            {sortOrder === 'asc' ? '↑' : '↓'}
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- Bulk Actions Bar -->
  {#if showBulkActions}
    <div class="security-card bg-blue-50 border-blue-200">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <span class="text-sm font-medium text-blue-900">
            {selectedRules.size} rules selected
          </span>
          
          <div class="flex items-center gap-2">
            <button
              class="text-sm text-blue-700 hover:text-blue-900 underline"
              on:click={selectAllRules}
            >
              Select All ({filteredRules.length})
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
            on:click={() => performBulkOperation('enable')}
          >
            ✅ Enable
          </button>
          
          <button
            class="btn-sm btn-secondary"
            disabled={bulkOperationInProgress}
            on:click={() => performBulkOperation('disable')}
          >
            ⏸️ Disable
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

  <!-- Rules List -->
  <div class="space-y-4">
    {#if loading}
      <div class="security-card">
        <div class="flex items-center justify-center py-12">
          <div class="flex items-center gap-3 text-gray-600">
            <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
            <span>Loading allowlist rules...</span>
          </div>
        </div>
      </div>
    {:else if error}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-red-600 mb-4">❌</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">Failed to Load Rules</h3>
          <p class="text-gray-600 mb-4">{error}</p>
          <button class="btn-primary" on:click={loadRules}>
            🔄 Retry
          </button>
        </div>
      </div>
    {:else if paginatedRules.length === 0}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-gray-400 mb-4 text-4xl">📋</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">
            {searchQuery || filterType !== 'all' || filterAction !== 'all' 
              ? 'No Rules Match Your Filters' 
              : 'No Allowlist Rules'}
          </h3>
          <p class="text-gray-600 mb-4">
            {searchQuery || filterType !== 'all' || filterAction !== 'all'
              ? 'Try adjusting your search criteria or filters'
              : 'Get started by creating your first allowlist rule'}
          </p>
          <button class="btn-primary" on:click={() => openRuleEditor()}>
            ➕ Create First Rule
          </button>
        </div>
      </div>
    {:else}
      {#each paginatedRules as rule}
        <AllowlistRuleCard
          {rule}
          selected={selectedRules.has(rule.id)}
          on:select={() => toggleRuleSelection(rule.id)}
          on:edit={() => openRuleEditor(rule)}
          on:delete={() => deleteRule(rule.id)}
          on:toggle={() => toggleRuleStatus(rule)}
        />
      {/each}

      <!-- Pagination -->
      {#if totalPages > 1}
        <div class="security-card">
          <div class="flex items-center justify-between">
            <div class="text-sm text-gray-600">
              Showing {((currentPage - 1) * itemsPerPage) + 1} to {Math.min(currentPage * itemsPerPage, filteredRules.length)} of {filteredRules.length} rules
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

<!-- Rule Editor Modal -->
{#if showRuleEditor}
  <AllowlistRuleEditor
    rule={editingRule}
    on:save={handleRuleSave}
    on:cancel={closeRuleEditor}
  />
{/if}

<!-- Rule Tester Modal -->
{#if showRuleTester}
  <AllowlistRuleTester
    {rules}
    on:close={() => showRuleTester = false}
  />
{/if}