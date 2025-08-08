<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  
  // State management
  let config: any = null;
  let loading = true;
  let error = '';
  let saving = false;
  let validationResult: any = null;
  
  // Configuration sections
  let activeSection = 'global';
  const sections = [
    { id: 'global', name: 'Global Settings', icon: '🌐' },
    { id: 'allowlist', name: 'Allowlisting', icon: '✅' },
    { id: 'rbac', name: 'RBAC', icon: '👥' },
    { id: 'audit', name: 'Audit Logging', icon: '📝' },
    { id: 'sanitization', name: 'Sanitization', icon: '🧹' }
  ];
  
  // Load security configuration
  async function loadConfiguration() {
    try {
      loading = true;
      error = '';
      
      config = await securityApi.getSecurityConfig();
    } catch (err) {
      console.error('Failed to load security configuration:', err);
      error = `Failed to load security configuration: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Save configuration
  async function saveConfiguration() {
    try {
      saving = true;
      error = '';
      
      const updatedConfig = await securityApi.updateSecurityConfig(config);
      config = updatedConfig;
      
      alert('Security configuration saved successfully!');
    } catch (err) {
      console.error('Failed to save configuration:', err);
      error = `Failed to save configuration: ${err}`;
    } finally {
      saving = false;
    }
  }
  
  // Validate configuration
  async function validateConfiguration() {
    try {
      validationResult = await securityApi.validateSecurityConfig(config);
    } catch (err) {
      console.error('Failed to validate configuration:', err);
      error = `Failed to validate configuration: ${err}`;
    }
  }
  
  // Generate configuration for security level
  async function generateConfiguration(level: string) {
    try {
      const generatedConfig = await securityApi.generateSecurityConfig(level);
      config = generatedConfig;
      
      alert(`Configuration generated for ${level} security level`);
    } catch (err) {
      console.error('Failed to generate configuration:', err);
      error = `Failed to generate configuration: ${err}`;
    }
  }
  
  // Export configuration
  async function exportConfiguration(format: string) {
    try {
      const blob = await securityApi.exportSecurityConfig(format);
      const url = window.URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `security-config.${format}`;
      document.body.appendChild(a);
      a.click();
      window.URL.revokeObjectURL(url);
      document.body.removeChild(a);
    } catch (err) {
      console.error('Failed to export configuration:', err);
      error = `Failed to export configuration: ${err}`;
    }
  }
  
  // Import configuration
  async function importConfiguration(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    
    try {
      const content = await file.text();
      const format = file.name.endsWith('.json') ? 'json' : 'yaml';
      
      const result = await securityApi.importSecurityConfig(content, format);
      
      if (result.success) {
        alert(`Configuration imported successfully! ${result.imported} settings imported.`);
        await loadConfiguration();
      } else {
        alert(`Import completed with ${result.errors.length} errors.`);
      }
    } catch (err) {
      console.error('Failed to import configuration:', err);
      error = `Failed to import configuration: ${err}`;
    }
  }
  
  onMount(() => {
    loadConfiguration();
  });
</script>

<div class="space-y-6">
  <!-- Actions -->
  <div class="flex items-center justify-between mb-6">
    <p class="text-gray-600">Configure security settings and policies</p>
    <div class="flex gap-2">
      <button class="btn-secondary" on:click={() => exportConfiguration('yaml')}>
        ⬇️ Export YAML
      </button>
      <button class="btn-secondary" on:click={() => exportConfiguration('json')}>
        ⬇️ Export JSON
      </button>
      <label class="btn-secondary cursor-pointer">
        ⬆️ Import
        <input type="file" class="hidden" accept=".yaml,.yml,.json" on:change={importConfiguration} />
      </label>
    </div>
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
        <span>Loading security configuration...</span>
      </div>
    </div>
  {:else if config}
    <!-- Quick Actions -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Quick Configuration</h3>
      </div>
      
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <button class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 text-left" on:click={() => generateConfiguration('low')}>
          <h4 class="font-medium text-gray-900">🟢 Low Security</h4>
          <p class="text-sm text-gray-600 mt-1">Minimal restrictions, good for development</p>
        </button>
        
        <button class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 text-left" on:click={() => generateConfiguration('medium')}>
          <h4 class="font-medium text-gray-900">🟡 Medium Security</h4>
          <p class="text-sm text-gray-600 mt-1">Balanced security and usability</p>
        </button>
        
        <button class="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 text-left" on:click={() => generateConfiguration('high')}>
          <h4 class="font-medium text-gray-900">🔴 High Security</h4>
          <p class="text-sm text-gray-600 mt-1">Maximum security, enterprise-grade</p>
        </button>
      </div>
    </div>

    <!-- Configuration Editor -->
    <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
      <!-- Section Navigation -->
      <div class="lg:col-span-1">
        <div class="security-card">
          <h3 class="text-sm font-medium text-gray-700 mb-4">Configuration Sections</h3>
          <nav class="space-y-1">
            {#each sections as section}
              <button 
                class="w-full flex items-center px-3 py-2 text-sm font-medium rounded-md {
                  activeSection === section.id 
                    ? 'bg-blue-100 text-blue-700' 
                    : 'text-gray-600 hover:bg-gray-50'
                }"
                on:click={() => activeSection = section.id}
              >
                <span class="mr-3">{section.icon}</span>
                {section.name}
              </button>
            {/each}
          </nav>
        </div>
      </div>

      <!-- Configuration Form -->
      <div class="lg:col-span-3">
        <div class="security-card">
          <div class="security-card-header">
            <h3 class="security-card-title">
              {sections.find(s => s.id === activeSection)?.icon}
              {sections.find(s => s.id === activeSection)?.name}
            </h3>
            <div class="flex gap-2">
              <button class="btn-secondary" on:click={validateConfiguration}>
                ✅ Validate
              </button>
              <button class="btn-primary" on:click={saveConfiguration} disabled={saving}>
                {saving ? '💾 Saving...' : '💾 Save Changes'}
              </button>
            </div>
          </div>

          <!-- Global Settings -->
          {#if activeSection === 'global'}
            <div class="space-y-4">
              <div class="flex items-center">
                <input 
                  type="checkbox" 
                  bind:checked={config.global.enabled}
                  class="h-4 w-4 text-blue-600"
                />
                <label class="ml-2 block text-sm text-gray-900">Enable Security Framework</label>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700">Security Mode</label>
                <select bind:value={config.global.mode} class="mt-1 block w-full rounded-md border-gray-300">
                  <option value="permissive">Permissive</option>
                  <option value="balanced">Balanced</option>
                  <option value="strict">Strict</option>
                </select>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700">Log Level</label>
                <select bind:value={config.global.log_level} class="mt-1 block w-full rounded-md border-gray-300">
                  <option value="error">Error</option>
                  <option value="warn">Warning</option>
                  <option value="info">Info</option>
                  <option value="debug">Debug</option>
                </select>
              </div>
            </div>
          {/if}

          <!-- Allowlist Settings -->
          {#if activeSection === 'allowlist'}
            <div class="space-y-4">
              <div class="flex items-center">
                <input 
                  type="checkbox" 
                  bind:checked={config.allowlist.enabled}
                  class="h-4 w-4 text-blue-600"
                />
                <label class="ml-2 block text-sm text-gray-900">Enable Tool Allowlisting</label>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700">Default Action</label>
                <select bind:value={config.allowlist.default_action} class="mt-1 block w-full rounded-md border-gray-300">
                  <option value="allow">Allow</option>
                  <option value="warn">Warn</option>
                  <option value="deny">Deny</option>
                </select>
              </div>
            </div>
          {/if}

          <!-- RBAC Settings -->
          {#if activeSection === 'rbac'}
            <div class="space-y-4">
              <div class="flex items-center">
                <input 
                  type="checkbox" 
                  bind:checked={config.rbac.enabled}
                  class="h-4 w-4 text-blue-600"
                />
                <label class="ml-2 block text-sm text-gray-900">Enable Role-Based Access Control</label>
              </div>
              
              <div class="flex items-center">
                <input 
                  type="checkbox" 
                  bind:checked={config.rbac.require_authentication}
                  class="h-4 w-4 text-blue-600"
                />
                <label class="ml-2 block text-sm text-gray-900">Require Authentication</label>
              </div>
            </div>
          {/if}

          <!-- Audit Settings -->
          {#if activeSection === 'audit'}
            <div class="space-y-4">
              <div class="flex items-center">
                <input 
                  type="checkbox" 
                  bind:checked={config.audit.enabled}
                  class="h-4 w-4 text-blue-600"
                />
                <label class="ml-2 block text-sm text-gray-900">Enable Audit Logging</label>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700">Retention Days</label>
                <input 
                  type="number" 
                  bind:value={config.audit.retention_days}
                  class="mt-1 block w-full rounded-md border-gray-300"
                  min="1"
                  max="3650"
                />
              </div>
            </div>
          {/if}

          <!-- Sanitization Settings -->
          {#if activeSection === 'sanitization'}
            <div class="space-y-4">
              <div class="flex items-center">
                <input 
                  type="checkbox" 
                  bind:checked={config.sanitization.enabled}
                  class="h-4 w-4 text-blue-600"
                />
                <label class="ml-2 block text-sm text-gray-900">Enable Request Sanitization</label>
              </div>
              
              <div>
                <label class="block text-sm font-medium text-gray-700">Default Action</label>
                <select bind:value={config.sanitization.default_action} class="mt-1 block w-full rounded-md border-gray-300">
                  <option value="log">Log Only</option>
                  <option value="alert">Alert</option>
                  <option value="block">Block</option>
                </select>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- Validation Results -->
    {#if validationResult}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Validation Results</h3>
          <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {
            validationResult.valid ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
          }">
            {validationResult.valid ? '✅ Valid' : '❌ Invalid'}
          </span>
        </div>
        
        {#if validationResult.errors?.length > 0}
          <div class="mb-4">
            <h4 class="text-sm font-medium text-red-800 mb-2">Errors:</h4>
            <ul class="list-disc list-inside space-y-1">
              {#each validationResult.errors as error}
                <li class="text-sm text-red-600">{error}</li>
              {/each}
            </ul>
          </div>
        {/if}
        
        {#if validationResult.warnings?.length > 0}
          <div>
            <h4 class="text-sm font-medium text-yellow-800 mb-2">Warnings:</h4>
            <ul class="list-disc list-inside space-y-1">
              {#each validationResult.warnings as warning}
                <li class="text-sm text-yellow-600">{warning}</li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    {/if}
  {/if}
</div>