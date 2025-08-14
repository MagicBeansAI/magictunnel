<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { rootsApi, type Root, type AddManualRootRequest } from '$lib/api/roots';

  export let manualRoots: Root[];

  const dispatch = createEventDispatcher();

  let showAddForm = false;
  let newRoot: AddManualRootRequest = {
    root_type: 'filesystem',
    path: '',
    name: '',
    permissions: ['read']
  };

  const rootTypes = [
    { value: 'filesystem', label: 'File System', icon: '💾' },
    { value: 'uri', label: 'URI/URL', icon: '🌐' },
    { value: 'database', label: 'Database', icon: '🗄️' },
    { value: 'api', label: 'API Endpoint', icon: '🔌' },
    { value: 'cloud_storage', label: 'Cloud Storage', icon: '☁️' },
    { value: 'custom', label: 'Custom', icon: '⚙️' },
  ];

  const permissionOptions = [
    { value: 'read', label: 'Read', color: 'blue' },
    { value: 'write', label: 'Write', color: 'orange' },
    { value: 'execute', label: 'Execute', color: 'green' },
    { value: 'create', label: 'Create', color: 'purple' },
    { value: 'delete', label: 'Delete', color: 'red' },
  ];

  async function addRoot() {
    try {
      await rootsApi.addManualRoot(newRoot);
      dispatch('rootAdded');
      resetForm();
      showAddForm = false;
    } catch (error) {
      alert(`Failed to add root: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  async function removeRoot(rootId: string) {
    if (confirm('Are you sure you want to remove this manual root?')) {
      try {
        await rootsApi.removeManualRoot(rootId);
        dispatch('rootRemoved');
      } catch (error) {
        alert(`Failed to remove root: ${error instanceof Error ? error.message : 'Unknown error'}`);
      }
    }
  }

  function resetForm() {
    newRoot = {
      root_type: 'filesystem',
      path: '',
      name: '',
      permissions: ['read']
    };
  }

  function togglePermission(permission: string) {
    if (newRoot.permissions.includes(permission)) {
      newRoot.permissions = newRoot.permissions.filter(p => p !== permission);
    } else {
      newRoot.permissions = [...newRoot.permissions, permission];
    }
  }
</script>

<div class="manual-roots-manager">
  <div class="manager-header">
    <div class="header-left">
      <h2 class="manager-title">⚙️ Manual Root Management</h2>
      <p class="manager-description">{manualRoots.length} manual roots configured</p>
    </div>
    <button class="add-root-btn" on:click={() => showAddForm = !showAddForm}>
      {showAddForm ? '✕ Cancel' : '+ Add Manual Root'}
    </button>
  </div>

  {#if showAddForm}
    <div class="add-form">
      <h3>Add Manual Root</h3>
      
      <div class="form-grid">
        <div class="form-group">
          <label>Root Type</label>
          <select bind:value={newRoot.root_type} class="form-select">
            {#each rootTypes as type}
              <option value={type.value}>{type.icon} {type.label}</option>
            {/each}
          </select>
        </div>

        <div class="form-group">
          <label>Path *</label>
          <input type="text" bind:value={newRoot.path} placeholder="/path/to/root or https://example.com" class="form-input" />
        </div>

        <div class="form-group">
          <label>Name (optional)</label>
          <input type="text" bind:value={newRoot.name} placeholder="Friendly name" class="form-input" />
        </div>

        <div class="form-group full-width">
          <label>Permissions</label>
          <div class="permission-checkboxes">
            {#each permissionOptions as perm}
              <label class="permission-label">
                <input 
                  type="checkbox" 
                  checked={newRoot.permissions.includes(perm.value)}
                  on:change={() => togglePermission(perm.value)}
                />
                <span class="permission-badge {perm.color}">{perm.label}</span>
              </label>
            {/each}
          </div>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn secondary" on:click={() => showAddForm = false}>Cancel</button>
        <button class="btn primary" on:click={addRoot} disabled={!newRoot.path.trim()}>Add Root</button>
      </div>
    </div>
  {/if}

  <div class="roots-list">
    {#each manualRoots as root (root.id)}
      <div class="root-card">
        <div class="root-header">
          <div class="root-icon">{rootTypes.find(t => t.value === root.root_type)?.icon || '📁'}</div>
          <div class="root-info">
            <div class="root-name">{root.name || root.path}</div>
            <div class="root-path">{root.path}</div>
            <div class="root-type">{rootsApi.getRootTypeDisplayName(root.root_type)}</div>
          </div>
          <button class="remove-btn" on:click={() => removeRoot(root.id)}>🗑️</button>
        </div>
        
        <div class="root-permissions">
          {#each root.permissions as permission}
            <span class="permission-badge {permissionOptions.find(p => p.value === permission)?.color || 'gray'}">
              {rootsApi.getPermissionDisplayName(permission)}
            </span>
          {/each}
        </div>
      </div>
    {:else}
      <div class="empty-state">
        <div class="empty-icon">⚙️</div>
        <h3>No Manual Roots</h3>
        <p>Add custom roots that aren't automatically discovered</p>
      </div>
    {/each}
  </div>
</div>

<style>
  .manual-roots-manager {
    background: white;
    border-radius: 12px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    overflow: hidden;
  }

  .manager-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .manager-title {
    font-size: 1.25rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
  }

  .manager-description {
    color: #6b7280;
    margin: 0;
    font-size: 0.875rem;
  }

  .add-root-btn {
    padding: 0.75rem 1.5rem;
    background: #3b82f6;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-weight: 500;
  }

  .add-form {
    padding: 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
  }

  .add-form h3 {
    margin: 0 0 1rem 0;
    color: #374151;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .form-group.full-width {
    grid-column: 1 / -1;
  }

  .form-group label {
    font-weight: 500;
    color: #374151;
    font-size: 0.875rem;
  }

  .form-input,
  .form-select {
    padding: 0.75rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
  }

  .permission-checkboxes {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
  }

  .permission-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }

  .permission-badge {
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .permission-badge.blue { background: #dbeafe; color: #1e40af; }
  .permission-badge.orange { background: #fed7aa; color: #c2410c; }
  .permission-badge.green { background: #dcfce7; color: #166534; }
  .permission-badge.purple { background: #e9d5ff; color: #7c3aed; }
  .permission-badge.red { background: #fecaca; color: #dc2626; }
  .permission-badge.gray { background: #f3f4f6; color: #4b5563; }

  .form-actions {
    display: flex;
    gap: 1rem;
    justify-content: flex-end;
  }

  .btn {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
  }

  .btn.primary {
    background: #3b82f6;
    color: white;
  }

  .btn.secondary {
    background: #f3f4f6;
    color: #374151;
    border: 1px solid #d1d5db;
  }

  .roots-list {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .root-card {
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 1rem;
  }

  .root-header {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 0.75rem;
  }

  .root-icon {
    font-size: 1.5rem;
    flex-shrink: 0;
  }

  .root-info {
    flex: 1;
    min-width: 0;
  }

  .root-name {
    font-weight: 500;
    color: #1a1a1a;
  }

  .root-path {
    font-family: monospace;
    font-size: 0.875rem;
    color: #6b7280;
    overflow-wrap: break-word;
  }

  .root-type {
    font-size: 0.875rem;
    color: #9ca3af;
  }

  .remove-btn {
    background: #fee2e2;
    border: none;
    border-radius: 4px;
    padding: 0.5rem;
    cursor: pointer;
    color: #dc2626;
  }

  .root-permissions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: #6b7280;
  }

  .empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .empty-state h3 {
    margin-bottom: 0.5rem;
    color: #374151;
  }

  @media (max-width: 768px) {
    .manager-header {
      flex-direction: column;
      gap: 1rem;
      align-items: stretch;
    }

    .form-grid {
      grid-template-columns: 1fr;
    }

    .form-actions {
      justify-content: stretch;
    }

    .btn {
      flex: 1;
    }
  }
</style>