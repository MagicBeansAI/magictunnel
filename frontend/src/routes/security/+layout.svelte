<script lang="ts">
  import { page } from '$app/stores';
  
  // Get the current page title based on route
  $: pageTitle = getPageTitle($page.route?.id || '');
  
  function getPageTitle(routeId: string): string {
    const titles: Record<string, string> = {
      '/security': 'Security Overview',
      '/security/allowlist': 'Tool Allowlisting & Pattern Testing',
      '/security/rbac': 'Access Control',
      '/security/rbac/roles': 'Roles Management',
      '/security/rbac/users': 'Users Management',
      '/security/rbac/permissions': 'Permissions Management',
      '/security/audit': 'Audit Logging',
      '/security/audit/search': 'Audit Search',
      '/security/audit/violations': 'Security Violations',
      '/security/sanitization': 'Sanitization',
      '/security/sanitization/policies': 'Sanitization Policies',
      '/security/sanitization/filtering': 'Content Filtering',
      '/security/sanitization/secrets': 'Secret Detection',
      '/security/sanitization/testing': 'Sanitization Testing',
      '/security/config': 'Configuration',
    };
    
    return titles[routeId] || 'Security Management';
  }
</script>

<!-- Simplified Security Layout -->
<div class="min-h-screen bg-gray-50">
  <!-- Page Header -->
  <div class="bg-white border-b border-gray-200">
    <div class="px-6 py-4">
      <h1 class="text-2xl font-bold text-gray-900">{pageTitle}</h1>
    </div>
  </div>

  <!-- Page Content -->
  <main class="p-6">
    <slot />
  </main>
</div>

<style>
  /* Security-specific styling */
  :global(.security-card) {
    @apply bg-white border border-gray-200 rounded-lg p-6 shadow-sm;
  }
  
  :global(.security-card-header) {
    @apply flex items-center justify-between mb-4;
  }
  
  :global(.security-card-title) {
    @apply text-lg font-semibold text-gray-900;
  }
  
  :global(.security-status-healthy) {
    @apply bg-green-100 text-green-800;
  }
  
  :global(.security-status-warning) {
    @apply bg-yellow-100 text-yellow-800;
  }
  
  :global(.security-status-error) {
    @apply bg-red-100 text-red-800;
  }
  
  :global(.security-status-disabled) {
    @apply bg-gray-100 text-gray-600;
  }
</style>