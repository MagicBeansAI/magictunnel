<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { SecurityStatus, SecurityAlert, SecurityMetric } from '$lib/types/security';
  
  // State management
  let securityStatus: SecurityStatus | null = null;
  let loading = true;
  let error = '';
  let lastUpdated: Date | null = null;

  // Load security dashboard data
  async function loadSecurityData() {
    try {
      loading = true;
      error = '';
      
      securityStatus = await securityApi.getSecurityStatus();
      lastUpdated = new Date();
    } catch (err) {
      console.error('Failed to load security data:', err);
      error = `Failed to load security data: ${err}`;
    } finally {
      loading = false;
    }
  }

  // Get component status display properties
  function getComponentStatusProps(enabled: boolean, status: string) {
    if (!enabled) {
      return { color: 'security-status-disabled', icon: '⏸️', text: 'Disabled' };
    }
    
    switch (status) {
      case 'healthy':
        return { color: 'security-status-healthy', icon: '✅', text: 'Healthy' };
      case 'warning':
        return { color: 'security-status-warning', icon: '⚠️', text: 'Warning' };
      case 'error':
        return { color: 'security-status-error', icon: '❌', text: 'Error' };
      default:
        return { color: 'security-status-disabled', icon: '❓', text: 'Unknown' };
    }
  }

  // Security metrics calculation
  $: securityMetrics = calculateSecurityMetrics(securityStatus);
  
  // Component status props
  $: allowlistProps = getComponentStatusProps(
    securityStatus?.components.allowlist.enabled || false,
    securityStatus?.components.allowlist.status || 'unknown'
  );
  $: rbacProps = getComponentStatusProps(
    securityStatus?.components.rbac.enabled || false,
    securityStatus?.components.rbac.status || 'unknown'
  );
  $: auditProps = getComponentStatusProps(
    securityStatus?.components.audit.enabled || false,
    securityStatus?.components.audit.status || 'unknown'
  );
  $: sanitizationProps = getComponentStatusProps(
    securityStatus?.components.sanitization.enabled || false,
    securityStatus?.components.sanitization.status || 'unknown'
  );
  $: policiesProps = getComponentStatusProps(
    securityStatus?.components.policies.enabled || false,
    securityStatus?.components.policies.status || 'unknown'
  );
  
  function calculateSecurityMetrics(status: SecurityStatus | null): SecurityMetric[] {
    if (!status) return [];
    
    return [
      {
        label: 'Active Rules',
        value: (status.components.allowlist.metrics?.rulesCount || 0) + 
               (status.components.sanitization.metrics?.policiesCount || 0) +
               (status.components.policies.metrics?.policiesCount || 0),
        format: 'number'
      },
      {
        label: 'Total Roles',
        value: status.components.rbac.metrics?.rolesCount || 0,
        format: 'number'
      },
      {
        label: 'Violations (24h)',
        value: status.violations?.last24Hours || 0,
        format: 'number',
        trend: status.violations?.last24Hours === 0 ? 'stable' : 'up'
      },
      {
        label: 'Audit Entries',
        value: status.components.audit.metrics?.entriesCount || 0,
        format: 'number'
      }
    ];
  }

  // Quick actions
  function navigateToComponent(component: string) {
    const routes: Record<string, string> = {
      allowlist: '/security/allowlist',
      rbac: '/security/rbac', 
      audit: '/security/audit',
      sanitization: '/security/sanitization',
      policies: '/security/policies',
      config: '/security/config'
    };
    
    const route = routes[component];
    if (route) {
      window.location.href = route;
    }
  }

  async function performQuickSecurityTest() {
    try {
      const testResult = await securityApi.testSecurity({
        tool: 'test_tool',
        user: 'test_user',
        roles: ['user']
      });
      
      alert(`Security Test Result:\n\nAllowed: ${testResult.overall.allowed}\nDecision: ${testResult.overall.decision}\nReason: ${testResult.overall.reason}`);
    } catch (err) {
      alert(`Security test failed: ${err}`);
    }
  }

  // Emergency Lockdown functionality
  async function triggerEmergencyLockdown() {
    const confirmed = confirm(
      'EMERGENCY LOCKDOWN\\n\\n' +
      'This will immediately:\\n' +
      '• Block all new tool requests\\n' +
      '• Enable maximum security restrictions\\n' +
      '• Log all current activities\\n' +
      '• Require admin intervention to restore\\n\\n' +
      'Are you sure you want to proceed?'
    );
    
    if (!confirmed) return;
    
    const adminConfirm = prompt(
      'Enter \"LOCKDOWN\" to confirm emergency lockdown:'
    );
    
    if (adminConfirm !== 'LOCKDOWN') {
      alert('Emergency lockdown cancelled - confirmation text did not match.');
      return;
    }
    
    try {
      // Call the emergency lockdown API
      const result = await securityApi.triggerEmergencyLockdown({
        reason: 'Manual emergency lockdown triggered from security dashboard',
        timestamp: new Date().toISOString(),
        triggeredBy: 'security_dashboard_user'
      });
      
      if (result.success) {
        alert(
          'EMERGENCY LOCKDOWN ACTIVATED\\n\\n' +
          `• Lockdown ID: ${result.lockdownId}\\n` +
          `• Status: ${result.status}\\n` +
          `• Active Restrictions: ${result.activeRestrictions.join(', ')}\\n\\n` +
          'Contact your security administrator to restore normal operations.'
        );
        
        // Refresh the security status to show lockdown state
        window.location.reload();
      } else {
        alert(`Emergency lockdown failed: ${result.error}`);
      }
    } catch (err) {
      console.error('Emergency lockdown failed:', err);
      alert(`Emergency lockdown failed: ${err}`);
    }
  }

  onMount(() => {
    loadSecurityData();
    
    // Auto-refresh every 30 seconds
    const interval = setInterval(loadSecurityData, 30000);
    
    return () => clearInterval(interval);
  });
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading security dashboard...</span>
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
        <h3 class="text-lg font-medium text-gray-900 mb-2">Security Dashboard Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadSecurityData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else if securityStatus}
    <!-- Emergency Quick Actions -->
    <div class="security-card bg-gradient-to-r from-red-50 to-orange-50 border-red-200">
      <div class="security-card-header">
        <h3 class="security-card-title text-red-800">🚨 Emergency Actions</h3>
        <div class="text-sm text-red-600">
          Critical security controls
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- Emergency Lockdown -->
        <button
          class="flex items-center justify-center px-4 py-3 bg-red-600 text-white font-medium rounded-lg hover:bg-red-700 transition-colors"
          title="Emergency security lockdown"
          on:click={triggerEmergencyLockdown}
        >
          <span class="mr-3 text-lg">🚨</span>
          <span>Emergency Lockdown</span>
        </button>
        
        <!-- Export Configuration -->
        <a
          href="/security/config"
          class="flex items-center justify-center px-4 py-3 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
          title="Export security configuration"
        >
          <span class="mr-3 text-lg">📤</span>
          <span>Export Config</span>
        </a>
      </div>
    </div>

    <!-- Security Overview Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      <!-- Overall Security Status -->
      <div class="security-card">
        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-sm font-medium text-gray-500">Security Status</h3>
            <div class="mt-2 flex items-center">
              <span class="text-2xl mr-2">
                {securityStatus.enabled ? '🔒' : '🔓'}
              </span>
              <div>
                <div class="text-lg font-semibold text-gray-900">
                  {securityStatus.enabled ? 'Enabled' : 'Disabled'}
                </div>
                <div class="text-sm text-gray-600">
                  {securityStatus.health?.overallStatus || 'Unknown'}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Security Metrics -->
      {#each securityMetrics as metric}
        <div class="security-card">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="text-sm font-medium text-gray-500">{metric.label}</h3>
              <div class="mt-2 flex items-baseline">
                <div class="text-2xl font-semibold text-gray-900">
                  {metric.value}
                </div>
                {#if metric.trend}
                  <span class="ml-2 text-sm font-medium {
                    metric.trend === 'up' ? 'text-red-600' :
                    metric.trend === 'down' ? 'text-green-600' :
                    'text-gray-600'
                  }">
                    {metric.trend === 'up' ? '↗️' : metric.trend === 'down' ? '↘️' : '➡️'}
                  </span>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Security Components Status -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Security Components</h3>
        <div class="text-sm text-gray-500">
          Last updated: {lastUpdated?.toLocaleTimeString() || 'Never'}
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <!-- Tool Allowlisting -->
        <button
          class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors text-left"
          on:click={() => navigateToComponent('allowlist')}
        >
          <div class="flex items-center justify-between mb-2">
            <h4 class="font-medium text-gray-900">Tool Allowlisting</h4>
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {allowlistProps.color}">
              {allowlistProps.icon} {allowlistProps.text}
            </span>
          </div>
          <p class="text-sm text-gray-600">
            Control which tools and resources can be accessed
          </p>
          {#if securityStatus.components.allowlist.metrics?.rulesCount}
            <div class="mt-2 text-xs text-gray-500">
              {securityStatus.components.allowlist.metrics.rulesCount} active rules
            </div>
          {/if}
        </button>

        <!-- RBAC -->
        <button
          class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors text-left"
          on:click={() => navigateToComponent('rbac')}
        >
          <div class="flex items-center justify-between mb-2">
            <h4 class="font-medium text-gray-900">Role-Based Access Control</h4>
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {rbacProps.color}">
              {rbacProps.icon} {rbacProps.text}
            </span>
          </div>
          <p class="text-sm text-gray-600">
            Manage roles and permissions for users and API keys
          </p>
          {#if securityStatus.components.rbac.metrics?.rolesCount}
            <div class="mt-2 text-xs text-gray-500">
              {securityStatus.components.rbac.metrics.rolesCount} roles configured
            </div>
          {/if}
        </button>

        <!-- Audit Logging -->
        <button
          class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors text-left"
          on:click={() => navigateToComponent('audit')}
        >
          <div class="flex items-center justify-between mb-2">
            <h4 class="font-medium text-gray-900">Audit Logging</h4>
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {auditProps.color}">
              {auditProps.icon} {auditProps.text}
            </span>
          </div>
          <p class="text-sm text-gray-600">
            Complete audit trail of all security events
          </p>
          {#if securityStatus.components.audit.metrics?.entriesCount}
            <div class="mt-2 text-xs text-gray-500">
              {securityStatus.components.audit.metrics.entriesCount} entries logged
            </div>
          {/if}
        </button>

        <!-- Request Sanitization -->
        <button
          class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors text-left"
          on:click={() => navigateToComponent('sanitization')}
        >
          <div class="flex items-center justify-between mb-2">
            <h4 class="font-medium text-gray-900">Request Sanitization</h4>
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {sanitizationProps.color}">
              {sanitizationProps.icon} {sanitizationProps.text}
            </span>
          </div>
          <p class="text-sm text-gray-600">
            Content filtering and secret detection
          </p>
          {#if securityStatus.components.sanitization.metrics?.policiesCount}
            <div class="mt-2 text-xs text-gray-500">
              {securityStatus.components.sanitization.metrics.policiesCount} active policies
            </div>
          {/if}
        </button>

        <!-- Security Policies -->
        <button
          class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors text-left"
          on:click={() => navigateToComponent('policies')}
        >
          <div class="flex items-center justify-between mb-2">
            <h4 class="font-medium text-gray-900">Security Policies</h4>
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {policiesProps.color}">
              {policiesProps.icon} {policiesProps.text}
            </span>
          </div>
          <p class="text-sm text-gray-600">
            Organization-wide policy enforcement
          </p>
          {#if securityStatus.components.policies.metrics?.policiesCount}
            <div class="mt-2 text-xs text-gray-500">
              {securityStatus.components.policies.metrics.policiesCount} active policies
            </div>
          {/if}
        </button>
      </div>
    </div>

    <!-- Security Violations Alert -->
    {#if securityStatus.violations?.last24Hours > 0}
      <div class="security-card bg-red-50 border-red-200">
        <div class="flex items-center justify-between">
          <div class="flex items-center">
            <div class="flex-shrink-0">
              <svg class="h-8 w-8 text-red-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.502 0L4.314 15.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
            </div>
            <div class="ml-3">
              <h3 class="text-lg font-medium text-red-800">Security Violations Detected</h3>
              <div class="mt-1 text-sm text-red-700">
                <p>{securityStatus.violations.last24Hours} violations in the last 24 hours</p>
                {#if securityStatus.violations.critical > 0}
                  <p class="font-medium">{securityStatus.violations.critical} critical violations require immediate attention</p>
                {/if}
              </div>
            </div>
          </div>
          <div class="flex-shrink-0">
            <a href="/security/audit/violations" class="btn-primary bg-red-600 hover:bg-red-700">
              🚨 Review Violations
            </a>
          </div>
        </div>
      </div>
    {:else}
      <div class="security-card bg-green-50 border-green-200">
        <div class="flex items-center">
          <div class="flex-shrink-0">
            <svg class="h-8 w-8 text-green-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div class="ml-3">
            <h3 class="text-lg font-medium text-green-800">All Clear</h3>
            <p class="mt-1 text-sm text-green-700">No security violations detected in the last 24 hours</p>
          </div>
        </div>
      </div>
    {/if}

    <!-- Quick Actions -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Quick Actions</h3>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <button class="btn-secondary" on:click={performQuickSecurityTest}>
          🧪 Run Security Test
        </button>
        
        <button class="btn-secondary" on:click={() => navigateToComponent('config')}>
          ⚙️ Configuration
        </button>
        
        <button class="btn-secondary" on:click={() => window.location.href = '/security/audit/search'}>
          🔍 Search Logs
        </button>
        
        <button class="btn-secondary" on:click={loadSecurityData}>
          🔄 Refresh Data
        </button>
      </div>
    </div>

    <!-- Security Health Issues -->
    {#if securityStatus.health?.issues && securityStatus.health.issues.length > 0}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Security Issues</h3>
          <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800">
            {securityStatus.health.issues.length} issues
          </span>
        </div>

        <div class="space-y-3">
          {#each securityStatus.health.issues as issue}
            <div class="flex items-start p-3 bg-gray-50 rounded-lg">
              <span class="flex-shrink-0 text-lg mr-3">
                {issue.severity === 'critical' ? '🚨' :
                 issue.severity === 'high' ? '⚠️' :
                 issue.severity === 'medium' ? '🔶' : 'ℹ️'}
              </span>
              <div class="flex-1">
                <div class="flex items-center justify-between">
                  <h4 class="text-sm font-medium text-gray-900">{issue.component.toUpperCase()}</h4>
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {
                    issue.severity === 'critical' ? 'bg-red-100 text-red-800' :
                    issue.severity === 'high' ? 'bg-orange-100 text-orange-800' :
                    issue.severity === 'medium' ? 'bg-yellow-100 text-yellow-800' :
                    'bg-blue-100 text-blue-800'
                  }">
                    {issue.severity}
                  </span>
                </div>
                <p class="text-sm text-gray-600 mt-1">{issue.message}</p>
                {#if issue.resolution}
                  <p class="text-xs text-gray-500 mt-2">
                    <strong>Resolution:</strong> {issue.resolution}
                  </p>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>