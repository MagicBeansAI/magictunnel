<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  
  // Policy management state
  let policies: any[] = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let showEditModal = false;
  let selectedPolicy: any = null;
  
  // Filters
  let filterEnabled: 'all' | 'enabled' | 'disabled' = 'all';
  let searchQuery = '';
  let sortBy: 'name' | 'priority' | 'created_at' = 'priority';
  let sortDesc = true;
  
  // Statistics
  let policyStats: any = null;
  
  // New policy form
  let newPolicy = {
    name: '',
    description: '',
    priority: 50,
    enabled: true,
    conditions: [],
    actions: []
  };
  
  // Available condition types
  const conditionTypes = [
    { value: 'time_window', label: 'Time Window', example: '09:00-17:00' },
    { value: 'user_role', label: 'User Role', example: 'admin' },
    { value: 'tool_category', label: 'Tool Category', example: 'file_system' },
    { value: 'ip_range', label: 'IP Range', example: '192.168.1.0/24' },
    { value: 'request_rate', label: 'Request Rate', example: '100/hour' }
  ];
  
  // Available action types  
  const actionTypes = [
    { value: 'block', label: 'Block Request' },
    { value: 'require_approval', label: 'Require Approval' },
    { value: 'log_access', label: 'Log Access' },
    { value: 'rate_limit', label: 'Apply Rate Limiting' },
    { value: 'notify_admin', label: 'Notify Administrator' }
  ];
  
  onMount(async () => {
    await Promise.all([
      loadPolicies(),
      loadPolicyStatistics()
    ]);
  });
  
  async function loadPolicies() {
    try {
      loading = true;
      error = '';
      
      // Use our new API with filters
      const params: any = {};
      if (filterEnabled !== 'all') {
        params.enabled = filterEnabled === 'enabled';
      }
      if (searchQuery.trim()) {
        params.search = searchQuery.trim();
      }
      params.sortBy = sortBy;
      params.sortDesc = sortDesc;
      
      policies = await securityApi.getSecurityPolicies(params);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load policies';
      console.error('Error loading policies:', e);
      policies = [];
    } finally {
      loading = false;
    }
  }
  
  async function loadPolicyStatistics() {
    try {
      policyStats = await securityApi.getPolicyStatistics();
    } catch (e) {
      console.error('Error loading policy statistics:', e);
      // Don't show error for stats, just log it
    }
  }
  
  async function createPolicy() {
    try {
      await securityApi.createSecurityPolicy(newPolicy);
      showCreateModal = false;
      resetNewPolicyForm();
      await loadPolicies();
      await loadPolicyStatistics();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create policy';
      console.error('Error creating policy:', e);
    }
  }
  
  async function updatePolicy() {
    if (!selectedPolicy) return;
    
    try {
      await securityApi.updateSecurityPolicy(selectedPolicy.id, selectedPolicy);
      showEditModal = false;
      selectedPolicy = null;
      await loadPolicies();
      await loadPolicyStatistics();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to update policy';
      console.error('Error updating policy:', e);
    }
  }
  
  async function deletePolicy(policyId: string) {
    if (!confirm('Are you sure you want to delete this policy?')) return;
    
    try {
      await securityApi.deleteSecurityPolicy(policyId);
      await loadPolicies();
      await loadPolicyStatistics();
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete policy';
      console.error('Error deleting policy:', e);
    }
  }
  
  async function testPolicy(policyId: string) {
    try {
      const testContext = {
        user_id: 'test_user',
        user_role: 'admin',
        tool_name: 'file_read',
        request_time: new Date().toISOString(),
        client_ip: '192.168.1.1'
      };
      
      const response = await fetch(`/dashboard/api/security/policies/${policyId}/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(testContext)
      });
      
      const result = await response.json();
      if (result.success) {
        const testResult = result.data;
        alert(`Policy Test Result:\n\nPassed: ${testResult.test_passed}\nViolations: ${testResult.violations.length}\nExecution Time: ${testResult.execution_time_ms}ms`);
      } else {
        throw new Error(result.message || 'Failed to test policy');
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to test policy';
    }
  }
  
  function resetNewPolicyForm() {
    newPolicy = {
      name: '',
      description: '',
      priority: 50,
      enabled: true,
      conditions: [],
      actions: []
    };
  }
  
  function addCondition(target: any) {
    target.conditions.push({
      type: 'user_role',
      value: ''
    });
  }
  
  function removeCondition(target: any, index: number) {
    target.conditions.splice(index, 1);
  }
  
  function addAction(target: any) {
    if (!target.actions.includes('log_access')) {
      target.actions.push('log_access');
    }
  }
  
  function removeAction(target: any, action: string) {
    const index = target.actions.indexOf(action);
    if (index > -1) {
      target.actions.splice(index, 1);
    }
  }
  
  function editPolicy(policy: any) {
    selectedPolicy = { ...policy };
    showEditModal = true;
  }
  
  function getSeverityClass(priority: number): string {
    if (priority >= 80) return 'bg-red-100 text-red-800';
    if (priority >= 60) return 'bg-yellow-100 text-yellow-800';
    if (priority >= 40) return 'bg-blue-100 text-blue-800';
    return 'bg-gray-100 text-gray-800';
  }
  
  // Reactive filtering
  $: filteredPolicies = policies.filter(policy => {
    if (filterEnabled !== 'all' && policy.enabled !== (filterEnabled === 'enabled')) {
      return false;
    }
    if (searchQuery.trim() && !policy.name.toLowerCase().includes(searchQuery.toLowerCase()) && 
        !policy.description.toLowerCase().includes(searchQuery.toLowerCase())) {
      return false;
    }
    return true;
  }).sort((a, b) => {
    const direction = sortDesc ? -1 : 1;
    if (sortBy === 'name') return direction * a.name.localeCompare(b.name);
    if (sortBy === 'priority') return direction * (a.priority - b.priority);
    if (sortBy === 'created_at') return direction * (new Date(a.created_at).getTime() - new Date(b.created_at).getTime());
    return 0;
  });
</script>

<svelte:head>
  <title>Security Policies - MagicTunnel</title>
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
          <p>The Policy Engine is currently in alpha stage and requires thorough testing before production use. Please test all policy configurations carefully before enabling in production environments.</p>
        </div>
      </div>
    </div>
  </div>

  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-2xl font-bold text-gray-900">
        Security Policies 
        <span class="alpha-badge">Alpha</span>
      </h1>
      <p class="text-gray-600">Manage business rules and access controls</p>
    </div>
    <button
      on:click={() => showCreateModal = true}
      class="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 transition-colors"
    >
      Create Policy
    </button>
  </div>

  <!-- Error Display -->
  {#if error}
    <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">
      {error}
    </div>
  {/if}

  <!-- Statistics Cards -->
  {#if policyStats}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-blue-100 rounded-lg">
            <svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{policyStats.total_policies}</p>
            <p class="text-gray-600">Total Policies</p>
          </div>
        </div>
      </div>

      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-green-100 rounded-lg">
            <svg class="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{policyStats.active_policies}</p>
            <p class="text-gray-600">Active Policies</p>
          </div>
        </div>
      </div>

      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-purple-100 rounded-lg">
            <svg class="w-6 h-6 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{policyStats.total_evaluations.toLocaleString()}</p>
            <p class="text-gray-600">Total Evaluations</p>
          </div>
        </div>
      </div>

      <div class="bg-white p-6 rounded-lg shadow border">
        <div class="flex items-center">
          <div class="p-2 bg-red-100 rounded-lg">
            <svg class="w-6 h-6 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
          </div>
          <div class="ml-4">
            <p class="text-2xl font-bold text-gray-900">{policyStats.violations_today}</p>
            <p class="text-gray-600">Violations Today</p>
          </div>
        </div>
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
          placeholder="Search policies..."
          bind:value={searchQuery}
          on:input={() => loadPolicies()}
          class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        />
      </div>

      <!-- Filters -->
      <div class="flex gap-2">
        <select 
          bind:value={filterEnabled}
          on:change={() => loadPolicies()}
          class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="all">All Policies</option>
          <option value="enabled">Enabled Only</option>
          <option value="disabled">Disabled Only</option>
        </select>

        <select 
          bind:value={sortBy}
          on:change={() => loadPolicies()}
          class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="priority">Sort by Priority</option>
          <option value="name">Sort by Name</option>
          <option value="created_at">Sort by Created</option>
        </select>

        <button
          on:click={() => { sortDesc = !sortDesc; loadPolicies(); }}
          class="px-3 py-2 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
          title={sortDesc ? 'Sort Ascending' : 'Sort Descending'}
        >
          {sortDesc ? '↓' : '↑'}
        </button>
      </div>
    </div>
  </div>

  <!-- Policies List -->
  <div class="bg-white rounded-lg shadow border">
    {#if loading}
      <div class="p-8 text-center text-gray-500">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto mb-4"></div>
        Loading policies...
      </div>
    {:else if filteredPolicies.length === 0}
      <div class="p-8 text-center text-gray-500">
        <svg class="w-12 h-12 mx-auto mb-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        <p class="text-lg font-medium mb-2">No policies found</p>
        <p class="text-gray-600">Create your first security policy to get started</p>
      </div>
    {:else}
      <div class="overflow-hidden">
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-gray-50">
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Policy</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Priority</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Conditions</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Actions</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Actions</th>
            </tr>
          </thead>
          <tbody class="bg-white divide-y divide-gray-200">
            {#each filteredPolicies as policy}
              <tr class="hover:bg-gray-50">
                <td class="px-6 py-4 whitespace-nowrap">
                  <div>
                    <div class="text-sm font-medium text-gray-900">{policy.name}</div>
                    <div class="text-sm text-gray-500">{policy.description}</div>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {getSeverityClass(policy.priority)}">
                    {policy.priority}
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="text-sm text-gray-900">
                    {policy.conditions.length} condition{policy.conditions.length !== 1 ? 's' : ''}
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="text-sm text-gray-900">
                    {policy.actions.length} action{policy.actions.length !== 1 ? 's' : ''}
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {policy.enabled ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                    {policy.enabled ? 'Enabled' : 'Disabled'}
                  </span>
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium space-x-2">
                  <button
                    on:click={() => testPolicy(policy.id)}
                    class="text-blue-600 hover:text-blue-900"
                  >
                    Test
                  </button>
                  <button
                    on:click={() => editPolicy(policy)}
                    class="text-indigo-600 hover:text-indigo-900"
                  >
                    Edit
                  </button>
                  <button
                    on:click={() => deletePolicy(policy.id)}
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

<!-- Create Policy Modal -->
{#if showCreateModal}
  <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50" on:click={() => showCreateModal = false}>
    <div class="relative top-20 mx-auto p-5 border w-11/12 max-w-2xl shadow-lg rounded-md bg-white" on:click|stopPropagation>
      <div class="mt-3">
        <h3 class="text-lg font-medium text-gray-900 mb-4">Create Security Policy</h3>
        
        <form on:submit|preventDefault={createPolicy} class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
            <input
              type="text"
              bind:value={newPolicy.name}
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
            <textarea
              bind:value={newPolicy.description}
              rows="3"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            ></textarea>
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Priority (0-100)</label>
            <input
              type="number"
              bind:value={newPolicy.priority}
              min="0"
              max="100"
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          
          <div>
            <label class="flex items-center">
              <input
                type="checkbox"
                bind:checked={newPolicy.enabled}
                class="rounded border-gray-300 text-blue-600 shadow-sm focus:border-blue-300 focus:ring focus:ring-blue-200 focus:ring-opacity-50"
              />
              <span class="ml-2 text-sm text-gray-700">Enabled</span>
            </label>
          </div>
          
          <!-- Conditions -->
          <div>
            <div class="flex justify-between items-center mb-2">
              <label class="block text-sm font-medium text-gray-700">Conditions</label>
              <button
                type="button"
                on:click={() => addCondition(newPolicy)}
                class="text-sm text-blue-600 hover:text-blue-700"
              >
                + Add Condition
              </button>
            </div>
            {#each newPolicy.conditions as condition, index}
              <div class="flex gap-2 mb-2">
                <select
                  bind:value={condition.type}
                  class="px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                >
                  {#each conditionTypes as condType}
                    <option value={condType.value}>{condType.label}</option>
                  {/each}
                </select>
                <input
                  type="text"
                  bind:value={condition.value}
                  placeholder={conditionTypes.find(c => c.value === condition.type)?.example || ''}
                  class="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                />
                <button
                  type="button"
                  on:click={() => removeCondition(newPolicy, index)}
                  class="text-red-600 hover:text-red-700 px-2"
                >
                  ✕
                </button>
              </div>
            {/each}
          </div>
          
          <!-- Actions -->
          <div>
            <div class="flex justify-between items-center mb-2">
              <label class="block text-sm font-medium text-gray-700">Actions</label>
              <button
                type="button"
                on:click={() => addAction(newPolicy)}
                class="text-sm text-blue-600 hover:text-blue-700"
              >
                + Add Action
              </button>
            </div>
            <div class="space-y-2">
              {#each actionTypes as actionType}
                <label class="flex items-center">
                  <input
                    type="checkbox"
                    checked={newPolicy.actions.includes(actionType.value)}
                    on:change={(e) => {
                      if (e.target.checked) {
                        if (!newPolicy.actions.includes(actionType.value)) {
                          newPolicy.actions.push(actionType.value);
                        }
                      } else {
                        removeAction(newPolicy, actionType.value);
                      }
                    }}
                    class="rounded border-gray-300 text-blue-600 shadow-sm focus:border-blue-300 focus:ring focus:ring-blue-200 focus:ring-opacity-50"
                  />
                  <span class="ml-2 text-sm text-gray-700">{actionType.label}</span>
                </label>
              {/each}
            </div>
          </div>
          
          <div class="flex justify-end space-x-2 pt-4">
            <button
              type="button"
              on:click={() => { showCreateModal = false; resetNewPolicyForm(); }}
              class="px-4 py-2 text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              Create Policy
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
{/if}

<!-- Edit Policy Modal -->
{#if showEditModal && selectedPolicy}
  <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50" on:click={() => showEditModal = false}>
    <div class="relative top-20 mx-auto p-5 border w-11/12 max-w-2xl shadow-lg rounded-md bg-white" on:click|stopPropagation>
      <div class="mt-3">
        <h3 class="text-lg font-medium text-gray-900 mb-4">Edit Security Policy</h3>
        
        <form on:submit|preventDefault={updatePolicy} class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
            <input
              type="text"
              bind:value={selectedPolicy.name}
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
            <textarea
              bind:value={selectedPolicy.description}
              rows="3"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            ></textarea>
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-1">Priority (0-100)</label>
            <input
              type="number"
              bind:value={selectedPolicy.priority}
              min="0"
              max="100"
              required
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            />
          </div>
          
          <div>
            <label class="flex items-center">
              <input
                type="checkbox"
                bind:checked={selectedPolicy.enabled}
                class="rounded border-gray-300 text-blue-600 shadow-sm focus:border-blue-300 focus:ring focus:ring-blue-200 focus:ring-opacity-50"
              />
              <span class="ml-2 text-sm text-gray-700">Enabled</span>
            </label>
          </div>
          
          <div class="flex justify-end space-x-2 pt-4">
            <button
              type="button"
              on:click={() => { showEditModal = false; selectedPolicy = null; }}
              class="px-4 py-2 text-gray-600 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              Update Policy
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
{/if}