<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  
  // State management
  let policies: any[] = [];
  let loading = true;
  let error = '';
  let showCreateModal = false;
  let editingPolicy: any = null;
  let selectedPolicies = new Set<string>();
  
  // New policy form
  let newPolicy = {
    name: '',
    enabled: true,
    rules: []
  };
  
  // Load security policies
  async function loadPolicies() {
    try {
      loading = true;
      error = '';
      
      policies = await securityApi.getSecurityPolicies();
    } catch (err) {
      console.error('Failed to load security policies:', err);
      error = `Failed to load security policies: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Create policy
  async function createPolicy() {
    try {
      await securityApi.createSecurityPolicy(newPolicy);
      showCreateModal = false;
      newPolicy = { name: '', enabled: true, rules: [] };
      await loadPolicies();
    } catch (err) {
      console.error('Failed to create policy:', err);
      error = `Failed to create policy: ${err}`;
    }
  }
  
  // Update policy
  async function updatePolicy() {
    if (!editingPolicy) return;
    
    try {
      await securityApi.updateSecurityPolicy(editingPolicy.id, editingPolicy);
      editingPolicy = null;
      await loadPolicies();
    } catch (err) {
      console.error('Failed to update policy:', err);
      error = `Failed to update policy: ${err}`;
    }
  }
  
  // Delete policy
  async function deletePolicy(policyId: string) {
    if (!confirm('Are you sure you want to delete this policy?')) return;
    
    try {
      await securityApi.deleteSecurityPolicy(policyId);
      await loadPolicies();
    } catch (err) {
      console.error('Failed to delete policy:', err);
      error = `Failed to delete policy: ${err}`;
    }
  }
  
  // Test policy
  async function testPolicy(policy: any) {
    try {
      const result = await securityApi.testSecurityPolicy({
        policy_id: policy.id,
        test_data: { tool: 'test_tool', user: 'test_user' }
      });
      
      alert(`Policy Test Result:\n\nOutcome: ${result.results.outcome}\nConfidence: ${result.results.confidence}`);
    } catch (err) {
      console.error('Failed to test policy:', err);
      error = `Failed to test policy: ${err}`;
    }
  }
  
  // Toggle policy selection
  function togglePolicySelection(policyId: string) {
    if (selectedPolicies.has(policyId)) {
      selectedPolicies.delete(policyId);
    } else {
      selectedPolicies.add(policyId);
    }
    selectedPolicies = selectedPolicies;
  }
  
  // Select all policies
  function selectAllPolicies() {
    if (selectedPolicies.size === policies.length) {
      selectedPolicies.clear();
    } else {
      selectedPolicies = new Set(policies.map(p => p.id));
    }
    selectedPolicies = selectedPolicies;
  }
  
  onMount(() => {
    loadPolicies();
  });
</script>

<div class="space-y-6">
  <!-- Actions -->
  <div class="flex items-center justify-between mb-6">
    <p class="text-gray-600">Organization-wide policy enforcement and management</p>
    <button class="btn-primary" on:click={() => showCreateModal = true}>
      ➕ Create Policy
    </button>
  </div>

  <!-- Error Display -->
  {#if error}
    <div class="security-card bg-red-50 border-red-200">
      <p class="text-red-600">{error}</p>
      <button class="btn-secondary mt-2" on:click={() => error = ''}>Dismiss</button>
    </div>
  {/if}

  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600 mr-3"></div>
        <span>Loading security policies...</span>
      </div>
    </div>
  {:else}
    <!-- Policies Table -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Security Policies ({policies.length})</h3>
        <div class="flex gap-2">
          <button class="btn-secondary" on:click={loadPolicies}>
            🔄 Refresh
          </button>
          {#if selectedPolicies.size > 0}
            <button class="btn-danger" on:click={() => console.log('Bulk delete')}>
              🗑️ Delete Selected ({selectedPolicies.size})
            </button>
          {/if}
        </div>
      </div>

      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-gray-50">
            <tr>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                <input 
                  type="checkbox" 
                  checked={selectedPolicies.size === policies.length && policies.length > 0}
                  on:change={selectAllPolicies}
                />
              </th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Name</th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Rules</th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Updated</th>
              <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Actions</th>
            </tr>
          </thead>
          <tbody class="bg-white divide-y divide-gray-200">
            {#each policies as policy}
              <tr class="hover:bg-gray-50">
                <td class="px-4 py-4 whitespace-nowrap">
                  <input 
                    type="checkbox" 
                    checked={selectedPolicies.has(policy.id)}
                    on:change={() => togglePolicySelection(policy.id)}
                  />
                </td>
                <td class="px-4 py-4 whitespace-nowrap">
                  <div class="text-sm font-medium text-gray-900">{policy.name}</div>
                  <div class="text-sm text-gray-500">ID: {policy.id}</div>
                </td>
                <td class="px-4 py-4 whitespace-nowrap">
                  <span class="inline-flex px-2 py-1 text-xs font-semibold rounded-full {
                    policy.enabled 
                      ? 'bg-green-100 text-green-800' 
                      : 'bg-red-100 text-red-800'
                  }">
                    {policy.enabled ? '✅ Enabled' : '❌ Disabled'}
                  </span>
                </td>
                <td class="px-4 py-4 whitespace-nowrap">
                  <div class="text-sm text-gray-900">{policy.rules?.length || 0} rules</div>
                </td>
                <td class="px-4 py-4 whitespace-nowrap text-sm text-gray-500">
                  {policy.updated_at ? new Date(policy.updated_at).toLocaleDateString() : 'Never'}
                </td>
                <td class="px-4 py-4 whitespace-nowrap text-sm font-medium space-x-2">
                  <button class="text-blue-600 hover:text-blue-900" on:click={() => testPolicy(policy)}>
                    🧪 Test
                  </button>
                  <button class="text-indigo-600 hover:text-indigo-900" on:click={() => editingPolicy = {...policy}}>
                    ✏️ Edit
                  </button>
                  <button class="text-red-600 hover:text-red-900" on:click={() => deletePolicy(policy.id)}>
                    🗑️ Delete
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        
        {#if policies.length === 0}
          <div class="text-center py-12">
            <div class="text-gray-400 mb-4">
              <svg class="h-12 w-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            </div>
            <h3 class="text-lg font-medium text-gray-900 mb-2">No Security Policies</h3>
            <p class="text-gray-600 mb-4">Get started by creating your first security policy.</p>
            <button class="btn-primary" on:click={() => showCreateModal = true}>
              ➕ Create Policy
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Create Policy Modal -->
{#if showCreateModal}
  <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
    <div class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white">
      <h3 class="text-lg font-bold text-gray-900 mb-4">Create Security Policy</h3>
      
      <form on:submit|preventDefault={createPolicy}>
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700">Policy Name</label>
            <input 
              type="text" 
              bind:value={newPolicy.name}
              class="mt-1 block w-full rounded-md border-gray-300 shadow-sm"
              required
            />
          </div>
          
          <div class="flex items-center">
            <input 
              type="checkbox" 
              bind:checked={newPolicy.enabled}
              class="h-4 w-4 text-blue-600"
            />
            <label class="ml-2 block text-sm text-gray-900">Enable policy</label>
          </div>
        </div>
        
        <div class="mt-6 flex justify-end space-x-3">
          <button type="button" class="btn-secondary" on:click={() => showCreateModal = false}>
            Cancel
          </button>
          <button type="submit" class="btn-primary">
            Create Policy
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Edit Policy Modal -->
{#if editingPolicy}
  <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
    <div class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white">
      <h3 class="text-lg font-bold text-gray-900 mb-4">Edit Security Policy</h3>
      
      <form on:submit|preventDefault={updatePolicy}>
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700">Policy Name</label>
            <input 
              type="text" 
              bind:value={editingPolicy.name}
              class="mt-1 block w-full rounded-md border-gray-300 shadow-sm"
              required
            />
          </div>
          
          <div class="flex items-center">
            <input 
              type="checkbox" 
              bind:checked={editingPolicy.enabled}
              class="h-4 w-4 text-blue-600"
            />
            <label class="ml-2 block text-sm text-gray-900">Enable policy</label>
          </div>
        </div>
        
        <div class="mt-6 flex justify-end space-x-3">
          <button type="button" class="btn-secondary" on:click={() => editingPolicy = null}>
            Cancel
          </button>
          <button type="submit" class="btn-primary">
            Update Policy
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}