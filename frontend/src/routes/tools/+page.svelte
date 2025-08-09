<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Tool, type ToolsResponse } from '$lib/api';
  import ToolExecutionModal from '$lib/components/ToolExecutionModal.svelte';

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
    } catch (err) {
      error = `Failed to load tools: ${err}`;
      console.error('Tools loading error:', err);
    } finally {
      loading = false;
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

<svelte:head>
  <title>Tools - MagicTunnel Dashboard</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <div class="container mx-auto px-4 py-8">
    <!-- Header -->
    <header class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <div class="mb-2">
            <h1 class="text-4xl font-bold text-primary-700">Tools Management</h1>
          </div>
          <p class="text-gray-600">Manage and test available tools in your MagicTunnel instance</p>
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
        
        <!-- Refresh Button Row -->
        <div class="mt-4 flex justify-end">
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
              <div class="card hover:shadow-lg transition-shadow cursor-pointer border-l-4 {selectedTool?.name === tool.name ? 'border-l-primary-500 bg-primary-50' : 'border-l-gray-300'}"
                   on:click={() => selectTool(tool)}>
                <div class="flex items-start justify-between">
                  <div class="flex-1">
                    <div class="flex items-center gap-2 mb-2">
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
                  </div>
                  
                  <button 
                    class="btn-primary text-sm ml-4"
                    on:click|stopPropagation={() => openToolModal(tool)}
                  >
                    🧪 Test
                  </button>
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
            <div class="card sticky top-4">
              <div class="flex items-center justify-between mb-4">
                <h3 class="text-xl font-semibold text-gray-800">Tool Details</h3>
                <button 
                  class="text-gray-400 hover:text-gray-600"
                  on:click={clearSelection}
                >
                  ✕
                </button>
              </div>
              
              <div class="space-y-4">
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
                
                <div class="border-t pt-4">
                  <button 
                    class="btn-primary w-full"
                    on:click={() => selectedTool && openToolModal(selectedTool)}
                    disabled={!selectedTool}
                  >
                    🧪 Test This Tool
                  </button>
                </div>
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

<style>
  .line-clamp-3 {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>