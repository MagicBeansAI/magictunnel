<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { rootsApi, type UpdateRootsConfigRequest } from '$lib/api/roots';

  const dispatch = createEventDispatcher();

  // Configuration state
  let config: UpdateRootsConfigRequest = {
    blocked_patterns: [
      '^/etc/.*',
      '^/root/.*',
      '^/proc/.*',
      '.*/\\.ssh/.*',
      '.*/\\.aws/.*',
      '.*/\\.env.*'
    ],
    allowed_extensions: ['txt', 'json', 'yaml', 'yml', 'md', 'csv'],
    blocked_extensions: ['key', 'pem', 'p12', 'pfx', 'crt', 'cer'],
    max_depth: 5,
    follow_symlinks: false
  };

  // UI state
  let saving = false;
  let testing = false;
  let testResult: { pattern: string; valid: boolean; error?: string } | null = null;
  let newPattern = '';
  let newAllowedExt = '';
  let newBlockedExt = '';
  let testPath = '';

  // Predefined pattern templates
  const patternTemplates = [
    { name: 'System directories', pattern: '^/(etc|root|proc|sys)/.*', description: 'Block system configuration directories' },
    { name: 'SSH keys', pattern: '.*/\\.ssh/.*', description: 'Block SSH configuration and keys' },
    { name: 'Environment files', pattern: '.*/\\.(env|environment).*', description: 'Block environment variable files' },
    { name: 'AWS credentials', pattern: '.*/\\.aws/.*', description: 'Block AWS credential files' },
    { name: 'Hidden files', pattern: '.*/\\..*', description: 'Block all hidden files and directories' },
    { name: 'Temporary files', pattern: '.*/tmp/.*', description: 'Block temporary directories' },
    { name: 'Log files', pattern: '.*\\.(log|logs)/.*', description: 'Block log directories' },
  ];

  const extensionTemplates = {
    security: ['key', 'pem', 'p12', 'pfx', 'crt', 'cer', 'keystore'],
    code: ['js', 'ts', 'py', 'java', 'cpp', 'c', 'h', 'rs', 'go'],
    data: ['json', 'yaml', 'yml', 'xml', 'csv', 'tsv'],
    documents: ['txt', 'md', 'pdf', 'doc', 'docx'],
    config: ['conf', 'config', 'ini', 'toml', 'properties'],
  };

  async function saveConfiguration() {
    if (saving) return;
    
    saving = true;
    try {
      await rootsApi.updateConfig(config);
      dispatch('configUpdated');
      
      // Show success message (you could use a toast notification)
      alert('Configuration saved successfully!');
    } catch (error) {
      console.error('Failed to save configuration:', error);
      alert(`Failed to save configuration: ${error instanceof Error ? error.message : 'Unknown error'}`);
    } finally {
      saving = false;
    }
  }

  function addPattern() {
    if (!newPattern.trim()) return;
    
    const validation = rootsApi.validatePattern(newPattern);
    if (!validation.valid) {
      alert(`Invalid regex pattern: ${validation.error}`);
      return;
    }
    
    if (!config.blocked_patterns.includes(newPattern)) {
      config.blocked_patterns = [...config.blocked_patterns, newPattern];
      newPattern = '';
    }
  }

  function removePattern(index: number) {
    config.blocked_patterns = config.blocked_patterns.filter((_, i) => i !== index);
  }

  function addPatternTemplate(pattern: string) {
    if (!config.blocked_patterns.includes(pattern)) {
      config.blocked_patterns = [...config.blocked_patterns, pattern];
    }
  }

  function addAllowedExtension() {
    if (!newAllowedExt.trim()) return;
    const ext = newAllowedExt.toLowerCase().replace(/^\./, '');
    if (!config.allowed_extensions.includes(ext)) {
      config.allowed_extensions = [...config.allowed_extensions, ext];
      newAllowedExt = '';
    }
  }

  function removeAllowedExtension(index: number) {
    config.allowed_extensions = config.allowed_extensions.filter((_, i) => i !== index);
  }

  function addBlockedExtension() {
    if (!newBlockedExt.trim()) return;
    const ext = newBlockedExt.toLowerCase().replace(/^\./, '');
    if (!config.blocked_extensions.includes(ext)) {
      config.blocked_extensions = [...config.blocked_extensions, ext];
      newBlockedExt = '';
    }
  }

  function removeBlockedExtension(index: number) {
    config.blocked_extensions = config.blocked_extensions.filter((_, i) => i !== index);
  }

  function addExtensionTemplate(category: keyof typeof extensionTemplates) {
    const extensions = extensionTemplates[category];
    const newExtensions = extensions.filter(ext => !config.allowed_extensions.includes(ext));
    config.allowed_extensions = [...config.allowed_extensions, ...newExtensions];
  }

  function testPattern() {
    if (!newPattern.trim() || !testPath.trim()) return;
    
    testing = true;
    
    try {
      const validation = rootsApi.validatePattern(newPattern);
      if (!validation.valid) {
        testResult = {
          pattern: newPattern,
          valid: false,
          error: validation.error
        };
      } else {
        // Test if pattern matches the test path
        const regex = new RegExp(newPattern);
        const matches = regex.test(testPath);
        
        testResult = {
          pattern: newPattern,
          valid: true,
          error: matches ? `✅ Pattern matches "${testPath}"` : `❌ Pattern does not match "${testPath}"`
        };
      }
    } catch (error) {
      testResult = {
        pattern: newPattern,
        valid: false,
        error: error instanceof Error ? error.message : 'Test failed'
      };
    } finally {
      testing = false;
    }
  }

  function resetToDefaults() {
    if (confirm('Reset to default security configuration? This will overwrite your current settings.')) {
      config = {
        blocked_patterns: [
          '^/etc/.*',
          '^/root/.*', 
          '^/proc/.*',
          '.*/\\.ssh/.*',
          '.*/\\.aws/.*',
          '.*/\\.env.*'
        ],
        allowed_extensions: ['txt', 'json', 'yaml', 'yml', 'md', 'csv'],
        blocked_extensions: ['key', 'pem', 'p12', 'pfx', 'crt', 'cer'],
        max_depth: 5,
        follow_symlinks: false
      };
    }
  }
