<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type SystemConfig, type ConfigTemplates, type ConfigValidationResult, type ConfigBackupsResponse, type ConfigBackup, type ConfigSaveRequest, type ConfigSaveResponse, type EnvVarsResponse, type EnvVarInfo, type SetEnvVarsRequest, type DeleteEnvVarsRequest, type EnvVarsOperationResponse } from '$lib/api';

  let config: SystemConfig | null = null;
  let templates: ConfigTemplates | null = null;
  let loading = true;
  let error = '';
  let activeTab = 'current';
  let expandedSections: Set<string> = new Set();
  
  // Configuration Editor state
  let editorContent = '';
  let validationResult: ConfigValidationResult | null = null;
  let isValidating = false;
  let backups: ConfigBackup[] = [];
  let selectedBackup: string = '';
  let isLoadingBackups = false;
  let operationResult: string = '';
  
  // Save functionality state
  let isSaving = false;
  let saveResult: ConfigSaveResponse | null = null;
  let hasUnsavedChanges = false;
  let originalContent = '';
  
  // Restart functionality state
  let restartingMagicTunnel = false;
  let restartCountdown = 0;
  let restartResult: any = null;
  let showRestartDialog = false;
  let startupArgs = '--config magictunnel-config.yaml --log-level info';
  
  // Environment Variables state
  let envVars: EnvVarsResponse | null = null;
  let loadingEnvVars = false;
  let envVarsError = '';
  let includeSensitive = false;
  let envFilter = '';
  let selectedEnvFile = '.env.local';
  let newEnvVar = { name: '', value: '' };
  let editingEnvVar: EnvVarInfo | null = null;
  let showAddEnvVarDialog = false;
  let showEditEnvVarDialog = false;
  let envOperationResult = '';
  let selectedEnvVars: Set<string> = new Set();

  // Load configuration data
  async function loadConfigurationData() {
    loading = true;
    error = '';
    
    try {
      const [configData, templatesData] = await Promise.all([
        api.getConfig(),
        api.getConfigTemplates()
      ]);
      
      config = configData;
      templates = templatesData;
    } catch (err) {
      error = `Failed to load configuration: ${err}`;
      console.error('Configuration loading error:', err);
    } finally {
      loading = false;
    }
  }

  function toggleSection(sectionId: string) {
    if (expandedSections.has(sectionId)) {
      expandedSections.delete(sectionId);
    } else {
      expandedSections.add(sectionId);
    }
    expandedSections = new Set(expandedSections); // Trigger reactivity
  }

  function copyToClipboard(text: string, label: string = 'content') {
    navigator.clipboard.writeText(text).then(() => {
      alert(`${label} copied to clipboard!`);
    }).catch(err => {
      console.error('Failed to copy:', err);
      alert('Failed to copy to clipboard');
    });
  }

  function formatUptime(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
    return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
  }

  // Configuration Editor Functions
  async function validateConfiguration() {
    if (!editorContent.trim()) {
      alert('Please enter configuration content to validate');
      return;
    }
    
    isValidating = true;
    validationResult = null;
    
    try {
      validationResult = await api.validateConfig({
        content: editorContent,
        config_type: 'main'
      });
    } catch (err) {
      alert(`Validation failed: ${err}`);
    } finally {
      isValidating = false;
    }
  }

  async function createBackup() {
    try {
      const result = await api.backupConfig();
      operationResult = result.message;
      loadBackups(); // Refresh backup list
    } catch (err) {
      operationResult = `Backup failed: ${err}`;
    }
  }

  async function loadBackups() {
    isLoadingBackups = true;
    try {
      const result = await api.listConfigBackups();
      backups = result.backups;
    } catch (err) {
      console.error('Failed to load backups:', err);
    } finally {
      isLoadingBackups = false;
    }
  }

  async function restoreFromBackup() {
    if (!selectedBackup) {
      alert('Please select a backup to restore');
      return;
    }
    
    if (!confirm(`Are you sure you want to restore from backup: ${selectedBackup}?\n\nThis will overwrite the current configuration and require a restart.`)) {
      return;
    }
    
    try {
      const result = await api.restoreConfig({ backup_name: selectedBackup });
      operationResult = result.message;
      if (result.note) {
        operationResult += `\n\nNote: ${result.note}`;
      }
    } catch (err) {
      operationResult = `Restore failed: ${err}`;
    }
  }

  function loadCurrentConfig() {
    if (config?.config_files?.active_config?.content) {
      editorContent = config.config_files.active_config.content;
      originalContent = editorContent;
      hasUnsavedChanges = false;
      validationResult = null;
    } else {
      alert('No active configuration found to load');
    }
  }

  function loadTemplate() {
    if (templates?.main_config_template?.content) {
      editorContent = templates.main_config_template.content;
      hasUnsavedChanges = editorContent !== originalContent;
      validationResult = null;
    } else {
      alert('No template found to load');
    }
  }

  // Configuration save functions
  async function saveConfiguration() {
    if (!editorContent.trim()) {
      alert('Please enter configuration content to save');
      return;
    }

    if (!confirm('Are you sure you want to save the configuration?\n\nThis will overwrite the current configuration file and may require a restart to take effect.')) {
      return;
    }

    isSaving = true;
    saveResult = null;
    
    try {
      saveResult = await api.saveConfig({
        content: editorContent,
        config_path: 'magictunnel-config.yaml'
      });
      
      if (saveResult.success) {
        originalContent = editorContent;
        hasUnsavedChanges = false;
        operationResult = `Configuration saved successfully!\n\n${saveResult.message}`;
        if (saveResult.backup_created) {
          operationResult += `\nBackup created: ${saveResult.backup_created}`;
        }
        
        // Refresh backups list
        loadBackups();
      } else {
        operationResult = `Save failed: ${saveResult.message || 'Unknown error'}`;
      }
    } catch (err) {
      operationResult = `Save failed: ${err}`;
    } finally {
      isSaving = false;
    }
  }

  // Track editor changes
  function onEditorContentChange() {
    hasUnsavedChanges = editorContent !== originalContent;
  }

  // MagicTunnel restart functions
  function restartMagicTunnel() {
    // Show the restart confirmation dialog
    showRestartDialog = true;
  }

  function closeRestartDialog() {
    showRestartDialog = false;
  }

  async function confirmRestart() {
    showRestartDialog = false;
    restartingMagicTunnel = true;
    restartResult = null;
    
    try {
      // Parse startup args into array
      const args = startupArgs.trim().split(/\s+/).filter(arg => arg.length > 0);
      
      // Use custom restart with startup args
      const result = await api.customRestartMagicTunnel({
        start_args: args
      });
      
      restartResult = result;
      
      if (result.status === 'success') {
        showRestartCountdown();
      }
    } catch (err) {
      console.error('Restart failed:', err);
      restartResult = {
        action: 'restart_magictunnel',
        status: 'error',
        message: `Failed to restart: ${err}`,
        timestamp: new Date().toISOString()
      };
      restartingMagicTunnel = false;
    }
  }

  function showRestartCountdown() {
    restartCountdown = 30; // 30 second countdown
    const countdown = setInterval(() => {
      restartCountdown--;
      if (restartCountdown <= 0) {
        clearInterval(countdown);
        attemptReconnection();
      }
    }, 1000);
  }

  async function attemptReconnection() {
    let reconnectAttempts = 0;
    const maxAttempts = 12; // Try for 60 seconds (12 * 5 seconds)
    
    const tryReconnect = async () => {
      try {
        await api.getSystemStatus();
        // If successful, reconnection is complete
        restartingMagicTunnel = false;
        restartResult = {
          action: 'restart_magictunnel',
          status: 'success',
          message: 'MagicTunnel restarted successfully! Connection restored.',
          timestamp: new Date().toISOString()
        };
        // Refresh the configuration data after successful restart
        loadConfigurationData();
      } catch (err) {
        reconnectAttempts++;
        if (reconnectAttempts < maxAttempts) {
          setTimeout(tryReconnect, 5000); // Try again in 5 seconds
        } else {
          restartingMagicTunnel = false;
          restartResult = {
            action: 'restart_magictunnel',
            status: 'error',
            message: 'MagicTunnel restart completed, but unable to reconnect. Please check manually.',
            timestamp: new Date().toISOString()
          };
        }
      }
    };
    
    tryReconnect();
  }

  // ============================================================================
  // Environment Variables Functions
  // ============================================================================
  
  async function loadEnvVars() {
    loadingEnvVars = true;
    envVarsError = '';
    console.log('Loading environment variables...');
    
    try {
      const result = await api.getEnvVars({
        filter: envFilter || undefined,
        include_sensitive: includeSensitive
      });
      console.log('Environment variables loaded:', result);
      envVars = result;
    } catch (err) {
      envVarsError = `Failed to load environment variables: ${err}`;
      console.error('Environment variables loading error:', err);
    } finally {
      loadingEnvVars = false;
    }
  }

  async function addEnvVar() {
    if (!newEnvVar.name.trim() || !newEnvVar.value.trim()) {
      alert('Please provide both name and value for the environment variable');
      return;
    }

    try {
      const result = await api.setEnvVars({
        variables: { [newEnvVar.name]: newEnvVar.value },
        persist: true,
        env_file: selectedEnvFile
      });

      if (result.success) {
        envOperationResult = `Successfully added ${newEnvVar.name}`;
        newEnvVar = { name: '', value: '' };
        showAddEnvVarDialog = false;
        loadEnvVars();
      } else {
        envOperationResult = `Failed to add environment variable: ${result.errors.join(', ')}`;
      }
    } catch (err) {
      envOperationResult = `Error adding environment variable: ${err}`;
    }
  }

  async function updateEnvVar() {
    if (!editingEnvVar) return;

    try {
      const result = await api.setEnvVars({
        variables: { [editingEnvVar.name]: editingEnvVar.value },
        persist: true,
        env_file: selectedEnvFile
      });

      if (result.success) {
        envOperationResult = `Successfully updated ${editingEnvVar.name}`;
        editingEnvVar = null;
        showEditEnvVarDialog = false;
        loadEnvVars();
      } else {
        envOperationResult = `Failed to update environment variable: ${result.errors.join(', ')}`;
      }
    } catch (err) {
      envOperationResult = `Error updating environment variable: ${err}`;
    }
  }

  async function deleteSelectedEnvVars() {
    if (selectedEnvVars.size === 0) {
      alert('Please select environment variables to delete');
      return;
    }

    const varNames = Array.from(selectedEnvVars);
    if (!confirm(`Are you sure you want to delete ${varNames.length} environment variable(s)?\n\n${varNames.join(', ')}`)) {
      return;
    }

    try {
      const result = await api.deleteEnvVars({
        variables: varNames,
        persist: true,
        env_file: selectedEnvFile
      });

      if (result.success) {
        envOperationResult = `Successfully deleted ${varNames.length} environment variable(s)`;
        selectedEnvVars.clear();
        selectedEnvVars = selectedEnvVars; // Trigger reactivity
        loadEnvVars();
      } else {
        envOperationResult = `Failed to delete environment variables: ${result.errors.join(', ')}`;
      }
    } catch (err) {
      envOperationResult = `Error deleting environment variables: ${err}`;
    }
  }

  function startEditEnvVar(envVar: EnvVarInfo) {
    editingEnvVar = { ...envVar };
    showEditEnvVarDialog = true;
  }

  function toggleEnvVarSelection(varName: string) {
    if (selectedEnvVars.has(varName)) {
      selectedEnvVars.delete(varName);
    } else {
      selectedEnvVars.add(varName);
    }
    selectedEnvVars = selectedEnvVars; // Trigger reactivity
  }

  function selectAllEnvVars() {
    if (!envVars) return;
    selectedEnvVars = new Set(envVars.variables.filter(v => !v.is_sensitive || includeSensitive).map(v => v.name));
    selectedEnvVars = selectedEnvVars; // Trigger reactivity
  }

  function clearEnvVarSelection() {
    selectedEnvVars.clear();
    selectedEnvVars = selectedEnvVars; // Trigger reactivity
  }

  // Quick preset functions for common MagicTunnel environment variables
  function addOpenAIKey() {
    newEnvVar = { name: 'OPENAI_API_KEY', value: '' };
    showAddEnvVarDialog = true;
  }

  function addAnthropicKey() {
    newEnvVar = { name: 'ANTHROPIC_API_KEY', value: '' };
    showAddEnvVarDialog = true;
  }

  function addOllamaURL() {
    newEnvVar = { name: 'OLLAMA_BASE_URL', value: 'http://localhost:11434' };
    showAddEnvVarDialog = true;
  }

  function addMagicTunnelEnv() {
    newEnvVar = { name: 'MAGICTUNNEL_ENV', value: 'development' };
    showAddEnvVarDialog = true;
  }

  onMount(() => {
    loadConfigurationData();
  });
