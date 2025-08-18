<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Tool, type ToolsResponse, type ToolManagementResult, type UpdateToolStateRequest } from '$lib/api';
  import ToolExecutionModal from '$lib/components/ToolExecutionModal.svelte';
  import AllowlistControlPanel from '$lib/components/security/AllowlistControlPanel.svelte';
  import type { AllowlistRule } from '$lib/types/security';

  let toolsData: ToolsResponse | null = null;
  let selectedTool: Tool | null = null;
  let loading = true;
  let error = '';
  let searchTerm = '';
  let selectedCategory = 'all';
  let selectedStatus = 'enabled_visible'; // Default to enabled and visible only
  let selectedService = 'all'; // New service filter
  
  // Modal state
  let isModalOpen = false;
  let modalTool: Tool | null = null;
  
  // Execution mode for tools (similar to smart discovery)
  let executionMode = 'http'; // 'http', 'mcp', 'stdio' (simulate Claude)
  
  // Results display state
  let lastExecutionResult: any = null;
  let lastExecutionError: string | null = null;
  let showDebugDetails = false;

  // Tool management state
  let managementMode = false;
  let selectedTools: Set<string> = new Set();
  let bulkActionLoading = false;
  let lastManagementResult: ToolManagementResult | null = null;
  let lastManagementError: string | null = null;
  
  // Allowlist state
  let toolAllowlistRules: Map<string, AllowlistRule | null> = new Map();
  let allowlistLoading = false;
  
  // Helper function to safely parse JSON output
  function parseOutputSafely(output: string) {
    try {
      return JSON.parse(output);
    } catch {
      return { stdout: output, stderr: '', isParsed: false };
    }
  }

  // Categories for filtering (we'll extend this based on actual tool categories)
  const categories = ['all', 'general', 'file', 'network', 'system', 'ai', 'database', 'dev', 'data'];
  const statuses = [
    { value: 'enabled_visible', label: 'Enabled & Visible (Default)' },
    { value: 'all', label: 'All Tools' },
    { value: 'enabled', label: 'Enabled Only' },
    { value: 'disabled', label: 'Disabled Only' },
    { value: 'visible', label: 'Visible Only' },
    { value: 'hidden', label: 'Hidden Only' },
    { value: 'disabled_hidden', label: 'Disabled or Hidden' }
  ];

  async function loadTools() {
    loading = true;
    error = '';
    
    try {
      toolsData = await api.getTools();
      // Load allowlist rules for visible tools
      if (toolsData?.tools) {
        await loadAllowlistRules(toolsData.tools);
      }
    } catch (err) {
      error = `Failed to load tools: ${err}`;
      console.error('Tools loading error:', err);
    } finally {
      loading = false;
    }
  }

  async function loadAllowlistRules(tools: Tool[]) {
    allowlistLoading = true;
    try {
      // Load allowlist rules for all tools
      const rulePromises = tools.map(async (tool) => {
        try {
          const rule = await api.getToolAllowlistRule(tool.name);
          return { toolName: tool.name, rule };
        } catch (err) {
          console.warn(`Failed to load allowlist rule for ${tool.name}:`, err);
          return { toolName: tool.name, rule: null };
        }
      });
      
      const results = await Promise.all(rulePromises);
      const newRulesMap = new Map();
      results.forEach(({ toolName, rule }) => {
        newRulesMap.set(toolName, rule);
      });
      toolAllowlistRules = newRulesMap;
    } catch (err) {
      console.error('Failed to load allowlist rules:', err);
    } finally {
      allowlistLoading = false;
    }
  }

  function openToolModal(tool: Tool) {
    modalTool = tool;
    isModalOpen = true;
  }

  function closeModal() {
    isModalOpen = false;
    modalTool = null;
  }

  async function handleToolExecution(event: CustomEvent) {
    const { tool, arguments: args } = event.detail;
    
    // Clear previous results
    lastExecutionResult = null;
    lastExecutionError = null;
    
    try {
      let result;
      
      switch (executionMode) {
        case 'http':
          result = await api.executeToolTest(tool.name, args);
          break;
        case 'mcp':
          result = await api.executeToolMcp(tool.name, args);
          break;
        case 'stdio':
          result = await api.executeToolStdio(tool.name, args);
          break;
        default:
          throw new Error(`Unknown execution mode: ${executionMode}`);
      }
      
      lastExecutionResult = result;
      closeModal();
    } catch (err) {
      lastExecutionError = `Tool execution failed: ${err}`;
      closeModal();
    }
  }

  function selectTool(tool: Tool) {
    selectedTool = tool;
  }

  function clearSelection() {
    selectedTool = null;
  }

  // Tool management functions
  function toggleManagementMode() {
    managementMode = !managementMode;
    selectedTools.clear();
    selectedTools = selectedTools; // Trigger reactivity
  }

  function toggleToolSelection(toolName: string) {
    if (selectedTools.has(toolName)) {
      selectedTools.delete(toolName);
    } else {
      selectedTools.add(toolName);
    }
    selectedTools = selectedTools; // Trigger reactivity
  }

  function selectAllFilteredTools() {
    filteredTools.forEach(tool => selectedTools.add(tool.name));
    selectedTools = selectedTools;
  }

  function clearSelectedTools() {
    selectedTools.clear();
    selectedTools = selectedTools;
  }

  async function updateToolState(toolName: string, update: UpdateToolStateRequest) {
    try {
      const result = await api.updateToolState(toolName, update);
      lastManagementResult = result;
      lastManagementError = null;
      
      // Refresh tools to show updated state
      await loadTools();
    } catch (err) {
      lastManagementError = `Failed to update tool: ${err}`;
      lastManagementResult = null;
    }
  }

  async function performBulkAction(action: 'enable' | 'disable' | 'show' | 'hide') {
    if (selectedTools.size === 0) {
      lastManagementError = 'No tools selected for bulk action';
      return;
    }

    bulkActionLoading = true;
    lastManagementResult = null;
    lastManagementError = null;

    try {
      const toolNames = Array.from(selectedTools);
      let update: UpdateToolStateRequest = {};

      switch (action) {
        case 'enable':
          update.enabled = true;
          break;
        case 'disable':
          update.enabled = false;
          break;
        case 'show':
          update.hidden = false;
          break;
        case 'hide':
          update.hidden = true;
          break;
      }

      const result = await api.bulkUpdateTools({
        tool_names: toolNames,
        ...update
      });

      lastManagementResult = result;
      
      // Clear selection and refresh tools
      selectedTools.clear();
      selectedTools = selectedTools;
      await loadTools();
    } catch (err) {
      lastManagementError = `Bulk action failed: ${err}`;
    } finally {
      bulkActionLoading = false;
    }
  }

  async function performQuickAction(action: 'hide_all' | 'show_all' | 'enable_all' | 'disable_all') {
    bulkActionLoading = true;
    lastManagementResult = null;
    lastManagementError = null;

    try {
      const result = await api.quickActionTools({ action });
      lastManagementResult = result;
      await loadTools();
    } catch (err) {
      lastManagementError = `Quick action failed: ${err}`;
    } finally {
      bulkActionLoading = false;
    }
  }

  // Allowlist event handlers
  async function handleAllowlistChange(event: CustomEvent) {
    const { itemType, itemName, action, currentRule } = event.detail;
    
    try {
      if (action === 'remove') {
        // Remove the specific rule to use default policy
        if (currentRule) {
          await api.removeToolAllowlistRule(itemName);
          toolAllowlistRules.set(itemName, null);
        }
      } else {
        // Create or update the rule
        const ruleData = {
          type: itemType as 'tool' | 'server' | 'global',
          name: itemName,
          action: action as 'allow' | 'deny',
          enabled: true,
          reason: `${action === 'allow' ? 'Allow' : 'Deny'} access to ${itemName}`
        };
        
        const updatedRule = await api.setToolAllowlistRule(itemName, ruleData);
        toolAllowlistRules.set(itemName, updatedRule);
      }
      
      // Trigger reactivity
      toolAllowlistRules = toolAllowlistRules;
    } catch (err) {
      console.error('Failed to update allowlist rule:', err);
      lastManagementError = `Failed to update allowlist: ${err}`;
    }
  }

  async function handleAllowlistToggle(event: CustomEvent) {
    const { itemType, itemName, enabled, currentRule } = event.detail;
    
    if (!currentRule) return;
    
    try {
      const ruleData = {
        type: currentRule.type,
        name: currentRule.name,
        action: currentRule.action,
        enabled: enabled,
        reason: currentRule.reason
      };
      
      const updatedRule = await api.setToolAllowlistRule(itemName, ruleData);
      toolAllowlistRules.set(itemName, updatedRule);
      
      // Trigger reactivity
      toolAllowlistRules = toolAllowlistRules;
    } catch (err) {
      console.error('Failed to toggle allowlist rule:', err);
      lastManagementError = `Failed to toggle allowlist: ${err}`;
    }
  }

  async function handleAllowlistEdit(event: CustomEvent) {
    const { itemType, itemName, currentRule } = event.detail;
    
    // For now, we'll just log this - in the future we could open a detailed editor
    console.log('Edit allowlist rule for:', itemName, currentRule);
    
    // TODO: Open AllowlistRuleEditor modal
    lastManagementError = 'Rule editing UI will be implemented in a future update';
  }

  // Filter tools based on search, category, and status
  $: filteredTools = toolsData?.tools?.filter(tool => {
    const matchesSearch = tool.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
                         tool.description.toLowerCase().includes(searchTerm.toLowerCase());
    const matchesCategory = selectedCategory === 'all' || tool.category === selectedCategory;
    const matchesStatus = selectedStatus === 'all' ||
                         (selectedStatus === 'enabled' && tool.enabled) ||
                         (selectedStatus === 'disabled' && !tool.enabled) ||
                         (selectedStatus === 'visible' && !tool.hidden) ||
                         (selectedStatus === 'hidden' && tool.hidden) ||
                         (selectedStatus === 'enabled_visible' && tool.enabled && !tool.hidden) ||
                         (selectedStatus === 'disabled_hidden' && (!tool.enabled || tool.hidden));
    const matchesService = selectedService === 'all' || 
                          tool.name.toLowerCase().includes(selectedService.toLowerCase()) ||
                          tool.name.toLowerCase().startsWith(selectedService.toLowerCase() + '_') ||
                          tool.name.toLowerCase().startsWith(selectedService.toLowerCase() + '-') ||
                          (tool.name.startsWith('mcp_') && tool.name.includes(selectedService.toLowerCase())) ||
                          (tool.name.startsWith('external_') && tool.name.includes(selectedService.toLowerCase()));
    return matchesSearch && matchesCategory && matchesStatus && matchesService;
  }) || [];

  $: availableServices = toolsData ? getUniqueServices(toolsData.tools) : [];

  // Parse URL parameters for service filtering
  function parseUrlParameters() {
    const urlParams = new URLSearchParams(window.location.search);
    const serviceParam = urlParams.get('service');
    if (serviceParam) {
      selectedService = serviceParam;
      // When filtering by service, show all tools by default to ensure visibility
      selectedStatus = 'all';
    }
  }

  // Get unique services from tools for filtering
  function getUniqueServices(tools: Tool[]): string[] {
    const services = new Set<string>();
    tools.forEach(tool => {
      // Check if tool name indicates external service (common patterns)
      if (tool.name.includes('_') || tool.name.includes('-')) {
        const parts = tool.name.split(/[_-]/);
        if (parts.length > 1) {
          services.add(parts[0]);
        }
      }
      
      // Also check for common external MCP service names
      const commonServices = ['globalping', 'filesystem', 'brave', 'search', 'weather', 'memory'];
      commonServices.forEach(serviceName => {
        if (tool.name.toLowerCase().includes(serviceName)) {
          services.add(serviceName);
        }
      });
      
      // Check if tool.name starts with known external service patterns
      if (tool.name.startsWith('mcp_') || tool.name.startsWith('external_')) {
        const servicePart = tool.name.split('_')[1];
        if (servicePart) {
          services.add(servicePart);
        }
      }
    });
    return Array.from(services).sort();
  }

  onMount(() => {
    parseUrlParameters();
    loadTools();
  });