</script>

<div class="security-config-panel">
  <!-- Header -->
  <div class="panel-header">
    <div class="header-left">
      <h2 class="panel-title">
        🔒 Security Configuration
      </h2>
      <p class="panel-description">
        Configure access patterns and security boundaries for root discovery
      </p>
    </div>
    
    <div class="header-actions">
      <button class="action-btn secondary" on:click={resetToDefaults}>
        Reset to Defaults
      </button>
      <button class="action-btn primary" on:click={saveConfiguration} disabled={saving}>
        {saving ? 'Saving...' : 'Save Configuration'}
      </button>
    </div>
  </div>

  <div class="config-sections">
    <!-- Blocked Patterns Section -->
    <div class="config-section">
      <div class="section-header">
        <h3 class="section-title">
          🚫 Blocked Patterns
        </h3>
        <p class="section-description">
          Regular expressions to block specific paths and directories
        </p>
      </div>

      <div class="pattern-list">
        {#each config.blocked_patterns as pattern, index}
          <div class="pattern-item">
            <code class="pattern-code">{pattern}</code>
            <button class="remove-btn" on:click={() => removePattern(index)}>
              ✕
            </button>
          </div>
        {/each}
      </div>

      <div class="add-pattern-section">
        <div class="input-group">
          <input
            type="text"
            placeholder="Enter regex pattern..."
            bind:value={newPattern}
            class="pattern-input"
            on:keydown={(e) => e.key === 'Enter' && addPattern()}
          />
          <button class="add-btn" on:click={addPattern} disabled={!newPattern.trim()}>
            Add Pattern
          </button>
        </div>

        <!-- Pattern Testing -->
        <div class="pattern-testing">
          <div class="test-input-group">
            <input
              type="text"
              placeholder="Test path (e.g., /etc/passwd)"
              bind:value={testPath}
              class="test-input"
            />
            <button class="test-btn" on:click={testPattern} disabled={!newPattern.trim() || !testPath.trim() || testing}>
              {testing ? 'Testing...' : 'Test'}
            </button>
          </div>
          
          {#if testResult}
            <div class="test-result" class:valid={testResult.valid} class:invalid={!testResult.valid}>
              <strong>Test Result:</strong> {testResult.error || (testResult.valid ? 'Valid pattern' : 'Invalid pattern')}
            </div>
          {/if}
        </div>

        <!-- Pattern Templates -->
        <div class="pattern-templates">
          <label class="templates-label">Quick templates:</label>
          <div class="template-buttons">
            {#each patternTemplates as template}
              <button 
                class="template-btn"
                on:click={() => addPatternTemplate(template.pattern)}
                title={template.description}
              >
                {template.name}
              </button>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <!-- File Extensions Section -->
    <div class="config-section">
      <div class="section-header">
        <h3 class="section-title">
          📄 File Extensions
        </h3>
        <p class="section-description">
          Control which file types are allowed or blocked during discovery
        </p>
      </div>

      <div class="extensions-container">
        <!-- Allowed Extensions -->
        <div class="extension-group">
          <h4 class="extension-title">✅ Allowed Extensions</h4>
          
          <div class="extension-list">
            {#each config.allowed_extensions as ext, index}
              <div class="extension-item allowed">
                <span class="extension-text">.{ext}</span>
                <button class="remove-btn" on:click={() => removeAllowedExtension(index)}>
                  ✕
                </button>
              </div>
            {/each}
          </div>
          
          <div class="add-extension">
            <input
              type="text"
              placeholder="Add extension..."
              bind:value={newAllowedExt}
              class="extension-input"
              on:keydown={(e) => e.key === 'Enter' && addAllowedExtension()}
            />
            <button class="add-btn small" on:click={addAllowedExtension}>
              Add
            </button>
          </div>
        </div>

        <!-- Blocked Extensions -->
        <div class="extension-group">
          <h4 class="extension-title">❌ Blocked Extensions</h4>
          
          <div class="extension-list">
            {#each config.blocked_extensions as ext, index}
              <div class="extension-item blocked">
                <span class="extension-text">.{ext}</span>
                <button class="remove-btn" on:click={() => removeBlockedExtension(index)}>
                  ✕
                </button>
              </div>
            {/each}
          </div>
          
          <div class="add-extension">
            <input
              type="text"
              placeholder="Add extension..."
              bind:value={newBlockedExt}
              class="extension-input"
              on:keydown={(e) => e.key === 'Enter' && addBlockedExtension()}
            />
            <button class="add-btn small" on:click={addBlockedExtension}>
              Add
            </button>
          </div>
        </div>
      </div>

      <!-- Extension Templates -->
      <div class="extension-templates">
        <label class="templates-label">Quick add:</label>
        <div class="template-buttons">
          {#each Object.keys(extensionTemplates) as category}
            <button 
              class="template-btn"
              on:click={() => addExtensionTemplate(category)}
              title={`Add ${category} file extensions`}
            >
              {category.charAt(0).toUpperCase() + category.slice(1)} Files
            </button>
          {/each}
        </div>
      </div>
    </div>

    <!-- Discovery Options Section -->
    <div class="config-section">
      <div class="section-header">
        <h3 class="section-title">
          ⚙️ Discovery Options
        </h3>
        <p class="section-description">
          Configure how root discovery behaves
        </p>
      </div>

      <div class="options-grid">
        <div class="option-item">
          <label class="option-label">
            Max Depth
          </label>
          <input
            type="number"
            min="1"
            max="20"
            bind:value={config.max_depth}
            class="number-input"
          />
          <p class="option-description">
            Maximum directory depth to traverse during discovery
          </p>
        </div>

        <div class="option-item">
          <label class="option-label">
            <input
              type="checkbox"
              bind:checked={config.follow_symlinks}
              class="checkbox-input"
            />
            Follow Symbolic Links
          </label>
          <p class="option-description">
            Whether to follow symbolic links during directory traversal (security risk)
          </p>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .security-config-panel {
    background: white;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .header-left {
    flex: 1;
  }

  .panel-title {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
    color: #1a1a1a;
  }

  .panel-description {
    color: #6b7280;
    margin: 0;
    font-size: 0.875rem;
  }

  .header-actions {
    display: flex;
    gap: 1rem;
  }

  .action-btn {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 8px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .action-btn.primary {
    background: #3b82f6;
    color: white;
  }

  .action-btn.primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .action-btn.secondary {
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #d1d5db;
  }

  .action-btn.secondary:hover:not(:disabled) {
    background: #e5e7eb;
  }

  .action-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .config-sections {
    display: flex;
    flex-direction: column;
    gap: 2rem;
    padding: 1.5rem;
  }

  .config-section {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    overflow: hidden;
  }

  .section-header {
    padding: 1rem 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
  }

  .section-title {
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
    color: #1a1a1a;
  }

  .section-description {
    color: #6b7280;
    margin: 0;
    font-size: 0.875rem;
  }

  .pattern-list {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .pattern-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background: #f3f4f6;
    border-radius: 6px;
  }

  .pattern-code {
    font-family: monospace;
    font-size: 0.875rem;
    color: #1f2937;
    flex: 1;
    overflow-wrap: break-word;
  }

  .remove-btn {
    background: #fee2e2;
    color: #dc2626;
    border: none;
    border-radius: 4px;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
    font-size: 0.75rem;
    margin-left: 1rem;
  }

  .remove-btn:hover {
    background: #fecaca;
  }

  .add-pattern-section {
    padding: 1.5rem;
    border-top: 1px solid #e5e7eb;
    background: #fafafa;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .input-group {
    display: flex;
    gap: 0.75rem;
  }

  .pattern-input,
  .test-input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-family: monospace;
    font-size: 0.875rem;
  }

  .add-btn,
  .test-btn {
    padding: 0.75rem 1rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    white-space: nowrap;
  }

  .add-btn:hover:not(:disabled),
  .test-btn:hover:not(:disabled) {
    background: #2563eb;
  }

  .add-btn:disabled,
  .test-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .pattern-testing {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .test-input-group {
    display: flex;
    gap: 0.75rem;
  }

  .test-result {
    padding: 0.75rem;
    border-radius: 6px;
    font-size: 0.875rem;
  }

  .test-result.valid {
    background: #f0f9ff;
    color: #1e40af;
    border: 1px solid #bae6fd;
  }

  .test-result.invalid {
    background: #fef2f2;
    color: #dc2626;
    border: 1px solid #fecaca;
  }

  .pattern-templates {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .templates-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: #374151;
  }

  .template-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .template-btn {
    padding: 0.5rem 1rem;
    background: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    transition: all 0.2s ease;
  }

  .template-btn:hover {
    background: #e5e7eb;
  }

  .extensions-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
    padding: 1.5rem;
  }

  .extension-group {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .extension-title {
    font-size: 0.875rem;
    font-weight: 600;
    margin: 0;
    color: #374151;
  }

  .extension-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    min-height: 2rem;
  }

  .extension-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.875rem;
  }

  .extension-item.allowed {
    background: #f0f9ff;
    border: 1px solid #bae6fd;
    color: #1e40af;
  }

  .extension-item.blocked {
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #dc2626;
  }

  .extension-text {
    font-family: monospace;
  }

  .add-extension {
    display: flex;
    gap: 0.5rem;
  }

  .extension-input {
    flex: 1;
    padding: 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    font-size: 0.875rem;
  }

  .add-btn.small {
    padding: 0.5rem 0.75rem;
    font-size: 0.875rem;
  }

  .extension-templates {
    padding: 1.5rem;
    border-top: 1px solid #e5e7eb;
    background: #fafafa;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .options-grid {
    padding: 1.5rem;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
  }

  .option-item {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .option-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
    color: #374151;
  }

  .number-input {
    padding: 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 4px;
    width: 100px;
  }

  .checkbox-input {
    width: 1rem;
    height: 1rem;
  }

  .option-description {
    font-size: 0.875rem;
    color: #6b7280;
    margin: 0;
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .panel-header {
      flex-direction: column;
      gap: 1rem;
      align-items: stretch;
    }

    .header-actions {
      justify-content: stretch;
    }

    .action-btn {
      flex: 1;
      text-align: center;
    }

    .extensions-container {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }

    .input-group,
    .test-input-group {
      flex-direction: column;
    }

    .options-grid {
      grid-template-columns: 1fr;
      gap: 1rem;
    }
  }
</style>