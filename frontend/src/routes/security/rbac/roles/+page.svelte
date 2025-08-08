<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import RoleCard from '$lib/components/security/RoleCard.svelte';
  import RoleEditor from '$lib/components/security/RoleEditor.svelte';
  import RoleHierarchyVisualization from '$lib/components/security/RoleHierarchyVisualization.svelte';
  import type { Role, CreateRole, Permission } from '$lib/types/security';
  
  // State management
  let roles: Role[] = [];
  let permissions: Permission[] = [];
  let loading = true;
  let error = '';
  let searchQuery = '';
  let filterStatus: 'all' | 'active' | 'inactive' = 'all';
  let sortBy: 'name' | 'created' | 'users' | 'permissions' = 'name';
  let sortOrder: 'asc' | 'desc' = 'asc';
  let viewMode: 'cards' | 'hierarchy' | 'table' = 'cards';
  
  // Modal states
  let showRoleEditor = false;
  let showHierarchyView = false;
  let editingRole: Role | null = null;
  
  // Selection and bulk operations
  let selectedRoles = new Set<string>();
  let showBulkActions = false;
  let bulkOperationInProgress = false;
  
  // Pagination
  let currentPage = 1;
  let itemsPerPage = 12;
  
  // Statistics
  $: roleStats = calculateRoleStats(roles);
  $: filteredRoles = filterAndSortRoles(roles, searchQuery, filterStatus, sortBy, sortOrder);
  $: paginatedRoles = paginateRoles(filteredRoles, currentPage, itemsPerPage);
  $: totalPages = Math.ceil(filteredRoles.length / itemsPerPage);
  
  function calculateRoleStats(roles: Role[]) {
    return {
      total: roles.length,
      active: roles.filter(r => r.active).length,
      inactive: roles.filter(r => !r.active).length,
      withParent: roles.filter(r => r.parentRoleId).length,
      orphaned: roles.filter(r => r.parentRoleId && !roles.find(p => p.id === r.parentRoleId)).length
    };
  }
  
  function filterAndSortRoles(
    roles: Role[],
    query: string,
    status: string,
    sortBy: string,
    sortOrder: string
  ): Role[] {
    let filtered = [...roles];
    
    // Apply search filter
    if (query.trim()) {
      const lowerQuery = query.toLowerCase();
      filtered = filtered.filter(role =>
        role.name.toLowerCase().includes(lowerQuery) ||
        role.description?.toLowerCase().includes(lowerQuery) ||
        role.permissions?.some(p => p.toLowerCase().includes(lowerQuery))
      );
    }
    
    // Apply status filter
    if (status !== 'all') {
      filtered = filtered.filter(role => 
        status === 'active' ? role.active : !role.active
      );
    }
    
    // Apply sorting
    filtered.sort((a, b) => {
      let comparison = 0;
      
      switch (sortBy) {
        case 'name':
          comparison = a.name.localeCompare(b.name);
          break;
        case 'created':
          comparison = new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
          break;
        case 'users':
          // This would need user data - simplified for now
          comparison = 0;
          break;
        case 'permissions':
          comparison = (a.permissions?.length || 0) - (b.permissions?.length || 0);
          break;
      }
      
      return sortOrder === 'asc' ? comparison : -comparison;
    });
    
    return filtered;
  }
  
  function paginateRoles(roles: Role[], page: number, itemsPerPage: number): Role[] {
    const startIndex = (page - 1) * itemsPerPage;
    return roles.slice(startIndex, startIndex + itemsPerPage);
  }
  
  // Load data
  async function loadRoles() {
    try {
      loading = true;
      error = '';
      const [rolesData, permissionsData] = await Promise.all([
        securityApi.getRoles(),
        securityApi.getPermissions()
      ]);
      roles = rolesData;
      permissions = permissionsData;
    } catch (err) {
      console.error('Failed to load roles:', err);
      error = `Failed to load roles: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Role operations
  async function deleteRole(roleId: string) {
    const role = roles.find(r => r.id === roleId);
    const childRoles = roles.filter(r => r.parentRoleId === roleId);
    
    let confirmMessage = `Are you sure you want to delete the role "${role?.name}"?`;
    if (childRoles.length > 0) {
      confirmMessage += `\n\nThis role has ${childRoles.length} child role(s) that will become orphaned.`;
    }
    confirmMessage += '\n\nThis action cannot be undone.';
    
    if (!confirm(confirmMessage)) {
      return;
    }
    
    try {
      await securityApi.deleteRole(roleId);
      await loadRoles();
    } catch (err) {
      alert(`Failed to delete role: ${err}`);
    }
  }
  
  async function toggleRoleStatus(role: Role) {
    try {
      await securityApi.updateRole(role.id, {
        active: !role.active
      });
      await loadRoles();
    } catch (err) {
      alert(`Failed to update role status: ${err}`);
    }
  }
  
  async function duplicateRole(role: Role) {
    const duplicatedRole: CreateRole = {
      name: `${role.name} (Copy)`,
      description: role.description ? `${role.description} (Copy)` : '',
      permissions: [...(role.permissions || [])],
      parentRoleId: role.parentRoleId,
      active: false // Start inactive
    };
    
    try {
      await securityApi.createRole(duplicatedRole);
      await loadRoles();
    } catch (err) {
      alert(`Failed to duplicate role: ${err}`);
    }
  }
  
  // Modal handlers
  function openRoleEditor(role?: Role) {
    editingRole = role || null;
    showRoleEditor = true;
  }
  
  function closeRoleEditor() {
    showRoleEditor = false;
    editingRole = null;
  }
  
  async function handleRoleSave(event: CustomEvent<CreateRole>) {
    try {
      if (editingRole) {
        await securityApi.updateRole(editingRole.id, event.detail);
      } else {
        await securityApi.createRole(event.detail);
      }
      
      await loadRoles();
      closeRoleEditor();
    } catch (err) {
      alert(`Failed to save role: ${err}`);
    }
  }
  
  // Selection management
  function toggleRoleSelection(roleId: string) {
    const newSelected = new Set(selectedRoles);
    if (newSelected.has(roleId)) {
      newSelected.delete(roleId);
    } else {
      newSelected.add(roleId);
    }
    selectedRoles = newSelected;
    showBulkActions = selectedRoles.size > 0;
  }
  
  function selectAllRoles() {
    selectedRoles = new Set(filteredRoles.map(r => r.id));
    showBulkActions = true;
  }
  
  function clearSelection() {
    selectedRoles = new Set();
    showBulkActions = false;
  }
  
  // Bulk operations
  async function performBulkOperation(operation: 'activate' | 'deactivate' | 'delete') {
    if (selectedRoles.size === 0) return;
    
    const roleIds = Array.from(selectedRoles);
    const confirmMessage = 
      operation === 'delete' 
        ? `Are you sure you want to delete ${roleIds.length} roles? This action cannot be undone.`
        : `Are you sure you want to ${operation} ${roleIds.length} roles?`;
    
    if (!confirm(confirmMessage)) return;
    
    try {
      bulkOperationInProgress = true;
      
      const operations: any = {};
      if (operation === 'activate') operations.activate = roleIds;
      else if (operation === 'deactivate') operations.deactivate = roleIds;
      else if (operation === 'delete') operations.delete = roleIds;
      
      const result = await securityApi.bulkUpdateRoles(operations);
      
      if (result.failed > 0) {
        alert(`Bulk operation completed with ${result.failed} failures:\n${result.errors.join('\n')}`);
      }
      
      await loadRoles();
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
    loadRoles();
  });
</script>

<div class="space-y-6">
  <!-- Header Section -->
  <div class="security-card">
    <div class="security-card-header">
      <div>
        <h2 class="security-card-title">Role Management</h2>
        <p class="text-sm text-gray-600 mt-1">
          Create and manage roles with hierarchical permissions
        </p>
      </div>
      
      <div class="flex items-center gap-3">
        <button 
          class="btn-secondary"
          on:click={() => showHierarchyView = !showHierarchyView}
        >
          {showHierarchyView ? '📋 List View' : '🌳 Hierarchy View'}
        </button>
        
        <button 
          class="btn-primary"
          on:click={() => openRoleEditor()}
        >
          ➕ Create Role
        </button>
      </div>
    </div>

    <!-- Statistics Cards -->
    <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-5 gap-4 mt-6">
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-900">{roleStats.total}</div>
        <div class="text-sm text-gray-600">Total Roles</div>
      </div>
      
      <div class="bg-green-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-green-700">{roleStats.active}</div>
        <div class="text-sm text-green-600">Active</div>
      </div>
      
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-600">{roleStats.inactive}</div>
        <div class="text-sm text-gray-600">Inactive</div>
      </div>
      
      <div class="bg-blue-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-blue-700">{roleStats.withParent}</div>
        <div class="text-sm text-blue-600">Child Roles</div>
      </div>
      
      <div class="bg-orange-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-orange-700">{roleStats.orphaned}</div>
        <div class="text-sm text-orange-600">Orphaned</div>
      </div>
    </div>
  </div>

  <!-- Hierarchy View -->
  {#if showHierarchyView}
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Role Hierarchy</h3>
      </div>
      <RoleHierarchyVisualization 
        {roles} 
        on:edit={(e) => openRoleEditor(e.detail)}
        on:delete={(e) => deleteRole(e.detail.id)}
        on:toggle={(e) => toggleRoleStatus(e.detail)}
        on:duplicate={(e) => duplicateRole(e.detail)}
      />
    </div>
  {:else}
    <!-- Filters and Search -->
    <div class="security-card">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- Search -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Search Roles</label>
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search by name, description, permissions..."
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
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
            <option value="created">Created Date</option>
            <option value="permissions">Permission Count</option>
          </select>
        </div>

        <!-- View Options -->
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
              {selectedRoles.size} roles selected
            </span>
            
            <div class="flex items-center gap-2">
              <button
                class="text-sm text-blue-700 hover:text-blue-900 underline"
                on:click={selectAllRoles}
              >
                Select All ({filteredRoles.length})
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

    <!-- Roles List -->
    <div class="space-y-4">
      {#if loading}
        <div class="security-card">
          <div class="flex items-center justify-center py-12">
            <div class="flex items-center gap-3 text-gray-600">
              <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
              <span>Loading roles...</span>
            </div>
          </div>
        </div>
      {:else if error}
        <div class="security-card">
          <div class="text-center py-12">
            <div class="text-red-600 mb-4">❌</div>
            <h3 class="text-lg font-medium text-gray-900 mb-2">Failed to Load Roles</h3>
            <p class="text-gray-600 mb-4">{error}</p>
            <button class="btn-primary" on:click={loadRoles}>
              🔄 Retry
            </button>
          </div>
        </div>
      {:else if paginatedRoles.length === 0}
        <div class="security-card">
          <div class="text-center py-12">
            <div class="text-gray-400 mb-4 text-4xl">👥</div>
            <h3 class="text-lg font-medium text-gray-900 mb-2">
              {searchQuery || filterStatus !== 'all' 
                ? 'No Roles Match Your Filters' 
                : 'No Roles Created Yet'}
            </h3>
            <p class="text-gray-600 mb-4">
              {searchQuery || filterStatus !== 'all'
                ? 'Try adjusting your search criteria or filters'
                : 'Get started by creating your first role'}
            </p>
            <button class="btn-primary" on:click={() => openRoleEditor()}>
              ➕ Create First Role
            </button>
          </div>
        </div>
      {:else}
        <!-- Role Cards Grid -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {#each paginatedRoles as role}
            <RoleCard
              {role}
              selected={selectedRoles.has(role.id)}
              on:select={() => toggleRoleSelection(role.id)}
              on:edit={() => openRoleEditor(role)}
              on:delete={() => deleteRole(role.id)}
              on:toggle={() => toggleRoleStatus(role)}
              on:duplicate={() => duplicateRole(role)}
            />
          {/each}
        </div>

        <!-- Pagination -->
        {#if totalPages > 1}
          <div class="security-card">
            <div class="flex items-center justify-between">
              <div class="text-sm text-gray-600">
                Showing {((currentPage - 1) * itemsPerPage) + 1} to {Math.min(currentPage * itemsPerPage, filteredRoles.length)} of {filteredRoles.length} roles
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
  {/if}
</div>

<!-- Role Editor Modal -->
{#if showRoleEditor}
  <RoleEditor
    role={editingRole}
    availableRoles={roles.filter(r => r.id !== editingRole?.id)}
    availablePermissions={permissions}
    on:save={handleRoleSave}
    on:cancel={closeRoleEditor}
  />
{/if}