</script>

<style>
  .dot {
    top: 50%;
    left: 2px;
    transform: translateY(-50%);
  }
  
  .dot.translate-x-6 {
    transform: translate(24px, -50%);
  }
  
  /* Hover effects for toggles */
  .toggle-container:hover .dot {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  }
  
  /* Improved button hover states */
  .btn-modern {
    @apply px-4 py-2 rounded-lg font-medium shadow-sm transition-all duration-200 flex items-center gap-2;
  }
  
  .btn-modern:disabled {
    @apply opacity-50 cursor-not-allowed;
  }
  
  .btn-modern:focus {
    @apply ring-2 ring-offset-2 ring-opacity-50;
  }
  
  .line-clamp-3 {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  
  /* Custom scrollbar styling for tool details panel */
  .overflow-y-auto {
    scrollbar-width: thin;
    scrollbar-color: #cbd5e1 #f1f5f9;
  }
  
  .overflow-y-auto::-webkit-scrollbar {
    width: 6px;
  }
  
  .overflow-y-auto::-webkit-scrollbar-track {
    background: #f1f5f9;
    border-radius: 3px;
  }
  
  .overflow-y-auto::-webkit-scrollbar-thumb {
    background: #cbd5e1;
    border-radius: 3px;
  }
  
  .overflow-y-auto::-webkit-scrollbar-thumb:hover {
    background: #94a3b8;
  }
</style>

<svelte:head>
  <title>Tools - MagicTunnel Dashboard</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <div class="container mx-auto px-4 py-8">
    <!-- Header -->
    <header class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-4xl font-bold text-primary-700 mb-2">Tools Management</h1>
          <p class="text-gray-600">
            {managementMode ? 'Select tools to enable/disable or hide/show them dynamically' : 'Manage and test available tools in your MagicTunnel instance'}
          </p>
          {#if selectedService !== 'all'}
            <div class="mt-2 p-2 bg-blue-50 border border-blue-200 rounded-md">
              <span class="text-sm text-blue-700">
                🔌 Filtering by service: <span class="font-medium">{selectedService}</span>
              </span>
              <button 
                class="ml-3 text-xs text-blue-600 hover:text-blue-800 underline"
                on:click={() => { selectedService = 'all'; selectedStatus = 'enabled_visible'; }}
              >
                Clear filter
              </button>
            </div>
          {/if}
        </div>
      </div>
      
      {#if loading}
        <div class="mt-4 text-sm text-blue-600">🔄 Loading tools...</div>
      {/if}
      
      {#if error}
        <div class="mt-4 text-sm text-red-600">❌ {error}</div>
      {/if}
    </header>

    {#if toolsData}
      <!-- Stats Section -->
      <div class="card mb-4">
        <div class="flex flex-wrap items-center gap-6">
          <div>
            <span class="text-3xl font-bold text-primary-600">{toolsData.total}</span>
            <span class="text-gray-600 ml-2">Total Tools</span>
          </div>
          <div>
            <span class="text-3xl font-bold text-green-600">{filteredTools.length}</span>
            <span class="text-gray-600 ml-2">Filtered</span>
          </div>
          
          <!-- Execution Mode Selector -->
          <div class="flex items-center gap-3 ml-auto">
            <label class="text-sm font-medium text-gray-700">Execution Mode:</label>
            <select
              bind:value={executionMode}
              class="px-3 py-2 text-sm border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            >
              <option value="http">🌐 HTTP API</option>
              <option value="mcp">⚡ MCP Client</option>
              <option value="stdio">🤖 Simulate Claude</option>
            </select>
          </div>
        </div>
      </div>

      <!-- Management Results Display -->
      {#if lastManagementResult || lastManagementError}
        <div class="card mb-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-gray-700">
              {lastManagementError ? '❌ Management Action Failed' : '✅ Management Action Successful'}
            </h3>
            <button
              class="text-gray-400 hover:text-gray-600"
              on:click={() => { lastManagementResult = null; lastManagementError = null; }}
            >
              ✕
            </button>
          </div>

          {#if lastManagementError}
            <div class="bg-red-50 border border-red-200 rounded-lg p-4">
              <p class="text-red-800 font-medium mb-2">Error</p>
              <p class="text-red-700 text-sm">{lastManagementError}</p>
            </div>
          {:else if lastManagementResult}
            <div class="bg-green-50 border border-green-200 rounded-lg p-4">
              <p class="text-green-800 font-medium mb-2">{lastManagementResult.message}</p>
              {#if lastManagementResult.affected_tools.length > 0 && lastManagementResult.affected_tools.length <= 10}
                <div class="text-green-700 text-sm">
                  <span class="font-medium">Affected tools:</span>
                  {lastManagementResult.affected_tools.join(', ')}
                </div>
              {:else if lastManagementResult.total_affected > 0}
                <div class="text-green-700 text-sm">
                  <span class="font-medium">Total tools affected:</span>
                  {lastManagementResult.total_affected}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Management Toolbar (only in management mode) -->
      {#if managementMode}
        <div class="card mb-6">
          <div class="flex flex-wrap items-center justify-between gap-4">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-gray-700">
                {selectedTools.size} tool{selectedTools.size !== 1 ? 's' : ''} selected
              </span>
              <button 
                class="btn-secondary text-sm" 
                on:click={selectAllFilteredTools}
                disabled={filteredTools.length === 0}
              >
                Select All ({filteredTools.length})
              </button>
              <button 
                class="btn-secondary text-sm" 
                on:click={clearSelectedTools}
                disabled={selectedTools.size === 0}
              >
                Clear Selection
              </button>
            </div>

            <div class="flex items-center gap-2">
              <!-- Bulk Actions -->
              <button 
                class="bg-emerald-600 hover:bg-emerald-700 text-white px-4 py-2 rounded-lg text-sm font-medium shadow-sm transition-all duration-200 flex items-center gap-2"
                on:click={() => performBulkAction('enable')}
                disabled={selectedTools.size === 0 || bulkActionLoading}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                </svg>
                Enable
              </button>
              <button 
                class="bg-red-500 hover:bg-red-600 text-white px-4 py-2 rounded-lg text-sm font-medium shadow-sm transition-all duration-200 flex items-center gap-2"
                on:click={() => performBulkAction('disable')}
                disabled={selectedTools.size === 0 || bulkActionLoading}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
                Disable
              </button>
              <button 
                class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm font-medium shadow-sm transition-all duration-200 flex items-center gap-2"
                on:click={() => performBulkAction('show')}
                disabled={selectedTools.size === 0 || bulkActionLoading}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
                </svg>
                Show
              </button>
              <button 
                class="bg-amber-500 hover:bg-amber-600 text-white px-4 py-2 rounded-lg text-sm font-medium shadow-sm transition-all duration-200 flex items-center gap-2"
                on:click={() => performBulkAction('hide')}
                disabled={selectedTools.size === 0 || bulkActionLoading}
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.878 9.878L3 3m6.878 6.878L15 15M21 3l-6.878 6.878"></path>
                </svg>
                Hide
              </button>

              <!-- Quick Actions -->
              <div class="border-l border-gray-300 pl-3 ml-3">
                <span class="text-xs text-gray-500 mr-3 font-medium">Quick Actions:</span>
                <button 
                  class="bg-slate-600 hover:bg-slate-700 text-white px-3 py-1.5 rounded-md text-xs font-medium shadow-sm transition-all duration-200"
                  on:click={() => performQuickAction('enable_all')}
                  disabled={bulkActionLoading}
                >
                  Enable All
                </button>
                <button 
                  class="bg-slate-600 hover:bg-slate-700 text-white px-3 py-1.5 rounded-md text-xs font-medium shadow-sm transition-all duration-200 ml-2"
                  on:click={() => performQuickAction('show_all')}
                  disabled={bulkActionLoading}
                >
                  Show All
                </button>
              </div>
            </div>
          </div>
        </div>
      {/if}

      <!-- Search and Filters Section -->
      <div class="card mb-6">
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          <!-- Search -->
          <div class="sm:col-span-2 lg:col-span-2 xl:col-span-2">
            <input
              type="text"
              placeholder="Search tools..."
              bind:value={searchTerm}
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>
          
          <!-- Category Filter -->
          <select
            bind:value={selectedCategory}
            class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          >
            {#each categories as category}
              <option value={category}>
                {category === 'all' ? 'All Categories' : category.charAt(0).toUpperCase() + category.slice(1)}
              </option>
            {/each}
          </select>
          
          <!-- Status Filter -->
          <select
            bind:value={selectedStatus}
            class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          >
            {#each statuses as status}
              <option value={status.value}>
                {status.label}
              </option>
            {/each}
          </select>

          <!-- Service Filter -->
          <select
            bind:value={selectedService}
            class="px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          >
            <option value="all">All Services</option>
            {#each availableServices as service}
              <option value={service}>
                {service.charAt(0).toUpperCase() + service.slice(1)} Service
              </option>
            {/each}
          </select>
        </div>
        
        <!-- Action Buttons Row -->
        <div class="mt-4 flex justify-between items-center">
          <button 
            class="bg-gradient-to-r from-primary-600 to-primary-700 hover:from-primary-700 hover:to-primary-800 text-white px-4 py-2 rounded-lg font-medium shadow-sm transition-all duration-200 flex items-center gap-2"
            on:click={toggleManagementMode}
          >
            {#if managementMode}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"></path>
              </svg>
              View Mode
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
              </svg>
              Manage Mode
            {/if}
          </button>
          
          <button class="btn-secondary" on:click={loadTools} disabled={loading}>
            {loading ? '🔄 Loading...' : '🔄 Refresh'}
          </button>
        </div>
      </div>

      <!-- Results Display -->
      {#if lastExecutionResult || lastExecutionError}
        <div class="card mb-6">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-gray-700">
              {lastExecutionError ? '❌ Execution Failed' : '✅ Execution Result'}
            </h3>
            <div class="flex items-center gap-3">
              <span class="text-sm text-gray-500">
                Mode: {executionMode === 'http' ? '🌐 HTTP API' : executionMode === 'mcp' ? '⚡ MCP Client' : '🤖 Simulate Claude'}
              </span>
              <button
                class="text-sm text-gray-500 hover:text-gray-700 underline"
                on:click={() => showDebugDetails = !showDebugDetails}
              >
                {showDebugDetails ? 'Hide' : 'Show'} Debug
              </button>
              <button
                class="text-gray-400 hover:text-gray-600"
                on:click={() => { lastExecutionResult = null; lastExecutionError = null; }}
              >
                ✕
              </button>
            </div>
          </div>

          {#if lastExecutionError}
            <div class="bg-red-50 border border-red-200 rounded-lg p-4">
              <p class="text-red-800 font-medium mb-2">Error</p>
              <p class="text-red-700 text-sm">{lastExecutionError}</p>
            </div>
          {:else if lastExecutionResult}
            <!-- Success Result -->
            <div class="bg-green-50 border border-green-200 rounded-lg p-4">
              {#if lastExecutionResult.content}
                <!-- MCP-style response with content array -->
                <div class="mb-4">
                  <h4 class="text-green-800 font-medium mb-2">Content</h4>
                  <div class="bg-white p-3 rounded border text-sm">
                    {#each lastExecutionResult.content as contentItem}
                      {#if contentItem.type === 'text'}
                        <pre class="whitespace-pre-wrap text-gray-800">{contentItem.text}</pre>
                      {:else}
                        <div class="text-gray-600 italic">
                          [{contentItem.type}]: {JSON.stringify(contentItem, null, 2)}
                        </div>
                      {/if}
                    {/each}
                  </div>
                </div>
              {:else if lastExecutionResult.result?.output}
                <!-- HTTP API style response with result.output -->
                <div class="mb-4">
                  <h4 class="text-green-800 font-medium mb-2">Output</h4>
                  <div class="bg-white p-3 rounded border text-sm">
                    {#if lastExecutionResult.result.status === 'success'}
                      <div class="mb-2">
                        <span class="inline-flex items-center px-2 py-1 bg-green-100 text-green-800 text-xs font-medium rounded-full">
                          ✅ Success ({lastExecutionResult.result.execution_time})
                        </span>
                      </div>
                      {@const parsedOutput = parseOutputSafely(lastExecutionResult.result.output)}
                      <pre class="whitespace-pre-wrap text-gray-800">{parsedOutput.stdout || lastExecutionResult.result.output}</pre>
                      {#if parsedOutput.stderr}
                        <div class="mt-3 p-2 bg-yellow-50 border border-yellow-200 rounded">
                          <h5 class="text-yellow-800 font-medium text-sm mb-1">Standard Error:</h5>
                          <pre class="text-yellow-700 text-xs">{parsedOutput.stderr}</pre>
                        </div>
                      {/if}
                    {:else}
                      <pre class="whitespace-pre-wrap text-red-800">{lastExecutionResult.result.output}</pre>
                    {/if}
                  </div>
                </div>
              {:else if lastExecutionResult.result}
                <!-- Generic result display -->
                <div class="mb-4">
                  <h4 class="text-green-800 font-medium mb-2">Result</h4>
                  <div class="bg-white p-3 rounded border text-sm">
                    <pre class="whitespace-pre-wrap text-gray-800">{JSON.stringify(lastExecutionResult.result, null, 2)}</pre>
                  </div>
                </div>
              {:else}
                <!-- Fallback for unknown structure -->
                <div class="mb-4">
                  <h4 class="text-green-800 font-medium mb-2">Response</h4>
                  <div class="bg-white p-3 rounded border text-sm">
                    <pre class="whitespace-pre-wrap text-gray-800">{JSON.stringify(lastExecutionResult, null, 2)}</pre>
                  </div>
                </div>
              {/if}

              <!-- Debug Details (Collapsible) -->
              {#if showDebugDetails}
                <details class="mt-4">
                  <summary class="cursor-pointer text-green-700 font-medium mb-2">
                    🔍 Debug Information
                  </summary>
                  <div class="bg-gray-50 p-3 rounded border mt-2">
                    <pre class="text-xs text-gray-700 overflow-auto">{JSON.stringify(lastExecutionResult, null, 2)}</pre>
                  </div>
                </details>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Tools Grid -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Tools List -->
        <div class="lg:col-span-2">
          <div class="space-y-4">
            {#each filteredTools as tool}
              <div class="card hover:shadow-lg transition-shadow border-l-4 {selectedTool?.name === tool.name ? 'border-l-primary-500 bg-primary-50' : managementMode && selectedTools.has(tool.name) ? 'border-l-blue-500 bg-blue-50' : 'border-l-gray-300'}"
                   class:cursor-pointer={!managementMode}
                   on:click={() => managementMode ? toggleToolSelection(tool.name) : selectTool(tool)}>
                <div class="flex items-start justify-between">
                  <div class="flex-1">
                    <div class="flex items-center gap-2 mb-2">
                      {#if managementMode}
                        <input 
                          type="checkbox" 
                          checked={selectedTools.has(tool.name)}
                          on:click|stopPropagation
                          on:change={() => toggleToolSelection(tool.name)}
                          class="mr-2"
                        />
                      {/if}
                      <h3 class="text-lg font-semibold text-gray-800">{tool.name}</h3>
                      <span class="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded-full">
                        {tool.category}
                      </span>
                      <!-- Status badges -->
                      <span class="px-2 py-1 text-xs rounded-full {tool.enabled ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                        {tool.enabled ? '✓ Enabled' : '✗ Disabled'}
                      </span>
                      {#if tool.hidden}
                        <span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-800 rounded-full">
                          🔒 Hidden
                        </span>
                      {/if}
                    </div>
                    <p class="text-gray-600 text-sm leading-relaxed line-clamp-3">
                      {tool.description.substring(0, 200)}{tool.description.length > 200 ? '...' : ''}
                    </p>
                    
                    <!-- Tool Stats -->
                    <div class="flex items-center gap-4 mt-3 text-xs text-gray-500">
                      <span>📊 Success Rate: {tool.success_rate || 'N/A'}</span>
                      <span>🕒 Last Used: {tool.last_used || 'Never'}</span>
                    </div>
                    
                    <!-- Allowlist Control (Compact View) -->
                    {#if !managementMode}
                      <div class="mt-3 pt-3 border-t border-gray-200">
                        {#if allowlistLoading}
                          <div class="flex items-center gap-2 text-xs text-gray-500">
                            <div class="animate-spin rounded-full h-3 w-3 border-b-2 border-gray-400"></div>
                            Loading allowlist rules...
                          </div>
                        {:else}
                          <AllowlistControlPanel
                            itemType="tool"
                            itemName={tool.name}
                            currentRule={toolAllowlistRules.get(tool.name) || null}
                            compact={true}
                            on:allowlist-change={handleAllowlistChange}
                            on:allowlist-toggle={handleAllowlistToggle}
                            on:allowlist-edit={handleAllowlistEdit}
                          />
                        {/if}
                      </div>
                    {/if}
                  </div>
                  
                  {#if managementMode}
                    <div class="flex flex-col gap-4 ml-6">
                      <!-- Enable/Disable Toggle Switch -->
                      <div class="flex items-center gap-3">
                        <label class="flex items-center cursor-pointer group" on:click|stopPropagation>
                          <div class="relative toggle-container">
                            <input 
                              type="checkbox" 
                              checked={tool.enabled}
                              on:click|stopPropagation
                              on:change={() => updateToolState(tool.name, { enabled: !tool.enabled })}
                              class="sr-only"
                            />
                            <div class="w-11 h-6 bg-gray-300 rounded-full shadow-inner transition-colors duration-200 {tool.enabled ? 'bg-emerald-500' : 'bg-gray-300'}"></div>
                            <div class="dot absolute w-4 h-4 bg-white rounded-full shadow transition-transform duration-200 {tool.enabled ? 'translate-x-6' : 'translate-x-0'}"></div>
                          </div>
                          <span class="ml-2 text-sm font-medium {tool.enabled ? 'text-emerald-700' : 'text-gray-600'}">
                            {tool.enabled ? 'Enabled' : 'Disabled'}
                          </span>
                        </label>
                      </div>
                      
                      <!-- Visible/Hidden Toggle Switch -->
                      <div class="flex items-center gap-3">
                        <label class="flex items-center cursor-pointer group" on:click|stopPropagation>
                          <div class="relative toggle-container">
                            <input 
                              type="checkbox" 
                              checked={!tool.hidden}
                              on:click|stopPropagation
                              on:change={() => updateToolState(tool.name, { hidden: !tool.hidden })}
                              class="sr-only"
                            />
                            <div class="w-11 h-6 bg-gray-300 rounded-full shadow-inner transition-colors duration-200 {!tool.hidden ? 'bg-blue-500' : 'bg-amber-500'}"></div>
                            <div class="dot absolute w-4 h-4 bg-white rounded-full shadow transition-transform duration-200 {!tool.hidden ? 'translate-x-6' : 'translate-x-0'}"></div>
                          </div>
                          <span class="ml-2 text-sm font-medium {!tool.hidden ? 'text-blue-700' : 'text-amber-700'}">
                            {tool.hidden ? 'Hidden' : 'Visible'}
                          </span>
                        </label>
                      </div>
                    </div>
                  {:else}
                    <button 
                      class="btn-primary text-sm ml-4"
                      on:click|stopPropagation={() => openToolModal(tool)}
                    >
                      🧪 Test
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
            
            {#if filteredTools.length === 0}
              <div class="card text-center py-12">
                <div class="text-gray-400 text-6xl mb-4">🔍</div>
                <h3 class="text-lg font-medium text-gray-600 mb-2">No tools found</h3>
                <p class="text-gray-500">
                  {searchTerm || selectedCategory !== 'all' 
                    ? 'Try adjusting your search or filter criteria.' 
                    : 'No tools are currently available.'}
                </p>
              </div>
            {/if}
          </div>
        </div>

        <!-- Tool Details Panel -->
        <div class="lg:col-span-1">
          {#if selectedTool}
            <div class="card sticky top-4 max-h-[85vh] flex flex-col">
              <div class="flex items-center justify-between mb-4 flex-shrink-0">
                <h3 class="text-xl font-semibold text-gray-800">Tool Details</h3>
                <button 
                  class="text-gray-400 hover:text-gray-600"
                  on:click={clearSelection}
                >
                  ✕
                </button>
              </div>
              
              <div class="space-y-4 overflow-y-auto flex-1 pr-2 -mr-2 pb-2">
                <div>
                  <label class="block text-sm font-medium text-gray-700 mb-1">Name</label>
                  <p class="text-gray-900 font-mono text-sm bg-gray-50 p-2 rounded">
                    {selectedTool.name}
                  </p>
                </div>
                
                <div>
                  <label class="block text-sm font-medium text-gray-700 mb-1">Category & Status</label>
                  <div class="flex flex-wrap gap-2">
                    <span class="px-3 py-1 text-sm bg-primary-100 text-primary-800 rounded-full">
                      {selectedTool.category}
                    </span>
                    <span class="px-3 py-1 text-sm rounded-full {selectedTool.enabled ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                      {selectedTool.enabled ? '✓ Enabled' : '✗ Disabled'}
                    </span>
                    <span class="px-3 py-1 text-sm rounded-full {selectedTool.hidden ? 'bg-yellow-100 text-yellow-800' : 'bg-blue-100 text-blue-800'}">
                      {selectedTool.hidden ? '🔒 Hidden' : '👁 Visible'}
                    </span>
                  </div>
                </div>
                
                <div>
                  <label class="block text-sm font-medium text-gray-700 mb-1">Description</label>
                  <p class="text-gray-700 text-sm leading-relaxed bg-gray-50 p-3 rounded">
                    {selectedTool.description}
                  </p>
                </div>
                
                <div>
                  <label class="block text-sm font-medium text-gray-700 mb-1">Input Schema</label>
                  <pre class="text-xs bg-gray-50 p-3 rounded overflow-auto max-h-64 text-gray-700">
{JSON.stringify(selectedTool.input_schema, null, 2)}</pre>
                </div>
                
                {#if !managementMode}
                  <!-- Allowlist Control (Full View) -->
                  <div>
                    {#if allowlistLoading}
                      <div class="bg-gray-50 border border-gray-200 rounded-lg p-4">
                        <div class="flex items-center gap-3">
                          <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-400"></div>
                          <span class="text-sm text-gray-600">Loading allowlist rules...</span>
                        </div>
                      </div>
                    {:else}
                      <AllowlistControlPanel
                        itemType="tool"
                        itemName={selectedTool.name}
                        currentRule={toolAllowlistRules.get(selectedTool.name) || null}
                        compact={false}
                        on:allowlist-change={handleAllowlistChange}
                        on:allowlist-toggle={handleAllowlistToggle}
                        on:allowlist-edit={handleAllowlistEdit}
                      />
                    {/if}
                  </div>
                {/if}
              </div>
              
              <!-- Action buttons - always visible at bottom -->
              <div class="border-t pt-4 mt-4 pb-2 flex-shrink-0 bg-white">
                {#if managementMode}
                  <div class="space-y-4">
                    <!-- Enable/Disable Toggle Switch -->
                    <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                      <span class="text-sm font-medium text-gray-700">Tool Status</span>
                      <label class="flex items-center cursor-pointer" on:click|stopPropagation>
                        <div class="relative toggle-container">
                          <input 
                            type="checkbox" 
                            checked={selectedTool.enabled}
                            on:click|stopPropagation
                            on:change={() => selectedTool && updateToolState(selectedTool.name, { enabled: !selectedTool.enabled })}
                            class="sr-only"
                          />
                          <div class="w-11 h-6 bg-gray-300 rounded-full shadow-inner transition-colors duration-200 {selectedTool.enabled ? 'bg-emerald-500' : 'bg-gray-300'}"></div>
                          <div class="dot absolute w-4 h-4 bg-white rounded-full shadow transition-transform duration-200 {selectedTool.enabled ? 'translate-x-6' : 'translate-x-0'}"></div>
                        </div>
                        <span class="ml-3 text-sm font-medium {selectedTool.enabled ? 'text-emerald-700' : 'text-gray-600'}">
                          {selectedTool.enabled ? 'Enabled' : 'Disabled'}
                        </span>
                      </label>
                    </div>
                    
                    <!-- Visible/Hidden Toggle Switch -->
                    <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                      <span class="text-sm font-medium text-gray-700">Visibility</span>
                      <label class="flex items-center cursor-pointer" on:click|stopPropagation>
                        <div class="relative toggle-container">
                          <input 
                            type="checkbox" 
                            checked={!selectedTool.hidden}
                            on:click|stopPropagation
                            on:change={() => selectedTool && updateToolState(selectedTool.name, { hidden: !selectedTool.hidden })}
                            class="sr-only"
                          />
                          <div class="w-11 h-6 bg-gray-300 rounded-full shadow-inner transition-colors duration-200 {!selectedTool.hidden ? 'bg-blue-500' : 'bg-amber-500'}"></div>
                          <div class="dot absolute w-4 h-4 bg-white rounded-full shadow transition-transform duration-200 {!selectedTool.hidden ? 'translate-x-6' : 'translate-x-0'}"></div>
                        </div>
                        <span class="ml-3 text-sm font-medium {!selectedTool.hidden ? 'text-blue-700' : 'text-amber-700'}">
                          {selectedTool.hidden ? 'Hidden' : 'Visible'}
                        </span>
                      </label>
                    </div>
                    
                    <button 
                      class="btn-secondary w-full text-sm"
                      on:click={() => selectedTool && toggleToolSelection(selectedTool.name)}
                    >
                      {selectedTools.has(selectedTool.name) ? 'Remove from Selection' : 'Add to Selection'}
                    </button>
                  </div>
                {:else}
                  <button 
                    class="btn-primary w-full"
                    on:click={() => selectedTool && openToolModal(selectedTool)}
                    disabled={!selectedTool}
                  >
                    🧪 Test This Tool
                  </button>
                {/if}
              </div>
            </div>
          {:else}
            <div class="card text-center py-12">
              <div class="text-gray-300 text-4xl mb-4">🔧</div>
              <h3 class="text-lg font-medium text-gray-600 mb-2">Select a Tool</h3>
              <p class="text-gray-500 text-sm">
                Click on any tool from the list to view its details and test it.
              </p>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- Tool Execution Modal -->
<ToolExecutionModal 
  bind:isOpen={isModalOpen} 
  tool={modalTool}
  {executionMode}
  on:close={closeModal}
  on:execute={handleToolExecution}
/>

