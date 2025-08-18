<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { Role, User, RoleStatistics } from '$lib/types/security';
  
  // State management
  let roles: Role[] = [];
  let users: User[] = [];
  let roleStats: RoleStatistics | null = null;
  let loading = true;
  let error = '';
  let lastUpdated: Date | null = null;
  let rbacEnabled = false;
  let configLoading = true;
  
  // UI state
  let viewMode: 'overview' | 'roles' | 'users' = 'overview';
  
  // Check if RBAC is enabled
  async function checkRBACConfig() {
    try {
      configLoading = true;
      const config = await securityApi.getSecurityConfig();
      rbacEnabled = config.rbac?.enabled || false;
    } catch (err) {
      console.error('Failed to load security config:', err);
      rbacEnabled = false;
    } finally {
      configLoading = false;
    }
  }
  
  // Load RBAC data
  async function loadRBACData() {
    try {
      loading = true;
      error = '';
      
      // Check if RBAC is enabled first
      if (!rbacEnabled) {
        loading = false;
        return;
      }
      
      // Only load roles for now (the only implemented endpoint)
      const rolesData = await securityApi.getRoles();
      roles = rolesData;
      
      // Try to load other data but don't fail if endpoints don't exist yet
      try {
        const usersData = await securityApi.getUsers();
        users = usersData;
      } catch (err) {
        console.log('Users endpoint not available yet:', err);
        users = []; // Mock some users for demo
      }
      
      try {
        const statsData = await securityApi.getRoleStatistics();
        roleStats = statsData;
      } catch (err) {
        console.log('Statistics endpoint not available yet:', err);
        // Generate mock statistics from available data
        roleStats = {
          totalRoles: roles.length,
          activeRoles: roles.filter(r => r.active).length,
          totalUsers: users.length,
          activeUsers: users.filter(u => u.active).length,
          averagePermissionsPerRole: roles.length > 0 
            ? roles.reduce((sum, role) => sum + (role.permissions?.length || 0), 0) / roles.length 
            : 0
        };
      }
      
      lastUpdated = new Date();
    } catch (err) {
      console.error('Failed to load RBAC data:', err);
      error = `Failed to load RBAC data: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Calculate derived statistics
  $: derivedStats = calculateDerivedStats(roles, users);
  
  function calculateDerivedStats(roles: Role[], users: User[]) {
    const totalPermissions = roles.reduce((sum, role) => sum + (role.permissions?.length || 0), 0);
    const activeRoles = roles.filter(r => r.active).length;
    const activeUsers = users.filter(u => u.active).length;
    const usersWithMultipleRoles = users.filter(u => u.roles && u.roles.length > 1).length;
    
    // Role usage analysis
    const roleUsage = roles.map(role => ({
      ...role,
      userCount: users.filter(u => u.roles?.includes(role.id)).length
    })).sort((a, b) => b.userCount - a.userCount);
    
    const mostUsedRole = roleUsage[0];
    const unusedRoles = roleUsage.filter(r => r.userCount === 0).length;
    
    return {
      totalPermissions,
      activeRoles,
      activeUsers,
      usersWithMultipleRoles,
      mostUsedRole,
      unusedRoles,
      roleUsage
    };
  }
  
  // Get role hierarchy levels
  function getRoleHierarchy(roles: Role[]): { level: number; roles: Role[] }[] {
    const hierarchy: { level: number; roles: Role[] }[] = [];
    const processed = new Set<string>();
    
    // Find root roles (no parent)
    const rootRoles = roles.filter(r => !r.parentRoleId);
    hierarchy.push({ level: 0, roles: rootRoles });
    rootRoles.forEach(r => processed.add(r.id));
    
    let level = 1;
    while (processed.size < roles.length && level < 10) { // Safety limit
      const currentLevelRoles = roles.filter(r => 
        !processed.has(r.id) && 
        r.parentRoleId && 
        processed.has(r.parentRoleId)
      );
      
      if (currentLevelRoles.length === 0) break;
      
      hierarchy.push({ level, roles: currentLevelRoles });
      currentLevelRoles.forEach(r => processed.add(r.id));
      level++;
    }
    
    return hierarchy;
  }
  
  // Navigation helpers
  function navigateToRoles() {
    window.location.href = '/security/rbac/roles';
  }
  
  function navigateToUsers() {
    window.location.href = '/security/rbac/users';
  }
  
  // Quick actions
  async function performQuickRoleAudit() {
    try {
      try {
        const auditResult = await securityApi.auditRoles();
        const issues = auditResult.issues || [];
        
        let message = `Role Audit Complete:\n\n`;
        message += `✅ Total Roles Audited: ${auditResult.totalRoles}\n`;
        message += `⚠️ Issues Found: ${issues.length}\n\n`;
        
        if (issues.length > 0) {
          message += `Issues:\n${issues.map(issue => `• ${issue.severity.toUpperCase()}: ${issue.description}`).join('\n')}`;
        } else {
          message += `No issues found. All roles are properly configured.`;
        }
        
        alert(message);
      } catch (err) {
        // If audit endpoint doesn't exist, perform a basic client-side audit
        console.log('Audit endpoint not available, performing basic audit:', err);
        
        const issues = [];
        
        // Basic validation
        const inactiveRoles = roles.filter(r => !r.active).length;
        const rolesWithoutPermissions = roles.filter(r => !r.permissions || r.permissions.length === 0).length;
        
        if (inactiveRoles > 0) {
          issues.push(`${inactiveRoles} inactive roles found`);
        }
        if (rolesWithoutPermissions > 0) {
          issues.push(`${rolesWithoutPermissions} roles without permissions found`);
        }
        
        let message = `Basic Role Audit Complete:\n\n`;
        message += `✅ Total Roles Audited: ${roles.length}\n`;
        message += `⚠️ Issues Found: ${issues.length}\n\n`;
        
        if (issues.length > 0) {
          message += `Issues:\n${issues.map(issue => `• ${issue}`).join('\n')}`;
        } else {
          message += `No issues found. All roles appear to be properly configured.`;
        }
        
        alert(message);
      }
    } catch (err) {
      alert(`Role audit failed: ${err}`);
    }
  }
  
  onMount(async () => {
    await checkRBACConfig();
    if (rbacEnabled) {
      loadRBACData();
      
      // Auto-refresh every 60 seconds
      const interval = setInterval(loadRBACData, 60000);
      
      return () => clearInterval(interval);
    }
  });
</script>

<div class="space-y-6">
  <!-- Config Loading State -->
  {#if configLoading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading configuration...</span>
        </div>
      </div>
    </div>
  {:else if !rbacEnabled}
    <!-- Disabled State -->
    <div class="security-card">
      <div class="text-center py-12">
        <div class="text-gray-400 mb-4">
          <svg class="h-12 w-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L5.636 5.636" />
          </svg>
        </div>
        <h3 class="text-lg font-medium text-gray-900 mb-2">RBAC Service Disabled</h3>
        <p class="text-gray-600 mb-4">
          Role-Based Access Control (RBAC) is currently disabled in the security configuration.
        </p>
        <div class="text-sm text-gray-500 mb-4">
          To enable RBAC, update the security configuration:
        </div>
        <div class="bg-gray-50 rounded-lg p-4 text-left max-w-md mx-auto">
          <code class="text-sm text-gray-800">
            security:<br/>
            &nbsp;&nbsp;rbac:<br/>
            &nbsp;&nbsp;&nbsp;&nbsp;enabled: true
          </code>
        </div>
        <div class="mt-4">
          <button class="btn-secondary" on:click={() => window.location.href = '/security/config'}>
            📝 Security Configuration
          </button>
        </div>
      </div>
    </div>
  {:else if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading RBAC data...</span>
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
        <h3 class="text-lg font-medium text-gray-900 mb-2">RBAC Data Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadRBACData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Role-Based Access Control (RBAC)</h2>
          <p class="text-sm text-gray-600 mt-1">
            Manage roles, permissions, and user access across your organization
          </p>
        </div>
        
        <div class="flex items-center gap-3">
          <button 
            class="btn-secondary"
            on:click={performQuickRoleAudit}
          >
            🔍 Audit Roles
          </button>
          
          <button 
            class="btn-secondary"
            on:click={navigateToUsers}
          >
            👤 Manage Users
          </button>
          
          <button 
            class="btn-primary"
            on:click={navigateToRoles}
          >
            👥 Manage Roles
          </button>
        </div>
      </div>
      
      <!-- Last Updated -->
      <div class="text-xs text-gray-500">
        Last updated: {lastUpdated?.toLocaleString() || 'Never'}
      </div>
    </div>

    <!-- Statistics Overview -->
    <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
      <!-- Total Roles -->
      <div class="bg-blue-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-blue-700">{roles.length}</div>
        <div class="text-sm text-blue-600">Total Roles</div>
      </div>
      
      <!-- Active Roles -->
      <div class="bg-green-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-green-700">{derivedStats.activeRoles}</div>
        <div class="text-sm text-green-600">Active Roles</div>
      </div>
      
      <!-- Total Users -->
      <div class="bg-purple-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-purple-700">{users.length}</div>
        <div class="text-sm text-purple-600">Total Users</div>
      </div>
      
      <!-- Active Users -->
      <div class="bg-cyan-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-cyan-700">{derivedStats.activeUsers}</div>
        <div class="text-sm text-cyan-600">Active Users</div>
      </div>
      
      <!-- Total Permissions -->
      <div class="bg-orange-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-orange-700">{derivedStats.totalPermissions}</div>
        <div class="text-sm text-orange-600">Permissions</div>
      </div>
      
      <!-- Multi-Role Users -->
      <div class="bg-indigo-50 p-4 rounded-lg">
        <div class="text-2xl font-bold text-indigo-700">{derivedStats.usersWithMultipleRoles}</div>
        <div class="text-sm text-indigo-600">Multi-Role Users</div>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Role Overview -->
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Role Overview</h3>
          <button class="btn-sm btn-secondary" on:click={navigateToRoles}>
            View All →
          </button>
        </div>

        <!-- Role Hierarchy Visualization -->
        {#if roles.length > 0}
          {@const hierarchy = getRoleHierarchy(roles)}
          <div class="space-y-4">
            {#each hierarchy as level}
              <div class="flex items-center gap-4">
                <div class="flex-shrink-0 w-16 text-center">
                  <div class="text-xs text-gray-500">Level {level.level}</div>
                </div>
                
                <div class="flex-1">
                  <div class="flex flex-wrap gap-2">
                    {#each level.roles as role}
                      <div class="inline-flex items-center gap-2 px-3 py-2 bg-gray-100 rounded-lg">
                        <span class="text-sm font-medium text-gray-900">{role.name}</span>
                        <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {
                          role.active ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
                        }">
                          {role.active ? '✓' : '○'}
                        </span>
                        {#if role.permissions}
                          <span class="text-xs text-gray-500">
                            {role.permissions.length} perms
                          </span>
                        {/if}
                      </div>
                    {/each}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="text-center py-8">
            <div class="text-gray-400 mb-2 text-4xl">👥</div>
            <p class="text-gray-600">No roles configured yet</p>
            <button class="btn-primary mt-3" on:click={navigateToRoles}>
              Create First Role
            </button>
          </div>
        {/if}
      </div>

      <!-- User Overview -->
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">User Overview</h3>
          <button class="btn-sm btn-secondary" on:click={navigateToUsers}>
            View All →
          </button>
        </div>

        {#if users.length > 0}
          <!-- Recent Users -->
          <div class="space-y-3">
            {#each users.slice(0, 5) as user}
              <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 bg-blue-100 rounded-full flex items-center justify-center">
                    <span class="text-sm font-medium text-blue-700">
                      {user.name ? user.name.charAt(0).toUpperCase() : user.id.charAt(0).toUpperCase()}
                    </span>
                  </div>
                  
                  <div>
                    <div class="text-sm font-medium text-gray-900">
                      {user.name || user.id}
                    </div>
                    {#if user.email}
                      <div class="text-xs text-gray-600">{user.email}</div>
                    {/if}
                  </div>
                </div>
                
                <div class="flex items-center gap-2">
                  <!-- Role Count -->
                  {#if user.roles && user.roles.length > 0}
                    <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                      {user.roles.length} role{user.roles.length !== 1 ? 's' : ''}
                    </span>
                  {/if}
                  
                  <!-- Status -->
                  <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium {
                    user.active ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-600'
                  }">
                    {user.active ? '✅ Active' : '⏸️ Inactive'}
                  </span>
                </div>
              </div>
            {/each}
            
            {#if users.length > 5}
              <div class="text-center pt-2">
                <button class="text-sm text-blue-600 hover:text-blue-800" on:click={navigateToUsers}>
                  View {users.length - 5} more users →
                </button>
              </div>
            {/if}
          </div>
        {:else}
          <div class="text-center py-8">
            <div class="text-gray-400 mb-2 text-4xl">👤</div>
            <p class="text-gray-600">No users configured yet</p>
            <button class="btn-primary mt-3" on:click={navigateToUsers}>
              Add First User
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- Advanced Analytics -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Role Usage Analytics</h3>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Most Used Roles -->
        <div>
          <h4 class="text-sm font-medium text-gray-700 mb-3">Most Used Roles</h4>
          {#if derivedStats.roleUsage.length > 0}
            <div class="space-y-2">
              {#each derivedStats.roleUsage.slice(0, 5) as roleUsage}
                <div class="flex items-center justify-between p-2 bg-gray-50 rounded">
                  <span class="text-sm text-gray-900">{roleUsage.name}</span>
                  <span class="text-xs text-gray-600">{roleUsage.userCount} users</span>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-sm text-gray-600">No usage data available</p>
          {/if}
        </div>

        <!-- Role Health -->
        <div>
          <h4 class="text-sm font-medium text-gray-700 mb-3">Role Health</h4>
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-sm text-gray-600">Unused Roles</span>
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {
                derivedStats.unusedRoles === 0 ? 'bg-green-100 text-green-800' : 'bg-yellow-100 text-yellow-800'
              }">
                {derivedStats.unusedRoles}
              </span>
            </div>
            
            <div class="flex items-center justify-between">
              <span class="text-sm text-gray-600">Active Roles</span>
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800">
                {derivedStats.activeRoles}/{roles.length}
              </span>
            </div>
            
            <div class="flex items-center justify-between">
              <span class="text-sm text-gray-600">Multi-Role Users</span>
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                {Math.round((derivedStats.usersWithMultipleRoles / users.length) * 100) || 0}%
              </span>
            </div>
          </div>
        </div>

        <!-- Quick Insights -->
        <div>
          <h4 class="text-sm font-medium text-gray-700 mb-3">Quick Insights</h4>
          <div class="space-y-3">
            {#if derivedStats.mostUsedRole}
              <div class="p-3 bg-blue-50 rounded-lg">
                <div class="text-sm font-medium text-blue-900">Most Popular Role</div>
                <div class="text-xs text-blue-700 mt-1">
                  {derivedStats.mostUsedRole.name} ({derivedStats.mostUsedRole.userCount} users)
                </div>
              </div>
            {/if}
            
            {#if derivedStats.unusedRoles > 0}
              <div class="p-3 bg-yellow-50 rounded-lg">
                <div class="text-sm font-medium text-yellow-900">Cleanup Opportunity</div>
                <div class="text-xs text-yellow-700 mt-1">
                  {derivedStats.unusedRoles} unused roles can be reviewed
                </div>
              </div>
            {/if}
            
            {#if roleStats?.averagePermissionsPerRole}
              <div class="p-3 bg-gray-50 rounded-lg">
                <div class="text-sm font-medium text-gray-900">Avg Permissions/Role</div>
                <div class="text-xs text-gray-700 mt-1">
                  {Math.round(roleStats.averagePermissionsPerRole)} permissions
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Quick Actions</h3>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <button class="btn-secondary" on:click={performQuickRoleAudit}>
          🔍 Audit All Roles
        </button>
        
        <a href="/security/rbac/roles" class="btn-secondary text-center">
          ➕ Create New Role
        </a>
        
        <a href="/security/rbac/users" class="btn-secondary text-center">
          👤 Add New User
        </a>
        
        <button class="btn-secondary" on:click={loadRBACData}>
          🔄 Refresh Data
        </button>
      </div>
    </div>
  {/if}
</div>