<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { Permission, PermissionCategory } from '$lib/types/security';
  
  // State management
  let permissions: Permission[] = [];
  let categories: PermissionCategory[] = [];
  let loading = true;
  let error = '';
  let lastUpdated: Date | null = null;
  
  // UI state
  let selectedCategory = 'all';
  let searchTerm = '';
  let showCreateModal = false;
  
  // Load permissions data
  async function loadPermissions() {
    try {
      loading = true;
      error = '';
      
      try {
        const [permissionsData, categoriesData] = await Promise.all([
          securityApi.getPermissions(),
          securityApi.getPermissionCategories()
        ]);
        
        permissions = permissionsData;
        categories = categoriesData;
      } catch (err) {
        console.log('Permissions endpoints not available yet, using mock data:', err);
        
        // Generate mock permissions from roles data
        try {
          const rolesData = await securityApi.getRoles();
          const allPermissions = new Set();
          rolesData.forEach(role => {
            if (role.permissions) {
              role.permissions.forEach(perm => allPermissions.add(perm));
            }
          });
          
          permissions = Array.from(allPermissions).map((perm, index) => ({
            id: `perm-${index}`,
            name: perm,
            description: `Permission: ${perm}`,
            category: perm === '*' ? 'system' : 'standard',
            active: true,
            isSystem: perm === '*' || perm.startsWith('admin'),
          }));
          
          categories = [
            { id: 'system', name: 'System', description: 'System-level permissions', active: true },
            { id: 'standard', name: 'Standard', description: 'Standard user permissions', active: true }
          ];
        } catch (roleErr) {
          console.error('Could not load roles for mock permissions:', roleErr);
          permissions = [];
          categories = [];
        }
      }
      
      lastUpdated = new Date();
    } catch (err) {
      console.error('Failed to load permissions:', err);
      error = `Failed to load permissions: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Filter permissions
  $: filteredPermissions = permissions.filter(permission => {
    const matchesSearch = !searchTerm || 
      permission.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      permission.description?.toLowerCase().includes(searchTerm.toLowerCase());
    
    const matchesCategory = selectedCategory === 'all' || 
      permission.category === selectedCategory;
    
    return matchesSearch && matchesCategory;
  });
  
  // Group permissions by category
  $: permissionsByCategory = filteredPermissions.reduce((acc, permission) => {
    const category = permission.category || 'uncategorized';
    if (!acc[category]) {
      acc[category] = [];
    }
    acc[category].push(permission);
    return acc;
  }, {} as Record<string, Permission[]>);
  
  // Calculate statistics
  $: stats = {
    total: permissions.length,
    active: permissions.filter(p => p.active !== false).length,
    categories: categories.length,
    systemPerms: permissions.filter(p => p.isSystem).length
  };
  
  onMount(() => {
    loadPermissions();
  });
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading permissions...</span>
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
        <h3 class="text-lg font-medium text-gray-900 mb-2">Permissions Data Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadPermissions}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Permissions Management</h2>
          <p class="text-sm text-gray-600 mt-1">
            Manage system permissions and access controls
          </p>
        </div>
        
        <div class="flex items-center gap-3">
          <button 
            class="btn-secondary"
            on:click={() => showCreateModal = true}
          >
            ➕ Create Permission
          </button>
          
          <button 
            class="btn-secondary"
            on:click={loadPermissions}
          >
            🔄 Refresh
          </button>
        </div>
      </div>
      
      <!-- Last Updated -->
      <div class="text-xs text-gray-500">
        Last updated: {lastUpdated?.toLocaleString() || 'Never'}
      </div>
    </div>

    <!-- Statistics Overview -->
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="bg-blue-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-blue-700">{stats.total}</div>
        <div class="text-sm text-blue-600">Total Permissions</div>
      </div>
      
      <div class="bg-green-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-green-700">{stats.active}</div>
        <div class="text-sm text-green-600">Active Permissions</div>
      </div>
      
      <div class="bg-purple-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-purple-700">{stats.categories}</div>
        <div class="text-sm text-purple-600">Categories</div>
      </div>
      
      <div class="bg-orange-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-orange-700">{stats.systemPerms}</div>
        <div class="text-sm text-orange-600">System Permissions</div>
      </div>
    </div>

    <!-- Filters and Search -->
    <div class="security-card">
      <div class="flex flex-col md:flex-row gap-4">
        <!-- Search -->
        <div class="flex-1">
          <label for="search" class="block text-sm font-medium text-gray-700 mb-1">
            Search Permissions
          </label>
          <input
            id="search"
            type="text"
            bind:value={searchTerm}
            placeholder="Search by name or description..."
            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          />
        </div>
        
        <!-- Category Filter -->
        <div>
          <label for="category" class="block text-sm font-medium text-gray-700 mb-1">
            Filter by Category
          </label>
          <select
            id="category"
            bind:value={selectedCategory}
            class="px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500"
          >
            <option value="all">All Categories</option>
            {#each categories as category}
              <option value={category.id}>{category.name}</option>
            {/each}
          </select>
        </div>
      </div>
    </div>

    <!-- Permissions List -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">
          Permissions ({filteredPermissions.length})
        </h3>
      </div>

      {#if filteredPermissions.length === 0}
        <div class="text-center py-12">
          <div class="text-gray-400 mb-4">
            <svg class="h-12 w-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m0 0v2m0-2h2m-2 0H10m4-6.5V7a2 2 0 00-2-2H6a2 2 0 00-2 2v10a2 2 0 002 2h6a2 2 0 002-2v-.5" />
            </svg>
          </div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">No permissions found</h3>
          <p class="text-gray-600 mb-4">
            {searchTerm || selectedCategory !== 'all' 
              ? 'Try adjusting your search or filter criteria.'
              : 'No permissions are configured yet.'}
          </p>
          {#if !searchTerm && selectedCategory === 'all'}
            <button class="btn-primary" on:click={() => showCreateModal = true}>
              Create First Permission
            </button>
          {/if}
        </div>
      {:else}
        <!-- Group by Category -->
        <div class="space-y-6">
          {#each Object.entries(permissionsByCategory) as [categoryId, categoryPermissions]}
            {@const category = categories.find(c => c.id === categoryId) || { name: 'Uncategorized', description: '' }}
            
            <div class="space-y-3">
              <div class="flex items-center gap-3 pb-2 border-b border-gray-200">
                <h4 class="text-lg font-medium text-gray-900">{category.name}</h4>
                <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-gray-100 text-gray-800">
                  {categoryPermissions.length} permissions
                </span>
              </div>
              
              <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {#each categoryPermissions as permission}
                  <div class="p-4 border rounded-lg hover:shadow-md transition-shadow">
                    <div class="flex items-start justify-between mb-2">
                      <h5 class="font-medium text-gray-900">{permission.name}</h5>
                      <div class="flex items-center gap-2">
                        {#if permission.isSystem}
                          <span class="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium bg-purple-100 text-purple-800">
                            System
                          </span>
                        {/if}
                        <span class="inline-flex items-center px-1.5 py-0.5 rounded-full text-xs font-medium {
                          permission.active !== false ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
                        }">
                          {permission.active !== false ? '✓' : '○'}
                        </span>
                      </div>
                    </div>
                    
                    {#if permission.description}
                      <p class="text-sm text-gray-600 mb-3">{permission.description}</p>
                    {/if}
                    
                    <div class="flex items-center justify-between">
                      <div class="text-xs text-gray-500">
                        ID: {permission.id}
                      </div>
                      
                      <div class="flex items-center gap-1">
                        <button 
                          class="p-1 text-gray-600 hover:text-blue-600 transition-colors"
                          title="Edit permission"
                        >
                          ✏️
                        </button>
                        
                        {#if !permission.isSystem}
                          <button 
                            class="p-1 text-gray-600 hover:text-red-600 transition-colors"
                            title="Delete permission"
                          >
                            🗑️
                          </button>
                        {/if}
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Create Permission Modal (placeholder) -->
{#if showCreateModal}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg shadow-xl p-6 w-full max-w-md">
      <h3 class="text-lg font-medium text-gray-900 mb-4">Create Permission</h3>
      <p class="text-gray-600 mb-4">Permission creation functionality coming soon.</p>
      <div class="flex justify-end">
        <button 
          class="btn-secondary"
          on:click={() => showCreateModal = false}
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}