</script>

<div class="min-h-screen bg-gray-50">
  <div class="container mx-auto px-4 py-8">
    <!-- Header -->
    <header class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-4xl font-bold text-primary-700 mb-2">Configuration Viewer</h1>
          <p class="text-gray-600">System configuration, runtime status, and templates</p>
        </div>
        
        <div class="flex gap-2">
          <button 
            class="btn-secondary" 
            on:click={loadConfigurationData}
            disabled={loading}
          >
            {loading ? '🔄 Loading...' : '🔄 Refresh'}
          </button>
        </div>
      </div>
      
      {#if error}
        <div class="mt-4 p-3 bg-red-50 border border-red-200 rounded-lg">
          <div class="flex items-center">
            <span class="text-red-500 mr-2">❌</span>
            <span class="text-red-700">{error}</span>
          </div>
        </div>
      {/if}
    </header>

    {#if loading}
      <div class="text-center py-16">
        <div class="inline-block animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mb-4"></div>
        <div class="text-lg text-gray-600">Loading configuration data...</div>
      </div>
    {:else if config}
      <!-- Tab Navigation -->
      <div class="mb-8">
        <div class="border-b border-gray-200">
          <nav class="-mb-px flex space-x-8">
            <button 
              class="tab-nav {activeTab === 'current' ? 'tab-nav-active' : 'tab-nav-inactive'}"
              on:click={() => activeTab = 'current'}
            >
              🔧 Current Configuration
            </button>
            <button 
              class="tab-nav {activeTab === 'runtime' ? 'tab-nav-active' : 'tab-nav-inactive'}"
              on:click={() => activeTab = 'runtime'}
            >
              📊 Runtime Status
            </button>
            <button 
              class="tab-nav {activeTab === 'files' ? 'tab-nav-active' : 'tab-nav-inactive'}"
              on:click={() => activeTab = 'files'}
            >
              📄 Configuration Files
            </button>
            <button 
              class="tab-nav {activeTab === 'templates' ? 'tab-nav-active' : 'tab-nav-inactive'}"
              on:click={() => activeTab = 'templates'}
            >
              📖 Templates & Examples
            </button>
            <button 
              class="tab-nav {activeTab === 'editor' ? 'tab-nav-active' : 'tab-nav-inactive'}"
              on:click={() => { activeTab = 'editor'; loadBackups(); }}
            >
              ✏️ Editor & Management
            </button>
            <button 
              class="tab-nav {activeTab === 'env' ? 'tab-nav-active' : 'tab-nav-inactive'}"
              on:click={() => { activeTab = 'env'; loadEnvVars(); }}
            >
              🌍 Environment Variables
            </button>
          </nav>
        </div>
      </div>

      <!-- Tab Content -->
      {#if activeTab === 'current'}
        <!-- Current Configuration Tab -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- Server Configuration -->
          <div class="config-card">
            <div class="config-header">
              <h3 class="config-title">🌐 Server Configuration</h3>
            </div>
            <div class="config-content">
              <div class="config-row">
                <span class="config-label">Host:</span>
                <span class="config-value">{config.current_config.server.host}</span>
              </div>
              <div class="config-row">
                <span class="config-label">Port:</span>
                <span class="config-value">{config.current_config.server.port}</span>
              </div>
              <div class="config-row">
                <span class="config-label">WebSocket:</span>
                <span class="config-badge {config.current_config.server.websocket ? 'badge-success' : 'badge-error'}">
                  {config.current_config.server.websocket ? '✅ Enabled' : '❌ Disabled'}
                </span>
              </div>
              <div class="config-row">
                <span class="config-label">Timeout:</span>
                <span class="config-value">{config.current_config.server.timeout}s</span>
              </div>
              <div class="config-row">
                <span class="config-label">TLS Mode:</span>
                <span class="config-value">{config.current_config.server.tls.mode}</span>
              </div>
            </div>
          </div>

          <!-- Registry Configuration -->
          <div class="config-card">
            <div class="config-header">
              <h3 class="config-title">📋 Registry Configuration</h3>
            </div>
            <div class="config-content">
              <div class="config-row">
                <span class="config-label">Type:</span>
                <span class="config-value">{config.current_config.registry.type}</span>
              </div>
              <div class="config-row">
                <span class="config-label">Hot Reload:</span>
                <span class="config-badge {config.current_config.registry.hot_reload ? 'badge-success' : 'badge-error'}">
                  {config.current_config.registry.hot_reload ? '✅ Enabled' : '❌ Disabled'}
                </span>
              </div>
              <div class="config-row">
                <span class="config-label">Validation:</span>
                <span class="config-badge {config.current_config.registry.validation.strict ? 'badge-warning' : 'badge-info'}">
                  {config.current_config.registry.validation.strict ? '🔒 Strict' : '🔓 Relaxed'}
                </span>
              </div>
              <div class="config-row">
                <span class="config-label">Paths:</span>
                <div class="config-list">
                  {#each config.current_config.registry.paths as path}
                    <span class="config-list-item">{path}</span>
                  {/each}
                </div>
              </div>
            </div>
          </div>

          <!-- Smart Discovery Configuration -->
          <div class="config-card">
            <div class="config-header">
              <h3 class="config-title">🧠 Smart Discovery</h3>
            </div>
            <div class="config-content">
              <div class="config-row">
                <span class="config-label">Enabled:</span>
                <span class="config-badge {config.current_config.smart_discovery.enabled ? 'badge-success' : 'badge-error'}">
                  {config.current_config.smart_discovery.enabled ? '✅ Active' : '❌ Inactive'}
                </span>
              </div>
              <div class="config-row">
                <span class="config-label">Mode:</span>
                <span class="config-value">{config.current_config.smart_discovery.tool_selection_mode}</span>
              </div>
              <div class="config-row">
                <span class="config-label">Confidence:</span>
                <span class="config-value">{config.current_config.smart_discovery.default_confidence_threshold}</span>
              </div>
              <div class="config-row">
                <span class="config-label">LLM Provider:</span>
                <span class="config-value">{config.current_config.smart_discovery.llm_tool_selection.provider}</span>
              </div>
              <div class="config-row">
                <span class="config-label">Model:</span>
                <span class="config-value">{config.current_config.smart_discovery.llm_tool_selection.model}</span>
              </div>
            </div>
          </div>

          <!-- External MCP Configuration -->
          <div class="config-card">
            <div class="config-header">
              <h3 class="config-title">🔗 External MCP</h3>
              <button 
                class="btn-sm-secondary"
                on:click={() => toggleSection('external_mcp_content')}
              >
                {expandedSections.has('external_mcp_content') ? 'Hide Config' : 'Show Config'}
              </button>
            </div>
            <div class="config-content">
              <div class="config-row">
                <span class="config-label">Enabled:</span>
                <span class="config-badge {config.current_config.external_mcp.enabled ? 'badge-success' : 'badge-error'}">
                  {config.current_config.external_mcp.enabled ? '✅ Active' : '❌ Inactive'}
                </span>
              </div>
              <div class="config-row">
                <span class="config-label">Config File:</span>
                <span class="config-value">{config.current_config.external_mcp.config_file}</span>
              </div>
              <div class="config-row">
                <span class="config-label">Output Directory:</span>
                <span class="config-value">{config.current_config.external_mcp.capabilities_output_dir}</span>
              </div>
              <div class="config-row">
                <span class="config-label">Refresh Interval:</span>
                <span class="config-value">{config.current_config.external_mcp.refresh_interval_minutes}min</span>
              </div>
              
              {#if expandedSections.has('external_mcp_content')}
                <div class="mt-4 p-3 bg-gray-50 rounded-lg">
                  <div class="flex items-center justify-between mb-2">
                    <span class="text-sm font-medium text-gray-700">External MCP Configuration:</span>
                    <button 
                      class="btn-xs-secondary"
                      on:click={() => copyToClipboard(config.current_config.external_mcp.config_content, 'External MCP config')}
                    >
                      📋 Copy
                    </button>
                  </div>
                  <pre class="code-block">{config.current_config.external_mcp.config_content}</pre>
                </div>
              {/if}
            </div>
          </div>
        </div>

      {:else if activeTab === 'runtime'}
        <!-- Runtime Status Tab -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          <!-- System Metrics -->
          <div class="status-card">
            <div class="status-header">
              <h3 class="status-title">⚡ System Metrics</h3>
            </div>
            <div class="status-content">
              <div class="metric">
                <div class="metric-label">Uptime</div>
                <div class="metric-value">{formatUptime(config.runtime_status.uptime)}</div>
              </div>
              <div class="metric">
                <div class="metric-label">Tools Loaded</div>
                <div class="metric-value">{config.runtime_status.tools_loaded}</div>
              </div>
            </div>
          </div>

          <!-- Feature Status -->
          <div class="status-card">
            <div class="status-header">
              <h3 class="status-title">🔧 Features</h3>
            </div>
            <div class="status-content">
              <div class="feature-status">
                <span class="feature-name">Hot Reload</span>
                <span class="status-badge {config.runtime_status.hot_reload_active ? 'badge-success' : 'badge-error'}">
                  {config.runtime_status.hot_reload_active ? '✅' : '❌'}
                </span>
              </div>
              <div class="feature-status">
                <span class="feature-name">External MCP</span>
                <span class="status-badge {config.runtime_status.external_mcp_active ? 'badge-success' : 'badge-error'}">
                  {config.runtime_status.external_mcp_active ? '✅' : '❌'}
                </span>
              </div>
              <div class="feature-status">
                <span class="feature-name">Smart Discovery</span>
                <span class="status-badge {config.runtime_status.smart_discovery_active ? 'badge-success' : 'badge-error'}">
                  {config.runtime_status.smart_discovery_active ? '✅' : '❌'}
                </span>
              </div>
              <div class="feature-status">
                <span class="feature-name">Authentication</span>
                <span class="status-badge {config.runtime_status.authentication_enabled ? 'badge-success' : 'badge-error'}">
                  {config.runtime_status.authentication_enabled ? '✅' : '❌'}
                </span>
              </div>
              <div class="feature-status">
                <span class="feature-name">TLS</span>
                <span class="status-badge {config.runtime_status.tls_enabled ? 'badge-success' : 'badge-error'}">
                  {config.runtime_status.tls_enabled ? '✅' : '❌'}
                </span>
              </div>
            </div>
          </div>

          <!-- Environment Variables -->
          <div class="status-card">
            <div class="status-header">
              <h3 class="status-title">🔐 Environment</h3>
            </div>
            <div class="status-content">
              <div class="env-var">
                <div class="env-label">OpenAI API Key</div>
                <div class="env-value">
                  <span class="text-sm text-gray-600">{config.runtime_status.environment_variables.OPENAI_API_KEY.masked_value}</span>
                  {#if config.runtime_status.environment_variables.OPENAI_API_KEY.set}
                    <button 
                      class="btn-xs-secondary ml-2"
                      on:click={() => copyToClipboard(config.runtime_status.environment_variables.OPENAI_API_KEY.full_value, 'OpenAI API Key')}
                    >
                      📋
                    </button>
                  {/if}
                </div>
              </div>
              <div class="env-var">
                <div class="env-label">Anthropic API Key</div>
                <div class="env-value">
                  <span class="text-sm text-gray-600">{config.runtime_status.environment_variables.ANTHROPIC_API_KEY.masked_value}</span>
                  {#if config.runtime_status.environment_variables.ANTHROPIC_API_KEY.set}
                    <button 
                      class="btn-xs-secondary ml-2"
                      on:click={() => copyToClipboard(config.runtime_status.environment_variables.ANTHROPIC_API_KEY.full_value, 'Anthropic API Key')}
                    >
                      📋
                    </button>
                  {/if}
                </div>
              </div>
              <div class="env-var">
                <div class="env-label">Ollama URL</div>
                <div class="env-value text-sm text-gray-600">{config.runtime_status.environment_variables.OLLAMA_BASE_URL}</div>
              </div>
              <div class="env-var">
                <div class="env-label">Log Level</div>
                <div class="env-value text-sm text-gray-600">{config.runtime_status.environment_variables.RUST_LOG}</div>
              </div>
            </div>
          </div>
        </div>

      {:else if activeTab === 'files'}
        <!-- Configuration Files Tab -->
        <div class="space-y-6">
          <!-- Active Configuration -->
          <div class="file-card">
            <div class="file-header">
              <h3 class="file-title">📄 Active Configuration</h3>
              <div class="file-meta">
                <span class="file-path">{config.config_files.active_config.path}</span>
                <button 
                  class="btn-sm-secondary"
                  on:click={() => toggleSection('active_config')}
                >
                  {expandedSections.has('active_config') ? 'Hide Content' : 'Show Content'}
                </button>
              </div>
            </div>
            
            {#if expandedSections.has('active_config')}
              <div class="file-content">
                <div class="flex items-center justify-between mb-3">
                  <span class="text-sm text-gray-600">Current active configuration file</span>
                  <button 
                    class="btn-xs-secondary"
                    on:click={() => copyToClipboard(config.config_files.active_config.content, 'Active config')}
                  >
                    📋 Copy All
                  </button>
                </div>
                <pre class="code-block">{config.config_files.active_config.content}</pre>
              </div>
            {/if}
          </div>

          <!-- Template Files -->
          <div class="file-card">
            <div class="file-header">
              <h3 class="file-title">📋 Template Files</h3>
            </div>
            
            <div class="space-y-4">
              <!-- Main Config Template -->
              <div class="template-item">
                <div class="template-header">
                  <span class="template-name">Main Configuration Template</span>
                  <div class="template-actions">
                    <span class="template-path">{config.config_files.templates.main_config.path}</span>
                    <button 
                      class="btn-xs-secondary"
                      on:click={() => toggleSection('main_template')}
                    >
                      {expandedSections.has('main_template') ? 'Hide' : 'Show'}
                    </button>
                  </div>
                </div>
                
                {#if expandedSections.has('main_template')}
                  <div class="template-content">
                    <div class="flex items-center justify-between mb-2">
                      <span class="text-sm text-gray-600">Template for main configuration</span>
                      <button 
                        class="btn-xs-secondary"
                        on:click={() => copyToClipboard(config.config_files.templates.main_config.content, 'Main config template')}
                      >
                        📋 Copy
                      </button>
                    </div>
                    <pre class="code-block">{config.config_files.templates.main_config.content}</pre>
                  </div>
                {/if}
              </div>

              <!-- External MCP Template -->
              <div class="template-item">
                <div class="template-header">
                  <span class="template-name">External MCP Template</span>
                  <div class="template-actions">
                    <span class="template-path">{config.config_files.templates.external_mcp.path}</span>
                    <button 
                      class="btn-xs-secondary"
                      on:click={() => toggleSection('external_mcp_template')}
                    >
                      {expandedSections.has('external_mcp_template') ? 'Hide' : 'Show'}
                    </button>
                  </div>
                </div>
                
                {#if expandedSections.has('external_mcp_template')}
                  <div class="template-content">
                    <div class="flex items-center justify-between mb-2">
                      <span class="text-sm text-gray-600">Template for external MCP servers</span>
                      <button 
                        class="btn-xs-secondary"
                        on:click={() => copyToClipboard(config.config_files.templates.external_mcp.content, 'External MCP template')}
                      >
                        📋 Copy
                      </button>
                    </div>
                    <pre class="code-block">{config.config_files.templates.external_mcp.content}</pre>
                  </div>
                {/if}
              </div>
            </div>
          </div>

          <!-- Example Files -->
          <div class="file-card">
            <div class="file-header">
              <h3 class="file-title">📖 Example Files</h3>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              {#each Object.entries(config.config_files.examples) as [key, example]}
                <div class="example-item">
                  <div class="example-header">
                    <span class="example-name">{key.replace('_', ' ').toUpperCase()}</span>
                    <button 
                      class="btn-xs-secondary"
                      on:click={() => toggleSection(`example_${key}`)}
                    >
                      {expandedSections.has(`example_${key}`) ? 'Hide' : 'Show'}
                    </button>
                  </div>
                  <div class="example-path">{example.path}</div>
                  
                  {#if expandedSections.has(`example_${key}`)}
                    <div class="example-content">
                      <div class="flex items-center justify-between mb-2">
                        <span class="text-xs text-gray-500">Example configuration</span>
                        <button 
                          class="btn-xs-secondary"
                          on:click={() => copyToClipboard(example.content, `${key} example`)}
                        >
                          📋
                        </button>
                      </div>
                      <pre class="code-block-sm">{example.content}</pre>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        </div>

      {:else if activeTab === 'templates' && templates}
        <!-- Templates & Examples Tab -->
        <div class="space-y-8">
          <!-- Main Config Template Details -->
          <div class="template-details-card">
            <div class="template-details-header">
              <h3 class="template-details-title">📋 Main Configuration Template</h3>
              <div class="flex gap-2">
                <button 
                  class="btn-sm-secondary"
                  on:click={() => toggleSection('main_config_template_content')}
                >
                  {expandedSections.has('main_config_template_content') ? 'Hide Template' : 'View Template'}
                </button>
                <button 
                  class="btn-sm-secondary"
                  on:click={() => toggleSection('main_config_details')}
                >
                  {expandedSections.has('main_config_details') ? 'Hide Details' : 'Show Details'}
                </button>
              </div>
            </div>
            
            {#if expandedSections.has('main_config_template_content')}
              <div class="template-details-content border-b border-gray-200">
                <div class="flex items-center justify-between mb-3">
                  <h4 class="text-lg font-semibold text-gray-800">Complete Template File</h4>
                  <button 
                    class="btn-xs-secondary"
                    on:click={() => copyToClipboard(templates.main_config_template.content, 'Main config template')}
                  >
                    📋 Copy Template
                  </button>
                </div>
                <pre class="code-block">{templates.main_config_template.content}</pre>
              </div>
            {/if}
            
            {#if expandedSections.has('main_config_details')}
              <div class="template-details-content">
                <p class="text-gray-600 mb-6">{templates.main_config_template.description}</p>
                
                <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                  {#each Object.entries(templates.main_config_template.sections) as [sectionName, section]}
                    <div class="section-card">
                      <h4 class="section-title">{sectionName}</h4>
                      <p class="section-description">{section.description}</p>
                      
                      <div class="properties-grid">
                        {#each Object.entries(section.properties) as [prop, desc]}
                          <div class="property-item">
                            <span class="property-name">{prop}</span>
                            <span class="property-desc">
                              {#if typeof desc === 'string'}
                                {desc}
                              {:else}
                                <div class="nested-properties">
                                  {#each Object.entries(desc) as [subProp, subDesc]}
                                    <div class="nested-property">
                                      <strong>{subProp}:</strong> {subDesc}
                                    </div>
                                  {/each}
                                </div>
                              {/if}
                            </span>
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>

          <!-- External MCP Template -->
          <div class="template-details-card">
            <div class="template-details-header">
              <h3 class="template-details-title">🔗 External MCP Template</h3>
              <div class="flex gap-2">
                <button 
                  class="btn-sm-secondary"
                  on:click={() => toggleSection('external_mcp_template_content')}
                >
                  {expandedSections.has('external_mcp_template_content') ? 'Hide Template' : 'View Template'}
                </button>
                <button 
                  class="btn-sm-secondary"
                  on:click={() => toggleSection('external_mcp_details')}
                >
                  {expandedSections.has('external_mcp_details') ? 'Hide Examples' : 'Show Examples'}
                </button>
              </div>
            </div>
            
            {#if expandedSections.has('external_mcp_template_content')}
              <div class="template-details-content border-b border-gray-200">
                <div class="flex items-center justify-between mb-3">
                  <h4 class="text-lg font-semibold text-gray-800">Complete Template File</h4>
                  <button 
                    class="btn-xs-secondary"
                    on:click={() => copyToClipboard(templates.external_mcp_template.content, 'External MCP template')}
                  >
                    📋 Copy Template
                  </button>
                </div>
                <pre class="code-block">{templates.external_mcp_template.content}</pre>
              </div>
            {/if}
            
            {#if expandedSections.has('external_mcp_details')}
              <div class="template-details-content">
                <p class="text-gray-600 mb-6">{templates.external_mcp_template.description}</p>
                
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {#each Object.entries(templates.external_mcp_template.examples) as [name, example]}
                    <div class="mcp-example-card">
                      <h5 class="mcp-example-title">{name}</h5>
                      <p class="mcp-example-desc">{example.description || 'No description available'}</p>
                      <div class="mcp-example-details">
                        <div class="detail-row">
                          <span class="detail-label">Command:</span>
                          <code class="detail-value">{example.command}</code>
                        </div>
                        <div class="detail-row">
                          <span class="detail-label">Args:</span>
                          <code class="detail-value">{JSON.stringify(example.args)}</code>
                        </div>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>

          <!-- Authentication Examples -->
          <div class="template-details-card">
            <div class="template-details-header">
              <h3 class="template-details-title">🔒 Authentication Examples</h3>
              <div class="flex gap-2">
                <button 
                  class="btn-sm-secondary"
                  on:click={() => toggleSection('auth_template_content')}
                >
                  {expandedSections.has('auth_template_content') ? 'Hide Templates' : 'View Templates'}
                </button>
                <button 
                  class="btn-sm-secondary"
                  on:click={() => toggleSection('auth_examples')}
                >
                  {expandedSections.has('auth_examples') ? 'Hide Examples' : 'Show Examples'}
                </button>
              </div>
            </div>
            
            {#if expandedSections.has('auth_template_content')}
              <div class="template-details-content border-b border-gray-200">
                <div class="space-y-6">
                  <!-- API Key Template -->
                  <div>
                    <div class="flex items-center justify-between mb-3">
                      <h4 class="text-lg font-semibold text-gray-800">API Key Authentication Template</h4>
                      <button 
                        class="btn-xs-secondary"
                        on:click={() => copyToClipboard(templates.auth_examples.api_key_content, 'API key auth template')}
                      >
                        📋 Copy Template
                      </button>
                    </div>
                    <pre class="code-block">{templates.auth_examples.api_key_content}</pre>
                  </div>
                  
                  <!-- OAuth Template -->
                  <div>
                    <div class="flex items-center justify-between mb-3">
                      <h4 class="text-lg font-semibold text-gray-800">OAuth Authentication Template</h4>
                      <button 
                        class="btn-xs-secondary"
                        on:click={() => copyToClipboard(templates.auth_examples.oauth_content, 'OAuth auth template')}
                      >
                        📋 Copy Template
                      </button>
                    </div>
                    <pre class="code-block">{templates.auth_examples.oauth_content}</pre>
                  </div>
                </div>
              </div>
            {/if}
            
            {#if expandedSections.has('auth_examples')}
              <div class="template-details-content">
                <p class="text-gray-600 mb-6">{templates.auth_examples.description}</p>
                
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
                  {#each ['api_key_example', 'oauth_example', 'jwt_example'] as exampleType}
                    <div class="auth-example-card">
                      <h5 class="auth-example-title">{exampleType.replace('_', ' ').replace('example', '').trim()}</h5>
                      <div class="auth-type-badge">{templates.auth_examples[exampleType].type}</div>
                      
                      <div class="auth-properties">
                        {#each Object.entries(templates.auth_examples[exampleType].properties) as [prop, desc]}
                          <div class="auth-property">
                            <span class="auth-prop-name">{prop}</span>
                            <span class="auth-prop-desc">{desc}</span>
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>

          <!-- Additional Template Examples -->
          <div class="template-details-card">
            <div class="template-details-header">
              <h3 class="template-details-title">📚 Additional Templates</h3>
              <button 
                class="btn-sm-secondary"
                on:click={() => toggleSection('additional_templates')}
              >
                {expandedSections.has('additional_templates') ? 'Hide Templates' : 'Show Templates'}
              </button>
            </div>
            
            {#if expandedSections.has('additional_templates')}
              <div class="template-details-content">
                <div class="space-y-6">
                  <!-- TLS Examples -->
                  <div>
                    <div class="flex items-center justify-between mb-3">
                      <h4 class="text-lg font-semibold text-gray-800">TLS/SSL Configuration Examples</h4>
                      <button 
                        class="btn-xs-secondary"
                        on:click={() => copyToClipboard(templates.tls_examples.content, 'TLS examples')}
                      >
                        📋 Copy Template
                      </button>
                    </div>
                    <p class="text-gray-600 mb-3">{templates.tls_examples.description}</p>
                    <pre class="code-block">{templates.tls_examples.content}</pre>
                  </div>
                  
                  <!-- MCP Generator Examples -->
                  <div>
                    <div class="flex items-center justify-between mb-3">
                      <h4 class="text-lg font-semibold text-gray-800">MCP Generator Configuration</h4>
                      <button 
                        class="btn-xs-secondary"
                        on:click={() => copyToClipboard(templates.mcp_generator_examples.content, 'MCP generator examples')}
                      >
                        📋 Copy Template
                      </button>
                    </div>
                    <p class="text-gray-600 mb-3">{templates.mcp_generator_examples.description}</p>
                    <pre class="code-block">{templates.mcp_generator_examples.content}</pre>
                  </div>
                  
                  <!-- Capability Example -->
                  <div>
                    <div class="flex items-center justify-between mb-3">
                      <h4 class="text-lg font-semibold text-gray-800">Capability File Example</h4>
                      <button 
                        class="btn-xs-secondary"
                        on:click={() => copyToClipboard(templates.capability_example.content, 'Capability example')}
                      >
                        📋 Copy Template
                      </button>
                    </div>
                    <p class="text-gray-600 mb-3">{templates.capability_example.description}</p>
                    <pre class="code-block">{templates.capability_example.content}</pre>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>

      {:else if activeTab === 'editor'}
        <!-- Configuration Editor & Management Tab -->
        <div class="space-y-6">
          <!-- Configuration Editor -->
          <div class="editor-card">
            <div class="editor-header">
              <h3 class="editor-title">✏️ Configuration Editor</h3>
              <div class="editor-actions">
                <button class="btn-sm-secondary" on:click={loadCurrentConfig}>
                  📄 Load Current
                </button>
                <button class="btn-sm-secondary" on:click={loadTemplate}>
                  📋 Load Template
                </button>
                <button 
                  class="btn-sm-primary" 
                  on:click={validateConfiguration}
                  disabled={isValidating}
                >
                  {isValidating ? '🔄 Validating...' : '✅ Validate'}
                </button>
                <button 
                  class="btn-save {hasUnsavedChanges ? 'btn-save-unsaved' : ''}" 
                  on:click={saveConfiguration}
                  disabled={isSaving}
                  title="Save configuration to file"
                >
                  {isSaving ? '🔄 Saving...' : hasUnsavedChanges ? '💾 Save*' : '💾 Save'}
                </button>
                <button 
                  class="btn-restart" 
                  on:click={restartMagicTunnel}
                  disabled={restartingMagicTunnel}
                  title="Restart MagicTunnel to apply configuration changes"
                >
                  {restartingMagicTunnel ? '🔄 Restarting...' : '🚀 Restart MagicTunnel'}
                </button>
              </div>
            </div>
            
            <div class="editor-content">
              <textarea
                bind:value={editorContent}
                on:input={onEditorContentChange}
                placeholder="Paste your configuration YAML here or use 'Load Current' or 'Load Template' buttons above..."
                class="config-editor"
                rows="20"
              ></textarea>
              
              <!-- Validation Results -->
              {#if validationResult}
                <div class="validation-results">
                  <div class="validation-status {validationResult.valid ? 'validation-success' : 'validation-error'}">
                    <span class="validation-icon">
                      {validationResult.valid ? '✅' : '❌'}
                    </span>
                    <span class="validation-message">{validationResult.message}</span>
                  </div>
                  
                  {#if validationResult.errors.length > 0}
                    <div class="validation-section validation-errors">
                      <h4 class="validation-section-title">Errors ({validationResult.errors.length})</h4>
                      <ul class="validation-list">
                        {#each validationResult.errors as error}
                          <li class="validation-error-item">{error}</li>
                        {/each}
                      </ul>
                    </div>
                  {/if}
                  
                  {#if validationResult.warnings.length > 0}
                    <div class="validation-section validation-warnings">
                      <h4 class="validation-section-title">Warnings ({validationResult.warnings.length})</h4>
                      <ul class="validation-list">
                        {#each validationResult.warnings as warning}
                          <li class="validation-warning-item">{warning}</li>
                        {/each}
                      </ul>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          </div>

          <!-- Backup & Restore Management -->
          <div class="backup-management-grid">
            <!-- Create Backup -->
            <div class="backup-card">
              <div class="backup-header">
                <h3 class="backup-title">💾 Create Backup</h3>
              </div>
              <div class="backup-content">
                <p class="backup-description">
                  Create a timestamped backup of the current configuration before making changes.
                </p>
                <button class="btn-primary w-full" on:click={createBackup}>
                  💾 Create Backup Now
                </button>
              </div>
            </div>

            <!-- Restore from Backup -->
            <div class="backup-card">
              <div class="backup-header">
                <h3 class="backup-title">📦 Restore from Backup</h3>
                <button 
                  class="btn-xs-secondary" 
                  on:click={loadBackups}
                  disabled={isLoadingBackups}
                >
                  {isLoadingBackups ? '🔄' : '🔄 Refresh'}
                </button>
              </div>
              <div class="backup-content">
                {#if backups.length > 0}
                  <select bind:value={selectedBackup} class="backup-select">
                    <option value="">Select a backup to restore...</option>
                    {#each backups as backup}
                      <option value={backup.name}>
                        {backup.created_readable} ({(backup.size / 1024).toFixed(1)} KB)
                      </option>
                    {/each}
                  </select>
                  <button 
                    class="btn-warning w-full mt-3" 
                    on:click={restoreFromBackup}
                    disabled={!selectedBackup}
                  >
                    📦 Restore Selected Backup
                  </button>
                {:else}
                  <p class="text-gray-500 text-sm">No backups found. Create a backup first.</p>
                  <button class="btn-secondary w-full mt-3" on:click={loadBackups}>
                    🔄 Refresh Backup List
                  </button>
                {/if}
              </div>
            </div>
          </div>

          <!-- Operation Results -->
          {#if operationResult}
            <div class="operation-result">
              <div class="operation-result-header">
                <h3 class="operation-result-title">🔄 Operation Result</h3>
                <button 
                  class="text-gray-400 hover:text-gray-600"
                  on:click={() => operationResult = ''}
                >
                  ✕
                </button>
              </div>
              <div class="operation-result-content">
                <pre class="operation-result-text">{operationResult}</pre>
              </div>
            </div>
          {/if}

          <!-- Restart Status -->
          {#if restartingMagicTunnel && restartCountdown > 0}
            <div class="restart-status">
              <div class="restart-status-header">
                <h3 class="restart-status-title">🔄 Restarting MagicTunnel</h3>
              </div>
              <div class="restart-status-content">
                <div class="restart-countdown">
                  <div class="countdown-circle">
                    <span class="countdown-number">{restartCountdown}</span>
                  </div>
                  <p class="countdown-text">
                    Restarting MagicTunnel... ({restartCountdown}s remaining)
                  </p>
                </div>
                <div class="restart-info">
                  <p class="text-sm text-gray-600">
                    The page will automatically reconnect once MagicTunnel has restarted.
                  </p>
                </div>
              </div>
            </div>
          {/if}

          <!-- Restart Result -->
          {#if restartResult}
            <div class="restart-result">
              <div class="restart-result-header">
                <h3 class="restart-result-title">
                  {restartResult.status === 'success' ? '✅ Restart Successful' : '❌ Restart Failed'}
                </h3>
                <button 
                  class="text-gray-400 hover:text-gray-600"
                  on:click={() => restartResult = null}
                >
                  ✕
                </button>
              </div>
              <div class="restart-result-content">
                <div class="restart-result-status {restartResult.status === 'success' ? 'restart-success' : 'restart-error'}">
                  <p class="restart-result-message">{restartResult.message}</p>
                  <p class="restart-result-timestamp">
                    {new Date(restartResult.timestamp).toLocaleString()}
                  </p>
                </div>
              </div>
            </div>
          {/if}

          <!-- Usage Instructions -->
          <div class="usage-instructions">
            <div class="usage-header">
              <h3 class="usage-title">📚 Usage Instructions</h3>
            </div>
            <div class="usage-content">
              <div class="usage-grid">
                <div class="usage-step">
                  <div class="usage-step-number">1</div>
                  <div class="usage-step-content">
                    <h4 class="usage-step-title">Load Configuration</h4>
                    <p class="usage-step-description">
                      Use "Load Current" to edit the active configuration, or "Load Template" to start with a fresh template.
                    </p>
                  </div>
                </div>
                <div class="usage-step">
                  <div class="usage-step-number">2</div>
                  <div class="usage-step-content">
                    <h4 class="usage-step-title">Edit & Validate</h4>
                    <p class="usage-step-description">
                      Make your changes in the editor and click "Validate" to check for syntax errors and warnings.
                    </p>
                  </div>
                </div>
                <div class="usage-step">
                  <div class="usage-step-number">3</div>
                  <div class="usage-step-content">
                    <h4 class="usage-step-title">Backup & Apply</h4>
                    <p class="usage-step-description">
                      Create a backup first, then save your changes to the configuration file and use the "Restart MagicTunnel" button to apply changes.
                    </p>
                  </div>
                </div>
              </div>
              <div class="usage-warning">
                <span class="usage-warning-icon">⚠️</span>
                <span class="usage-warning-text">
                  Configuration changes require a MagicTunnel restart to take effect. Always create a backup before making changes. Use the "Restart MagicTunnel" button for convenient restart.
                </span>
              </div>
            </div>
          </div>
        </div>

      {:else if activeTab === 'env'}
        <!-- Environment Variables Tab -->
        <div class="card">
          <h3>Environment Variables Tab</h3>
          <p>DEBUG: Loading={loadingEnvVars}, EnvVars={envVars ? 'Present' : 'Null'}</p>
        </div>
      {:else}
        <div class="text-center py-16">
          <div class="text-6xl mb-4">❓</div>
          <div class="text-xl text-gray-600 mb-2">Unknown tab selected</div>
          <button class="btn-primary" on:click={() => activeTab = 'current'}>
            Go to Configuration
          </button>
        </div>
    {/if}
  {:else}
    <div class="text-center py-16">
      <div class="text-6xl mb-4">⚠️</div>
      <div class="text-xl text-gray-600 mb-2">Failed to load configuration data</div>
      <button class="btn-primary" on:click={loadConfigurationData}>
        Try Again
      </button>
    </div>
  {/if}
</div>

<!-- Edit Environment Variable Dialog -->
{#if showEditEnvVarDialog && editingEnvVar}
  <div class="restart-dialog-overlay" on:click={() => showEditEnvVarDialog = false}>
    <div class="restart-dialog" on:click|stopPropagation>
      <div class="restart-dialog-header">
        <h2 class="restart-dialog-title">✏️ Edit Environment Variable</h2>
        <button class="restart-dialog-close" on:click={() => showEditEnvVarDialog = false}>
          ✕
        </button>
      </div>
      
      <div class="restart-dialog-content">
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-700 mb-2">Variable Name</label>
          <input 
            type="text" 
            class="w-full px-3 py-2 border border-gray-300 rounded-lg bg-gray-50"
            bind:value={editingEnvVar.name}
            readonly
          />
          <p class="text-xs text-gray-500 mt-1">Variable name cannot be changed</p>
        </div>
        
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-700 mb-2">Variable Value</label>
          <input 
            type="text" 
            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            placeholder="Variable value..."
            bind:value={editingEnvVar.value}
          />
        </div>
        
        <div class="mb-4">
          <label class="block text-sm font-medium text-gray-700 mb-2">Target File</label>
          <select bind:value={selectedEnvFile} class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500">
            <option value=".env.local">.env.local (git-ignored)</option>
            <option value=".env">.env (base environment)</option>
            <option value=".env.development">.env.development</option>
            <option value=".env.production">.env.production</option>
          </select>
        </div>
        
        {#if editingEnvVar.description}
          <div class="mb-4 p-3 bg-blue-50 rounded-lg">
            <p class="text-sm text-blue-800"><strong>Description:</strong> {editingEnvVar.description}</p>
          </div>
        {/if}
      </div>
      
      <div class="restart-dialog-footer">
        <button class="btn-cancel" on:click={() => showEditEnvVarDialog = false}>
          Cancel
        </button>
        <button class="btn-confirm-restart" on:click={updateEnvVar}>
          💾 Update Variable
        </button>
      </div>
    </div>
  </div>
{/if}

</div>

<style>
  .btn-primary {
    @apply px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium;
  }

  .btn-secondary {
    @apply px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors font-medium;
  }

  .btn-sm-secondary {
    @apply px-3 py-1.5 text-sm bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors;
  }

  .btn-xs-secondary {
    @apply px-2 py-1 text-xs bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors;
  }

  .btn-danger {
    @apply px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors font-medium;
  }

  .btn-save {
    @apply px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors font-medium disabled:opacity-50 disabled:cursor-not-allowed;
  }

  .btn-save-unsaved {
    @apply bg-orange-600 hover:bg-orange-700;
  }

  /* Tab Navigation */
  .tab-nav {
    @apply py-2 px-1 border-b-2 font-medium text-sm focus:outline-none transition-colors;
  }

  .tab-nav-active {
    @apply border-blue-500 text-blue-600;
  }

  .tab-nav-inactive {
    @apply border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300;
  }

  /* Configuration Cards */
  .config-card {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .config-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between;
  }

  .config-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .config-content {
    @apply p-6 space-y-4;
  }

  .config-row {
    @apply flex items-center justify-between;
  }

  .config-label {
    @apply text-sm font-medium text-gray-600;
  }

  .config-value {
    @apply text-sm text-gray-900;
  }

  .config-badge {
    @apply px-2 py-1 text-xs font-medium rounded-full;
  }

  .badge-success {
    @apply bg-green-100 text-green-800;
  }

  .badge-error {
    @apply bg-red-100 text-red-800;
  }

  .badge-warning {
    @apply bg-yellow-100 text-yellow-800;
  }

  .badge-info {
    @apply bg-blue-100 text-blue-800;
  }

  .config-list {
    @apply flex flex-wrap gap-2;
  }

  .config-list-item {
    @apply px-2 py-1 bg-gray-100 text-gray-700 text-xs rounded;
  }

  /* Status Cards */
  .status-card {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .status-header {
    @apply px-4 py-3 bg-gray-50 border-b border-gray-200;
  }

  .status-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .status-content {
    @apply p-4 space-y-3;
  }

  .metric {
    @apply text-center;
  }

  .metric-label {
    @apply text-sm text-gray-600;
  }

  .metric-value {
    @apply text-2xl font-bold text-gray-900;
  }

  .feature-status {
    @apply flex items-center justify-between;
  }

  .feature-name {
    @apply text-sm text-gray-700;
  }

  .status-badge {
    @apply px-2 py-1 text-xs font-medium rounded;
  }

  .env-var {
    @apply border-b border-gray-100 pb-2 last:border-b-0;
  }

  .env-label {
    @apply text-xs font-medium text-gray-500 uppercase tracking-wide;
  }

  .env-value {
    @apply flex items-center justify-between mt-1;
  }

  /* File Cards */
  .file-card {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .file-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between;
  }

  .file-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .file-meta {
    @apply flex items-center gap-3;
  }

  .file-path {
    @apply text-sm text-gray-500 font-mono;
  }

  .file-content {
    @apply p-6;
  }

  .template-item {
    @apply border border-gray-200 rounded-lg overflow-hidden;
  }

  .template-header {
    @apply px-4 py-3 bg-gray-50 flex items-center justify-between;
  }

  .template-name {
    @apply font-medium text-gray-800;
  }

  .template-actions {
    @apply flex items-center gap-3;
  }

  .template-path {
    @apply text-sm text-gray-500 font-mono;
  }

  .template-content {
    @apply p-4;
  }

  .example-item {
    @apply border border-gray-200 rounded-lg overflow-hidden;
  }

  .example-header {
    @apply px-3 py-2 bg-gray-50 flex items-center justify-between;
  }

  .example-name {
    @apply text-sm font-medium text-gray-800;
  }

  .example-path {
    @apply px-3 py-1 text-xs text-gray-500 font-mono border-b border-gray-200;
  }

  .example-content {
    @apply p-3;
  }

  /* Template Details */
  .template-details-card {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .template-details-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between;
  }

  .template-details-title {
    @apply text-xl font-semibold text-gray-800;
  }

  .template-details-content {
    @apply p-6;
  }

  .section-card {
    @apply border border-gray-200 rounded-lg p-4;
  }

  .section-title {
    @apply text-lg font-semibold text-gray-800 capitalize mb-2;
  }

  .section-description {
    @apply text-sm text-gray-600 mb-4;
  }

  .properties-grid {
    @apply space-y-3;
  }

  .property-item {
    @apply bg-gray-50 p-3 rounded;
  }

  .property-name {
    @apply block font-semibold text-gray-800 mb-1;
  }

  .property-desc {
    @apply text-sm text-gray-600;
  }

  .nested-properties {
    @apply space-y-1;
  }

  .nested-property {
    @apply text-sm text-gray-600 ml-2;
  }

  .mcp-example-card {
    @apply border border-gray-200 rounded-lg p-4;
  }

  .mcp-example-title {
    @apply font-semibold text-gray-800 mb-2;
  }

  .mcp-example-desc {
    @apply text-sm text-gray-600 mb-3;
  }

  .mcp-example-details {
    @apply space-y-2;
  }

  .detail-row {
    @apply flex items-center gap-2;
  }

  .detail-label {
    @apply text-xs font-medium text-gray-500;
  }

  .detail-value {
    @apply text-xs bg-gray-100 px-2 py-1 rounded font-mono;
  }

  .auth-example-card {
    @apply border border-gray-200 rounded-lg p-4;
  }

  .auth-example-title {
    @apply font-semibold text-gray-800 mb-2 capitalize;
  }

  .auth-type-badge {
    @apply inline-block px-2 py-1 bg-blue-100 text-blue-800 text-xs font-medium rounded mb-3;
  }

  .auth-properties {
    @apply space-y-3;
  }

  .auth-property {
    @apply bg-gray-50 p-3 rounded;
  }

  .auth-prop-name {
    @apply block font-semibold text-gray-800 mb-1;
  }

  .auth-prop-desc {
    @apply text-sm text-gray-600;
  }

  /* Code Blocks */
  .code-block {
    @apply bg-gray-900 text-gray-100 p-4 rounded-lg text-sm font-mono overflow-auto max-h-96 border;
  }

  .code-block-sm {
    @apply bg-gray-900 text-gray-100 p-3 rounded text-xs font-mono overflow-auto max-h-64 border;
  }

  /* Configuration Editor Styles */
  .editor-card {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .editor-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between;
  }

  .editor-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .editor-actions {
    @apply flex items-center gap-3;
  }

  .editor-content {
    @apply p-6;
  }

  .config-editor {
    @apply w-full p-4 border border-gray-300 rounded-lg font-mono text-sm resize-y min-h-[400px] focus:ring-2 focus:ring-blue-500 focus:border-transparent;
  }

  .btn-sm-primary {
    @apply px-3 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors font-medium;
  }

  .btn-warning {
    @apply px-4 py-2 bg-orange-500 text-white rounded-lg hover:bg-orange-600 transition-colors font-medium;
  }

  /* Validation Results */
  .validation-results {
    @apply mt-4 border border-gray-200 rounded-lg overflow-hidden;
  }

  .validation-status {
    @apply px-4 py-3 border-b border-gray-200 flex items-center gap-3;
  }

  .validation-success {
    @apply bg-green-50 border-green-200;
  }

  .validation-error {
    @apply bg-red-50 border-red-200;
  }

  .validation-icon {
    @apply text-lg;
  }

  .validation-message {
    @apply font-medium text-gray-800;
  }

  .validation-section {
    @apply px-4 py-3 border-b border-gray-200 last:border-b-0;
  }

  .validation-errors {
    @apply bg-red-50;
  }

  .validation-warnings {
    @apply bg-yellow-50;
  }

  .validation-section-title {
    @apply font-medium text-gray-800 mb-2;
  }

  .validation-list {
    @apply space-y-1 list-disc list-inside;
  }

  .validation-error-item {
    @apply text-sm text-red-700;
  }

  .validation-warning-item {
    @apply text-sm text-yellow-700;
  }

  /* Backup Management */
  .backup-management-grid {
    @apply grid grid-cols-1 md:grid-cols-2 gap-6;
  }

  .backup-card {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .backup-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between;
  }

  .backup-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .backup-content {
    @apply p-6;
  }

  .backup-description {
    @apply text-sm text-gray-600 mb-4;
  }

  .backup-select {
    @apply w-full p-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent;
  }

  /* Operation Results */
  .operation-result {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .operation-result-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between;
  }

  .operation-result-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .operation-result-content {
    @apply p-6;
  }

  .operation-result-text {
    @apply bg-gray-900 text-gray-100 p-4 rounded-lg text-sm font-mono overflow-auto whitespace-pre-wrap;
  }

  /* Usage Instructions */
  .usage-instructions {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .usage-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200;
  }

  .usage-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .usage-content {
    @apply p-6;
  }

  .usage-grid {
    @apply grid grid-cols-1 md:grid-cols-3 gap-6 mb-6;
  }

  .usage-step {
    @apply flex items-start gap-4;
  }

  .usage-step-number {
    @apply flex-shrink-0 w-8 h-8 bg-blue-600 text-white rounded-full flex items-center justify-center text-sm font-bold;
  }

  .usage-step-content {
    @apply flex-1;
  }

  .usage-step-title {
    @apply font-semibold text-gray-800 mb-2;
  }

  .usage-step-description {
    @apply text-sm text-gray-600 leading-relaxed;
  }

  .usage-warning {
    @apply bg-yellow-50 border border-yellow-200 rounded-lg p-4 flex items-start gap-3;
  }

  .usage-warning-icon {
    @apply flex-shrink-0 text-yellow-600;
  }

  .usage-warning-text {
    @apply text-sm text-yellow-800;
  }

  /* Restart Button */
  .btn-restart {
    @apply px-3 py-1.5 text-sm bg-orange-600 text-white rounded hover:bg-orange-700 transition-colors font-medium disabled:opacity-50 disabled:cursor-not-allowed;
  }

  /* Restart Status */
  .restart-status {
    @apply bg-blue-50 border border-blue-200 rounded-lg shadow-sm overflow-hidden;
  }

  .restart-status-header {
    @apply px-6 py-4 bg-blue-100 border-b border-blue-200;
  }

  .restart-status-title {
    @apply text-lg font-semibold text-blue-800;
  }

  .restart-status-content {
    @apply p-6 text-center;
  }

  .restart-countdown {
    @apply flex flex-col items-center mb-4;
  }

  .countdown-circle {
    @apply w-16 h-16 bg-blue-600 rounded-full flex items-center justify-center mb-4;
  }

  .countdown-number {
    @apply text-2xl font-bold text-white;
  }

  .countdown-text {
    @apply text-blue-700 font-medium mb-2;
  }

  .restart-info {
    @apply border-t border-blue-200 pt-4;
  }

  /* Restart Result */
  .restart-result {
    @apply bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden;
  }

  .restart-result-header {
    @apply px-6 py-4 bg-gray-50 border-b border-gray-200 flex items-center justify-between;
  }

  .restart-result-title {
    @apply text-lg font-semibold text-gray-800;
  }

  .restart-result-content {
    @apply p-6;
  }

  .restart-result-status {
    @apply p-4 rounded-lg;
  }

  .restart-success {
    @apply bg-green-50 border border-green-200;
  }

  .restart-error {
    @apply bg-red-50 border border-red-200;
  }

  .restart-result-message {
    @apply font-medium text-gray-800 mb-2;
  }

  .restart-result-timestamp {
    @apply text-sm text-gray-500;
  }

  /* Restart Confirmation Dialog */
  .restart-dialog-overlay {
    @apply fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50;
  }

  .restart-dialog {
    @apply bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto;
  }

  .restart-dialog-header {
    @apply px-6 py-4 border-b border-gray-200 flex items-center justify-between;
  }

  .restart-dialog-title {
    @apply text-xl font-semibold text-gray-800;
  }

  .restart-dialog-close {
    @apply text-gray-400 hover:text-gray-600 text-xl leading-none;
  }

  .restart-dialog-content {
    @apply p-6 space-y-6;
  }

  .restart-warning {
    @apply flex items-start gap-4 p-4 bg-yellow-50 border border-yellow-200 rounded-lg;
  }

  .restart-warning-icon {
    @apply text-2xl flex-shrink-0;
  }

  .restart-warning-text {
    @apply flex-1;
  }

  .restart-warning-title {
    @apply font-semibold text-yellow-800 mb-1;
  }

  .restart-warning-description {
    @apply text-sm text-yellow-700;
  }

  .startup-options {
    @apply space-y-3;
  }

  .startup-options-label {
    @apply block text-sm font-medium text-gray-700 mb-2;
  }

  .preset-buttons {
    @apply mb-4;
  }

  .preset-label {
    @apply text-sm font-medium text-gray-700 mb-2;
  }

  .preset-button-group {
    @apply flex flex-wrap gap-2;
  }

  .btn-preset {
    @apply px-3 py-2 text-xs bg-blue-100 text-blue-700 rounded-lg hover:bg-blue-200 transition-colors font-medium border border-blue-200;
  }

  .startup-args-input {
    @apply w-full p-3 border border-gray-300 rounded-lg font-mono text-sm resize-y focus:ring-2 focus:ring-blue-500 focus:border-transparent;
  }

  .startup-options-help {
    @apply mt-4 p-4 bg-gray-50 border border-gray-200 rounded-lg;
  }

  .help-title {
    @apply font-medium text-gray-800 mb-3;
  }

  .help-options {
    @apply space-y-2;
  }

  .help-option {
    @apply text-sm text-gray-600;
  }

  .help-option code {
    @apply bg-gray-200 text-gray-800 px-2 py-1 rounded text-xs font-mono;
  }

  .restart-dialog-footer {
    @apply px-6 py-4 border-t border-gray-200 flex items-center justify-end gap-3;
  }

  .btn-cancel {
    @apply px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors font-medium;
  }

  .btn-confirm-restart {
    @apply px-4 py-2 bg-orange-600 text-white rounded-lg hover:bg-orange-700 transition-colors font-medium;
  }
</style>