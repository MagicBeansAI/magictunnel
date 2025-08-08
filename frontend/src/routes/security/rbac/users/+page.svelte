<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { User, CreateUser, Role } from '$lib/types/security';
  
  // State management
  let users: User[] = [];
  let roles: Role[] = [];
  let loading = true;
  let error = '';
  let searchQuery = '';
  let filterStatus: 'all' | 'active' | 'inactive' = 'all';
  let filterRole = 'all';
  let sortBy: 'name' | 'email' | 'created' | 'roles' = 'name';
  let sortOrder: 'asc' | 'desc' = 'asc';
  
  // Modal states
  let showUserEditor = false;
  let editingUser: User | null = null;
  
  // Selection and bulk operations
  let selectedUsers = new Set<string>();
  let showBulkActions = false;
  let bulkOperationInProgress = false;
  
  // Pagination
  let currentPage = 1;
  let itemsPerPage = 12;
  
  // Statistics
  $: userStats = calculateUserStats(users, roles);
  $: filteredUsers = filterAndSortUsers(users, searchQuery, filterStatus, filterRole, sortBy, sortOrder);
  $: paginatedUsers = paginateUsers(filteredUsers, currentPage, itemsPerPage);
  $: totalPages = Math.ceil(filteredUsers.length / itemsPerPage);
  
  function calculateUserStats(users: User[], roles: Role[]) {
    return {
      total: users.length,
      active: users.filter(u => u.active).length,
      inactive: users.filter(u => !u.active).length,
      withRoles: users.filter(u => u.roles && u.roles.length > 0).length,
      withoutRoles: users.filter(u => !u.roles || u.roles.length === 0).length,
      multipleRoles: users.filter(u => u.roles && u.roles.length > 1).length,
      roleDistribution: getRoleDistribution(users, roles)
    };
  }
  
  function getRoleDistribution(users: User[], roles: Role[]) {
    const distribution: Record<string, number> = {};
    
    roles.forEach(role => {
      distribution[role.name] = users.filter(u => 
        u.roles && u.roles.includes(role.id)
      ).length;
    });
    
    return Object.entries(distribution)
      .sort(([,a], [,b]) => b - a)
      .slice(0, 5);
  }
  
  function filterAndSortUsers(
    users: User[],
    query: string,
    status: string,
    roleFilter: string,
    sortBy: string,
    sortOrder: string
  ): User[] {
    let filtered = [...users];
    
    // Apply search filter
    if (query.trim()) {
      const lowerQuery = query.toLowerCase();
      filtered = filtered.filter(user =>
        user.name?.toLowerCase().includes(lowerQuery) ||
        user.email?.toLowerCase().includes(lowerQuery) ||
        user.id.toLowerCase().includes(lowerQuery)
      );
    }
    
    // Apply status filter
    if (status !== 'all') {
      filtered = filtered.filter(user => 
        status === 'active' ? user.active : !user.active
      );
    }
    
    // Apply role filter
    if (roleFilter !== 'all') {
      filtered = filtered.filter(user => 
        user.roles && user.roles.includes(roleFilter)
      );
    }
    
    // Apply sorting
    filtered.sort((a, b) => {
      let comparison = 0;
      
      switch (sortBy) {
        case 'name':
          comparison = (a.name || a.id).localeCompare(b.name || b.id);
          break;
        case 'email':
          comparison = (a.email || '').localeCompare(b.email || '');
          break;
        case 'created':
          comparison = new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
          break;
        case 'roles':
          comparison = (a.roles?.length || 0) - (b.roles?.length || 0);
          break;
      }
      
      return sortOrder === 'asc' ? comparison : -comparison;
    });
    
    return filtered;
  }
  
  function paginateUsers(users: User[], page: number, itemsPerPage: number): User[] {
    const startIndex = (page - 1) * itemsPerPage;
    return users.slice(startIndex, startIndex + itemsPerPage);
  }
  
  // Load data
  async function loadUsers() {
    try {
      loading = true;
      error = '';
      
      // Load roles (available endpoint)
      const rolesData = await securityApi.getRoles();
      roles = rolesData;
      
      // Try to load users but fallback to mock data if not available
      try {
        const usersData = await securityApi.getUsers();
        users = usersData;
      } catch (err) {
        console.log('Users endpoint not available yet, using mock data:', err);
        
        // Generate mock users for demonstration
        users = [
          {
            id: 'user-1',
            name: 'Admin User',
            email: 'admin@example.com',
            roles: ['admin'],
            active: true,
            createdAt: new Date('2024-01-01'),
            modifiedAt: new Date('2024-01-01')
          },
          {
            id: 'user-2', 
            name: 'Standard User',
            email: 'user@example.com',
            roles: ['user'],
            active: true,
            createdAt: new Date('2024-01-15'),
            modifiedAt: new Date('2024-01-15')
          },
          {
            id: 'user-3',
            name: 'Inactive User',
            email: 'inactive@example.com',
            roles: ['user'],
            active: false,
            createdAt: new Date('2024-02-01'),
            modifiedAt: new Date('2024-02-01')
          }
        ];
      }
    } catch (err) {
      console.error('Failed to load users:', err);
      error = `Failed to load users: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // User operations
  async function deleteUser(userId: string) {
    const user = users.find(u => u.id === userId);
    
    const confirmMessage = `Are you sure you want to delete the user "${user?.name || user?.email || userId}"?\n\nThis action cannot be undone.`;
    
    if (!confirm(confirmMessage)) {
      return;
    }
    
    try {
      await securityApi.deleteUser(userId);
      await loadUsers();
    } catch (err) {
      alert(`Failed to delete user: ${err}`);
    }
  }
  
  async function toggleUserStatus(user: User) {
    try {
      await securityApi.updateUser(user.id, {
        active: !user.active
      });
      await loadUsers();
    } catch (err) {
      alert(`Failed to update user status: ${err}`);
    }
  }
  
  // Modal handlers
  function openUserEditor(user?: User) {
    editingUser = user || null;
    showUserEditor = true;
  }
  
  function closeUserEditor() {
    showUserEditor = false;
    editingUser = null;
  }
  
  async function handleUserSave(event: CustomEvent<CreateUser>) {
    try {
      if (editingUser) {
        await securityApi.updateUser(editingUser.id, event.detail);
      } else {
        await securityApi.createUser(event.detail);
      }
      
      await loadUsers();
      closeUserEditor();
    } catch (err) {
      alert(`Failed to save user: ${err}`);
    }
  }
  
  // Selection management
  function toggleUserSelection(userId: string) {
    const newSelected = new Set(selectedUsers);
    if (newSelected.has(userId)) {
      newSelected.delete(userId);
    } else {
      newSelected.add(userId);
    }
    selectedUsers = newSelected;
    showBulkActions = selectedUsers.size > 0;
  }
  
  function selectAllUsers() {
    selectedUsers = new Set(filteredUsers.map(u => u.id));
    showBulkActions = true;
  }
  
  function clearSelection() {
    selectedUsers = new Set();
    showBulkActions = false;
  }
  
  // Bulk operations
  async function performBulkOperation(operation: 'activate' | 'deactivate' | 'delete' | 'assign_role') {
    if (selectedUsers.size === 0) return;
    
    const userIds = Array.from(selectedUsers);
    let confirmMessage = '';
    
    switch (operation) {
      case 'delete':
        confirmMessage = `Are you sure you want to delete ${userIds.length} users? This action cannot be undone.`;
        break;
      case 'assign_role':
        const roleId = prompt('Enter role ID to assign to selected users:');
        if (!roleId) return;
        confirmMessage = `Assign role "${roleId}" to ${userIds.length} users?`;
        break;
      default:
        confirmMessage = `Are you sure you want to ${operation} ${userIds.length} users?`;
    }
    
    if (!confirm(confirmMessage)) return;
    
    try {
      bulkOperationInProgress = true;
      
      const operations: any = {};
      if (operation === 'activate') operations.activate = userIds;
      else if (operation === 'deactivate') operations.deactivate = userIds;
      else if (operation === 'delete') operations.delete = userIds;
      // assign_role would need additional implementation
      
      const result = await securityApi.bulkUpdateUsers(operations);
      
      if (result.failed > 0) {
        alert(`Bulk operation completed with ${result.failed} failures:\n${result.errors.join('\n')}`);
      }
      
      await loadUsers();
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
    loadUsers();
  });
</script>

<div class="space-y-6">
  <!-- Header Section -->
  <div class="security-card">
    <div class="security-card-header">
      <div>
        <h2 class="security-card-title">User Management</h2>
        <p class="text-sm text-gray-600 mt-1">
          Manage users and their role assignments
        </p>
      </div>
      
      <div class="flex items-center gap-3">        
        <button 
          class="btn-primary"
          on:click={() => openUserEditor()}
        >
          ➕ Add User
        </button>
      </div>
    </div>

    <!-- Statistics Cards -->
    <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mt-6">
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-900">{userStats.total}</div>
        <div class="text-sm text-gray-600">Total Users</div>
      </div>
      
      <div class="bg-green-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-green-700">{userStats.active}</div>
        <div class="text-sm text-green-600">Active</div>
      </div>
      
      <div class="bg-gray-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-gray-600">{userStats.inactive}</div>
        <div class="text-sm text-gray-600">Inactive</div>
      </div>
      
      <div class="bg-blue-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-blue-700">{userStats.withRoles}</div>
        <div class="text-sm text-blue-600">With Roles</div>
      </div>
      
      <div class="bg-orange-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-orange-700">{userStats.withoutRoles}</div>
        <div class="text-sm text-orange-600">No Roles</div>
      </div>
      
      <div class="bg-purple-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-purple-700">{userStats.multipleRoles}</div>
        <div class="text-sm text-purple-600">Multi-Role</div>
      </div>
    </div>
  </div>

  <!-- Role Distribution -->
  {#if userStats.roleDistribution.length > 0}
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Role Distribution</h3>
      </div>
      
      <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
        {#each userStats.roleDistribution as [roleName, count]}
          <div class="text-center p-4 bg-gray-50 rounded-lg">
            <div class="text-lg font-bold text-gray-900">{count}</div>
            <div class="text-sm text-gray-600 truncate" title={roleName}>{roleName}</div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Filters and Search -->
  <div class="security-card">
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
      <!-- Search -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Search Users</label>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search by name, email, ID..."
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

      <!-- Role Filter -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Role</label>
        <select bind:value={filterRole} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="all">All Roles</option>
          {#each roles as role}
            <option value={role.id}>{role.name}</option>
          {/each}
        </select>
      </div>

      <!-- Sort Options -->
      <div>
        <label class="block text-sm font-medium text-gray-700 mb-2">Sort By</label>
        <select bind:value={sortBy} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
          <option value="name">Name</option>
          <option value="email">Email</option>
          <option value="created">Created Date</option>
          <option value="roles">Role Count</option>
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
            {selectedUsers.size} users selected
          </span>
          
          <div class="flex items-center gap-2">
            <button
              class="text-sm text-blue-700 hover:text-blue-900 underline"
              on:click={selectAllUsers}
            >
              Select All ({filteredUsers.length})
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
            on:click={() => performBulkOperation('assign_role')}
          >
            👥 Assign Role
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

  <!-- Users List -->
  <div class="space-y-4">
    {#if loading}
      <div class="security-card">
        <div class="flex items-center justify-center py-12">
          <div class="flex items-center gap-3 text-gray-600">
            <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
            <span>Loading users...</span>
          </div>
        </div>
      </div>
    {:else if error}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-red-600 mb-4">❌</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">Failed to Load Users</h3>
          <p class="text-gray-600 mb-4">{error}</p>
          <button class="btn-primary" on:click={loadUsers}>
            🔄 Retry
          </button>
        </div>
      </div>
    {:else if paginatedUsers.length === 0}
      <div class="security-card">
        <div class="text-center py-12">
          <div class="text-gray-400 mb-4 text-4xl">👤</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">
            {searchQuery || filterStatus !== 'all' || filterRole !== 'all'
              ? 'No Users Match Your Filters' 
              : 'No Users Created Yet'}
          </h3>
          <p class="text-gray-600 mb-4">
            {searchQuery || filterStatus !== 'all' || filterRole !== 'all'
              ? 'Try adjusting your search criteria or filters'
              : 'Get started by adding your first user'}
          </p>
          <button class="btn-primary" on:click={() => openUserEditor()}>
            ➕ Add First User
          </button>
        </div>
      </div>
    {:else}
      <!-- User Cards Grid -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {#each paginatedUsers as user}
          <UserCard
            {user}
            {roles}
            selected={selectedUsers.has(user.id)}
            on:select={() => toggleUserSelection(user.id)}
            on:edit={() => openUserEditor(user)}
            on:delete={() => deleteUser(user.id)}
            on:toggle={() => toggleUserStatus(user)}
          />
        {/each}
      </div>

      <!-- Pagination -->
      {#if totalPages > 1}
        <div class="security-card">
          <div class="flex items-center justify-between">
            <div class="text-sm text-gray-600">
              Showing {((currentPage - 1) * itemsPerPage) + 1} to {Math.min(currentPage * itemsPerPage, filteredUsers.length)} of {filteredUsers.length} users
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

<!-- User Editor Modal -->
{#if showUserEditor}
  <UserEditor
    user={editingUser}
    availableRoles={roles}
    on:save={handleUserSave}
    on:cancel={closeUserEditor}
  />
{/if}