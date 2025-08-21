<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  
  export let visible: boolean = false;
  
  // Pattern data
  let patterns: any = null;
  let loading = true;
  let error = '';
  
  // Pattern editing
  let editingPattern: any = null;
  let editModalVisible = false;
  let editForm = {
    name: '',
    regex: '',
    action: 'allow' as 'allow' | 'deny',
    reason: '',
    enabled: true,
    type: 'global' as 'global' | 'tools' | 'capabilities'
  };
  
  // Filter and sort
  let filterType: 'all' | 'global' | 'tools' | 'capabilities' = 'all';
  let filterEnabled: 'all' | 'enabled' | 'disabled' = 'all';
  let filterAction: 'all' | 'allow' | 'deny' = 'all';
  let searchQuery = '';
  let sortBy: 'name' | 'type' | 'action' | 'enabled' = 'name';
  let sortOrder: 'asc' | 'desc' = 'asc';
  
  // Filtered patterns for display
  let filteredPatterns: any[] = [];
  
  // Load patterns from API
  async function loadPatterns() {
    console.log('🎯 PatternManager: Loading patterns, visible:', visible);
    if (!visible) return;
    
    try {
      loading = true;
      error = '';
      
      // Call the new patterns API endpoint
      const response = await fetch('/dashboard/api/security/allowlist/patterns');
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }
      
      patterns = await response.json();
      console.log('✅ PatternManager: Loaded', patterns?.summary?.total_patterns || 0, 'patterns successfully');
    } catch (err) {
      console.error('❌ PatternManager: Failed to load patterns:', err);
      error = err instanceof Error ? err.message : 'Failed to load patterns';
    } finally {
      loading = false;
    }
  }
  
  // Get all patterns flattened for display
  function getAllPatterns(): any[] {
    if (!patterns) return [];
    
    const allPatterns = [
      ...patterns.global.map(p => ({ ...p, category: 'global' })),
      ...patterns.tools.map(p => ({ ...p, category: 'tools' })),
      ...patterns.capabilities.map(p => ({ ...p, category: 'capabilities' }))
    ];
    
    return allPatterns
      .filter(p => {
        // Type filter
        if (filterType !== 'all' && p.category !== filterType) return false;
        
        // Enabled filter
        if (filterEnabled !== 'all') {
          if (filterEnabled === 'enabled' && !p.enabled) return false;
          if (filterEnabled === 'disabled' && p.enabled) return false;
        }
        
        // Action filter
        if (filterAction !== 'all' && p.action !== filterAction) return false;
        
        // Search filter
        if (searchQuery && !p.name.toLowerCase().includes(searchQuery.toLowerCase()) && 
            !p.regex.toLowerCase().includes(searchQuery.toLowerCase())) return false;
        
        return true;
      })
      .sort((a, b) => {
        let aVal = a[sortBy];
        let bVal = b[sortBy];
        
        if (sortBy === 'enabled') {
          aVal = a.enabled ? 1 : 0;
          bVal = b.enabled ? 1 : 0;
        }
        
        const result = aVal < bVal ? -1 : aVal > bVal ? 1 : 0;
        return sortOrder === 'asc' ? result : -result;
      });
  }
  
  // Open edit modal
  function editPattern(pattern: any) {
    editingPattern = pattern;
    editForm = {
      name: pattern.name,
      regex: pattern.regex,
      action: pattern.action,
      reason: pattern.reason || '',
      enabled: pattern.enabled,
      type: pattern.category
    };
    editModalVisible = true;
  }
  
  // Close edit modal
  function closeEditModal() {
    editModalVisible = false;
    editingPattern = null;
  }
  
  // Save pattern changes (placeholder - would need API implementation)
  async function savePattern() {
    try {
      // TODO: Implement pattern update API call
      console.log('Saving pattern:', editForm);
      
      // Close modal and reload data
      closeEditModal();
      await loadPatterns();
    } catch (err) {
      console.error('Failed to save pattern:', err);
      error = `Failed to save pattern: ${err}`;
    }
  }
  
  // Toggle pattern enabled state (placeholder - would need API implementation)
  async function togglePattern(pattern: any) {
    try {
      // TODO: Implement pattern toggle API call
      console.log('Toggling pattern:', pattern.id, 'to:', !pattern.enabled);
      
      // Reload data
      await loadPatterns();
    } catch (err) {
      console.error('Failed to toggle pattern:', err);
      error = `Failed to toggle pattern: ${err}`;
    }
  }
  
  // Delete pattern (placeholder - would need API implementation)
  async function deletePattern(pattern: any) {
    if (!confirm(`Are you sure you want to delete the pattern "${pattern.name}"?`)) {
      return;
    }
    
    try {
      // TODO: Implement pattern delete API call
      console.log('Deleting pattern:', pattern.id);
      
      // Reload data
      await loadPatterns();
    } catch (err) {
      console.error('Failed to delete pattern:', err);
      error = `Failed to delete pattern: ${err}`;
    }
  }
  
  // Get pattern type badge styling
  function getTypeBadgeClass(type: string): string {
    switch (type) {
      case 'global':
        return 'bg-purple-100 text-purple-700';
      case 'tools':
        return 'bg-blue-100 text-blue-700';
      case 'capabilities':
        return 'bg-green-100 text-green-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  }
  
  // Get action badge styling
  function getActionBadgeClass(action: string): string {
    return action === 'allow' 
      ? 'bg-green-100 text-green-700' 
      : 'bg-red-100 text-red-700';
  }
  
  // Reactive loading when visibility changes
  $: {
    console.log('🎯 PatternManager visibility:', visible);
    if (visible) {
      loadPatterns();
    }
  }
  
  $: {
    filteredPatterns = getAllPatterns();
    if (patterns && filteredPatterns.length > 0) {
      console.log('📋 PatternManager: Displaying', filteredPatterns.length, 'filtered patterns');
    }
  }
</script>

{#if visible}
  <div class="space-y-6">
    <!-- Header with summary -->
    {#if patterns}
      <div class="bg-gradient-to-r from-blue-50 to-indigo-50 p-4 rounded-lg border border-blue-200">
        <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between">
          <div class="mb-3 sm:mb-0">
            <h3 class="text-lg font-semibold text-gray-900">📋 Pattern Summary</h3>
            <p class="text-sm text-gray-600">Active security patterns across all categories</p>
          </div>
          <div class="flex flex-wrap gap-4">
            <div class="flex items-center space-x-1">
              <div class="w-3 h-3 bg-purple-500 rounded-full"></div>
              <span class="text-sm font-medium text-gray-700">{patterns.summary.global_count} Global</span>
            </div>
            <div class="flex items-center space-x-1">
              <div class="w-3 h-3 bg-blue-500 rounded-full"></div>
              <span class="text-sm font-medium text-gray-700">{patterns.summary.tool_count} Tools</span>
            </div>
            <div class="flex items-center space-x-1">
              <div class="w-3 h-3 bg-green-500 rounded-full"></div>
              <span class="text-sm font-medium text-gray-700">{patterns.summary.capability_count} Capabilities</span>
            </div>
            <div class="flex items-center space-x-1">
              <div class="w-3 h-3 bg-gray-500 rounded-full"></div>
              <span class="text-sm font-medium text-gray-700">{patterns.summary.total_patterns} Total</span>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Controls -->
    <div class="bg-white p-4 rounded-lg border">
      <div class="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4">
        <h3 class="text-lg font-semibold text-gray-900">🔍 Filter & Search</h3>
        
        <div class="flex flex-col sm:flex-row gap-3">
          <!-- Filters Row -->
          <div class="flex flex-wrap gap-2">
            <!-- Type Filter -->
            <select bind:value={filterType} class="text-sm px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500">
              <option value="all">All Types</option>
              <option value="global">Global</option>
              <option value="tools">Tools</option>
              <option value="capabilities">Capabilities</option>
            </select>
            
            <!-- Action Filter -->
            <select bind:value={filterAction} class="text-sm px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500">
              <option value="all">All Actions</option>
              <option value="allow">Allow Only</option>
              <option value="deny">Deny Only</option>
            </select>
            
            <!-- Enabled Filter -->
            <select bind:value={filterEnabled} class="text-sm px-3 py-2 border border-gray-300 rounded focus:ring-2 focus:ring-blue-500">
              <option value="all">All States</option>
              <option value="enabled">Enabled</option>
              <option value="disabled">Disabled</option>
            </select>
            
            <!-- Sort Controls -->
            <div class="flex">
              <select bind:value={sortBy} class="text-sm px-3 py-2 border border-gray-300 rounded-l focus:ring-2 focus:ring-blue-500">
                <option value="name">Name</option>
                <option value="type">Type</option>
                <option value="action">Action</option>
                <option value="enabled">State</option>
              </select>
              <button 
                on:click={() => sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'}
                class="px-3 py-2 bg-gray-100 hover:bg-gray-200 border border-l-0 border-gray-300 rounded-r text-sm font-medium"
                title="Toggle sort order"
              >
                {sortOrder === 'asc' ? '↑' : '↓'}
              </button>
            </div>
          </div>
          
          <!-- Search -->
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search patterns..."
            class="flex-1 min-w-0 px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
      </div>
    </div>

    <!-- Loading State -->
    {#if loading}
      <div class="flex justify-center items-center py-12">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        <span class="ml-3 text-gray-600">Loading patterns...</span>
      </div>
    {/if}

    <!-- Error State -->
    {#if error}
      <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
        <strong class="font-bold">Error:</strong>
        <span class="block sm:inline">{error}</span>
      </div>
    {/if}

    <!-- Pattern Cards -->
    {#if !loading && filteredPatterns.length > 0}
      <div class="space-y-4">
        <div class="bg-white p-4 rounded-lg border">
          <h3 class="text-lg font-semibold text-gray-900 mb-2">
            🎯 Pattern Rules ({filteredPatterns.length})
          </h3>
          <p class="text-sm text-gray-600">
            These patterns are automatically applied to evaluate tool and capability access.
          </p>
        </div>
        
        <!-- Pattern Cards Grid -->
        <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {#each filteredPatterns as pattern (pattern.id)}
            <div class="bg-white rounded-lg border hover:shadow-md transition-shadow">
              <!-- Card Header -->
              <div class="p-4 border-b border-gray-100">
                <div class="flex items-center justify-between mb-2">
                  <h4 class="font-medium text-gray-900 truncate">{pattern.name}</h4>
                  <div class="flex items-center space-x-2">
                    <!-- Type Badge -->
                    <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {getTypeBadgeClass(pattern.category)}">
                      {pattern.category}
                    </span>
                    <!-- Action Badge -->
                    <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {getActionBadgeClass(pattern.action)}">
                      {pattern.action === 'allow' ? '✅' : '🚫'}
                    </span>
                  </div>
                </div>
                
                <!-- Status -->
                <div class="flex items-center justify-between">
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {pattern.enabled ? 'bg-green-100 text-green-800' : 'bg-gray-100 text-gray-800'}">
                    {pattern.enabled ? '🟢 Enabled' : '⚪ Disabled'}
                  </span>
                  <span class="text-xs text-gray-500">{pattern.action.toUpperCase()}</span>
                </div>
              </div>
              
              <!-- Card Body -->
              <div class="p-4">
                <!-- Regex Pattern -->
                <div class="mb-3">
                  <label class="text-xs font-medium text-gray-700 block mb-1">Pattern:</label>
                  <div class="text-xs text-gray-600 font-mono bg-gray-50 px-2 py-1 rounded border break-all">
                    {pattern.regex}
                  </div>
                </div>
                
                <!-- Reason -->
                {#if pattern.reason}
                  <div class="mb-3">
                    <label class="text-xs font-medium text-gray-700 block mb-1">Reason:</label>
                    <div class="text-xs text-gray-600">
                      {pattern.reason}
                    </div>
                  </div>
                {/if}
                
                <!-- Scope -->
                <div class="mb-4">
                  <label class="text-xs font-medium text-gray-700 block mb-1">Scope:</label>
                  <div class="text-xs text-gray-600">
                    {pattern.scope}
                  </div>
                </div>
              </div>
              
              <!-- Card Actions -->
              <div class="px-4 py-3 bg-gray-50 border-t border-gray-100 flex justify-end space-x-2">
                <button
                  on:click={() => editPattern(pattern)}
                  class="p-2 text-blue-600 hover:text-blue-800 hover:bg-blue-100 rounded-md transition-colors"
                  title="Edit pattern"
                >
                  ✏️
                </button>
                <button
                  on:click={() => togglePattern(pattern)}
                  class="p-2 text-yellow-600 hover:text-yellow-800 hover:bg-yellow-100 rounded-md transition-colors"
                  title="{pattern.enabled ? 'Disable' : 'Enable'} pattern"
                >
                  {pattern.enabled ? '⏸️' : '▶️'}
                </button>
                <button
                  on:click={() => deletePattern(pattern)}
                  class="p-2 text-red-600 hover:text-red-800 hover:bg-red-100 rounded-md transition-colors"
                  title="Delete pattern"
                >
                  🗑️
                </button>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {:else if !loading && filteredPatterns.length === 0 && patterns}
      <div class="text-center py-12 text-gray-500">
        <div class="text-4xl mb-4">🔍</div>
        <h3 class="text-lg font-medium text-gray-900 mb-2">No patterns found</h3>
        <p class="text-gray-600">Try adjusting your filters or search query.</p>
      </div>
    {/if}
  </div>

  <!-- Edit Pattern Modal -->
  {#if editModalVisible}
    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
      <div class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white">
        <div class="mt-3">
          <h3 class="text-lg font-medium text-gray-900 mb-4">Edit Pattern: {editingPattern?.name}</h3>
          
          <div class="space-y-4">
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Pattern Name</label>
              <input
                type="text"
                bind:value={editForm.name}
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Regex Pattern</label>
              <input
                type="text"
                bind:value={editForm.regex}
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
              />
            </div>
            
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Action</label>
              <select bind:value={editForm.action} class="w-full px-3 py-2 border border-gray-300 rounded-md">
                <option value="allow">Allow</option>
                <option value="deny">Deny</option>
              </select>
            </div>
            
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-1">Reason</label>
              <textarea
                bind:value={editForm.reason}
                rows="2"
                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                placeholder="Why does this pattern exist?"
              ></textarea>
            </div>
            
            <div class="flex items-center">
              <input
                type="checkbox"
                bind:checked={editForm.enabled}
                id="edit-enabled"
                class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label for="edit-enabled" class="ml-2 block text-sm text-gray-900">
                Pattern is enabled
              </label>
            </div>
          </div>
          
          <div class="flex justify-end gap-3 mt-6">
            <button
              type="button"
              on:click={closeEditModal}
              class="px-4 py-2 bg-gray-300 text-gray-700 rounded-md hover:bg-gray-400"
            >
              Cancel
            </button>
            <button
              type="button"
              on:click={savePattern}
              class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700"
            >
              Save Changes
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
{/if}