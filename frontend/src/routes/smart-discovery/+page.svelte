<script lang="ts">
  import { onMount } from 'svelte';
  import type { Tool } from '$lib/api';
  import { api } from '$lib/api';
  import SmartDiscoveryVisualizer from '$lib/components/SmartDiscoveryVisualizer.svelte';

  // Smart Discovery state
  let smartDiscoveryTool: Tool | null = null;
  let discoveryRequest = '';
  let discoveryResult: any = null;
  let discoveryLoading = false;
  let discoveryError = '';
  
  // Execution mode state
  let executionMode = 'http'; // 'http', 'mcp', 'stdio' (simulate Claude)

  async function loadSmartDiscoveryTool() {
    try {
      const toolsData = await api.getTools();
      if (toolsData?.tools) {
        smartDiscoveryTool = toolsData.tools.find(tool => tool.name === 'smart_tool_discovery') || null;
      }
    } catch (err) {
      console.error('Failed to load smart discovery tool:', err);
    }
  }

  async function runSmartDiscovery() {
    if (!discoveryRequest.trim()) {
      discoveryError = 'Please enter a discovery request';
      return;
    }
    
    discoveryLoading = true;
    discoveryError = '';
    discoveryResult = null;
    
    try {
      // Use selected execution mode
      let result;
      if (executionMode === 'mcp') {
        result = await api.executeToolMcp('smart_tool_discovery', {
          request: discoveryRequest.trim(),
          confidence_threshold: 0.5
        });
      } else if (executionMode === 'stdio') {
        result = await api.executeToolStdio('smart_tool_discovery', {
          request: discoveryRequest.trim(),
          confidence_threshold: 0.5
        });
      } else {
        result = await api.executeToolTest('smart_tool_discovery', {
          request: discoveryRequest.trim(),
          confidence_threshold: 0.5
        });
      }
      
      // Handle different response structures between HTTP, MCP, and stdio
      if (executionMode === 'mcp' || executionMode === 'stdio') {
        // MCP mode - extract result and map to expected structure
        const mcpResult = result.result || result;
        
        // Map MCP structure to match what visualizer expects
        discoveryResult = {
          // Extract output from MCP content structure
          output: mcpResult.content?.[0]?.text || 'No output available',
          
          // MCP/stdio doesn't provide these fields, so use fallbacks
          original_tool: `smart_tool_discovery (via ${executionMode.toUpperCase()})`,
          execution_time: 'N/A',
          confidence_score: 0.85, // Default confidence for MCP/stdio calls
          reasoning: `Tool executed successfully via ${executionMode.toUpperCase()} protocol`,
          discovery_reasoning: `Tool executed successfully via ${executionMode.toUpperCase()} protocol`,
          
          // For backward compatibility
          discovered_tool: 'N/A',
          request: discoveryRequest.trim()
        };
      } else {
        // HTTP mode - use result directly
        discoveryResult = result.result || result;
      }
    } catch (err) {
      discoveryError = `Discovery failed: ${err}`;
      console.error('Smart discovery error:', err);
    } finally {
      discoveryLoading = false;
    }
  }

  function clearDiscovery() {
    discoveryRequest = '';
    discoveryResult = null;
    discoveryError = '';
  }

  function handleExecuteTool(event) {
    const { toolName, parameters } = event.detail;
    // Execute the discovered tool
    api.executeToolTest(toolName, parameters).then(result => {
      alert(`Tool executed successfully:\n\n${JSON.stringify(result, null, 2)}`);
    }).catch(err => {
      alert(`Tool execution failed: ${err}`);
    });
  }

  async function handleShowToolDetails(event) {
    const toolName = event.detail;
    console.log('handleShowToolDetails called with tool:', toolName);
    
    try {
      // Load the tool details from the tools API
      console.log('Loading tools from API...');
      const toolsResponse = await api.getTools();
      console.log('Tools response:', toolsResponse);
      
      const tool = toolsResponse.tools.find(t => t.name === toolName);
      console.log('Found tool:', tool);
      
      if (tool) {
        // For now, navigate to tools page - in future could open a modal
        window.location.href = `/tools?service=${encodeURIComponent(toolName)}`;
      } else {
        alert('Tool not found in available tools');
      }
    } catch (err) {
      console.error('Failed to load tool details:', err);
      alert(`Failed to load tool details: ${err}`);
    }
  }

  onMount(() => {
    loadSmartDiscoveryTool();
  });
</script>

<svelte:head>
  <title>Smart Tool Discovery - MagicTunnel</title>
  <meta name="description" content="Intelligent tool discovery and routing with natural language processing for MagicTunnel" />
</svelte:head>

<div class="min-h-screen bg-gray-50 relative">
  <div class="container mx-auto px-4 py-8 relative">
    <!-- Header -->
    <header class="mb-8">
      <div class="mb-2">
        <h1 class="text-4xl font-bold text-primary-700">🧠 Smart Tool Discovery</h1>
      </div>
      <p class="text-gray-600">Describe what you want to do in natural language, and let MagicTunnel find the right tool for you</p>
    </header>

    <!-- Tool Information -->
    {#if smartDiscoveryTool}
      <div class="card mb-8">
        <h3 class="text-xl font-semibold text-gray-700 mb-4">🔧 Tool Information</h3>
        <div class="bg-gray-50 border border-gray-200 rounded-lg p-4">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <h4 class="text-sm font-medium text-gray-800 mb-2">Tool Name</h4>
              <p class="text-gray-900 font-mono text-sm">{smartDiscoveryTool.name}</p>
            </div>
            <div>
              <h4 class="text-sm font-medium text-gray-800 mb-2">Status</h4>
              <span class="px-2 py-1 text-xs rounded-full {smartDiscoveryTool.enabled ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                {smartDiscoveryTool.enabled ? '✓ Enabled' : '✗ Disabled'}
              </span>
            </div>
            <div class="md:col-span-2">
              <h4 class="text-sm font-medium text-gray-800 mb-2">Description</h4>
              <p class="text-gray-700 text-sm leading-relaxed">{smartDiscoveryTool.description}</p>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Discovery Interface -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-6">
        <h3 class="text-xl font-semibold text-gray-700">Discovery Engine</h3>
        <div class="flex items-center gap-4">
          <!-- Execution Mode Selector -->
          <div class="flex items-center gap-2">
            <span class="text-sm text-gray-600">Execution Mode:</span>
            <select 
              bind:value={executionMode} 
              class="px-3 py-2 text-sm bg-white border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 relative z-10"
            >
              <option value="http">🌐 HTTP API</option>
              <option value="mcp">⚡ MCP Client</option>
              <option value="stdio">🤖 Simulate Claude</option>
            </select>
          </div>
          
          <!-- Clear Button -->
          <button
            class="btn-secondary text-sm"
            on:click={clearDiscovery}
            disabled={discoveryLoading}
          >
            🗑️ Clear
          </button>
        </div>
      </div>
      
      <!-- Discovery Input -->
      <div class="mb-6">
        <label class="block text-sm font-medium text-gray-700 mb-3">
          What would you like to do?
        </label>
        <div class="flex gap-3">
          <input
            type="text"
            bind:value={discoveryRequest}
            placeholder="Describe what you want to do (e.g., 'ping google.com', 'check website status', 'search for files', 'get weather for London')"
            class="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-primary-500 focus:border-transparent text-lg"
            on:keypress={(e) => e.key === 'Enter' && runSmartDiscovery()}
            disabled={discoveryLoading}
          />
          <button
            class="btn-primary px-8"
            on:click={runSmartDiscovery}
            disabled={discoveryLoading || !discoveryRequest.trim()}
          >
            {discoveryLoading ? '🔍 Analyzing...' : '🔍 Discover'}
          </button>
        </div>
        
        {#if discoveryError && !discoveryLoading}
          <div class="mt-4 text-sm text-red-600 bg-red-50 border border-red-200 rounded-lg p-4">
            <div class="flex items-center">
              <span class="text-red-500 mr-2">⚠️</span>
              <strong>Discovery Error:</strong>
            </div>
            <p class="mt-1">{discoveryError}</p>
          </div>
        {/if}
      </div>

      <!-- Example Queries -->
      <div class="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6 relative z-0">
        <h4 class="text-sm font-medium text-blue-800 mb-2">💡 Example Queries:</h4>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-2 text-sm text-blue-700">
          <button 
            class="text-left hover:text-blue-900 hover:underline transition-colors"
            on:click={() => discoveryRequest = 'ping google.com'}
            disabled={discoveryLoading}
          >
            • "ping google.com"
          </button>
          <button 
            class="text-left hover:text-blue-900 hover:underline transition-colors"
            on:click={() => discoveryRequest = 'check if a website is up'}
            disabled={discoveryLoading}
          >
            • "check if a website is up"
          </button>
          <button 
            class="text-left hover:text-blue-900 hover:underline transition-colors"
            on:click={() => discoveryRequest = 'search for Python files'}
            disabled={discoveryLoading}
          >
            • "search for Python files"
          </button>
          <button 
            class="text-left hover:text-blue-900 hover:underline transition-colors"
            on:click={() => discoveryRequest = 'get weather information'}
            disabled={discoveryLoading}
          >
            • "get weather information"
          </button>
          <button 
            class="text-left hover:text-blue-900 hover:underline transition-colors"
            on:click={() => discoveryRequest = 'convert text to speech'}
            disabled={discoveryLoading}
          >
            • "convert text to speech"
          </button>
          <button 
            class="text-left hover:text-blue-900 hover:underline transition-colors"
            on:click={() => discoveryRequest = 'analyze network connectivity'}
            disabled={discoveryLoading}
          >
            • "analyze network connectivity"
          </button>
        </div>
      </div>
    </div>

    <!-- Discovery Results -->
    <div class="mb-8">
      <SmartDiscoveryVisualizer
        {discoveryResult}
        isLoading={discoveryLoading}
        error={discoveryError}
        on:execute-tool={handleExecuteTool}
        on:show-tool-details={handleShowToolDetails}
      />
    </div>

  </div>
</div>