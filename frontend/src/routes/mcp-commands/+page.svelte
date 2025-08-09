<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type McpExecuteResponse } from '$lib/api';

  // MCP testing state
  let mcpMethod = '';
  let mcpParams = '';
  let executingMcpCommand = false;
  let mcpResult: McpExecuteResponse | null = null;

  async function executeMcpCommand() {
    if (!mcpMethod.trim() || executingMcpCommand) return;

    executingMcpCommand = true;
    mcpResult = null;

    try {
      let params = null;
      if (mcpParams.trim()) {
        try {
          params = JSON.parse(mcpParams);
        } catch (e) {
          throw new Error(`Invalid JSON params: ${e}`);
        }
      }

      mcpResult = await api.executeMcpCommand(mcpMethod.trim(), params);
    } catch (err) {
      console.error('MCP command execution failed:', err);
      mcpResult = {
        action: 'execute_mcp',
        method: mcpMethod.trim(),
        status: 'error',
        message: `Failed to execute MCP command: ${err}`,
        timestamp: new Date().toISOString()
      };
    }
    executingMcpCommand = false;
  }

  function clearForm() {
    mcpMethod = '';
    mcpParams = '';
    mcpResult = null;
  }

  // Common MCP method templates
  const methodTemplates = [
    {
      name: 'List Tools',
      method: 'tools/list',
      params: '',
      description: 'Get all available tools'
    },
    {
      name: 'Call Tool',
      method: 'tools/call',
      params: '{\n  "name": "example_tool",\n  "arguments": {\n    "param": "value"\n  }\n}',
      description: 'Execute a specific tool'
    },
    {
      name: 'Get Capabilities',
      method: 'capabilities',
      params: '',
      description: 'Get server capabilities'
    },
    {
      name: 'List Resources',
      method: 'resources/list',
      params: '',
      description: 'Get all available resources'
    },
    {
      name: 'Initialize',
      method: 'initialize',
      params: '{\n  "protocolVersion": "2024-11-05",\n  "capabilities": {}\n}',
      description: 'Initialize MCP connection'
    }
  ];

  function useTemplate(template: typeof methodTemplates[0]) {
    mcpMethod = template.method;
    mcpParams = template.params;
    mcpResult = null;
  }
</script>

<svelte:head>
  <title>MCP Commands - MagicTunnel</title>
  <meta name="description" content="Test and execute MCP protocol commands directly for debugging and development" />
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <div class="container mx-auto px-4 py-8">
    <!-- Header -->
    <header class="mb-8">
      <div class="mb-2">
        <h1 class="text-4xl font-bold text-primary-700">🔧 MCP Commands</h1>
      </div>
      <p class="text-gray-600">Test and execute Model Context Protocol (MCP) commands directly for debugging and development</p>
    </header>

    <!-- Quick Templates -->
    <div class="card mb-8">
      <h3 class="text-xl font-semibold text-gray-700 mb-4">🚀 Quick Templates</h3>
      <p class="text-gray-600 mb-4">Use these common MCP command templates to get started quickly.</p>
      
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each methodTemplates as template}
          <button
            class="text-left p-4 bg-white border border-gray-200 hover:border-blue-300 hover:shadow-md rounded-lg transition-all duration-200"
            on:click={() => useTemplate(template)}
            disabled={executingMcpCommand}
          >
            <div class="font-medium text-gray-800 mb-1">{template.name}</div>
            <div class="text-sm font-mono text-blue-600 mb-2">{template.method}</div>
            <div class="text-xs text-gray-500">{template.description}</div>
          </button>
        {/each}
      </div>
    </div>

    <!-- MCP Command Interface -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-6">
        <h3 class="text-xl font-semibold text-gray-700">Command Interface</h3>
        <button
          class="btn-secondary"
          on:click={clearForm}
          disabled={executingMcpCommand}
        >
          🗑️ Clear All
        </button>
      </div>
      
      <div class="space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">
            MCP Method <span class="text-red-500">*</span>
          </label>
          <input
            type="text"
            bind:value={mcpMethod}
            placeholder="e.g., tools/list, tools/call, initialize"
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            disabled={executingMcpCommand}
          />
          <p class="text-xs text-gray-500 mt-1">
            Common methods: tools/list, tools/call, capabilities, resources/list, initialize
          </p>
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">
            Parameters (JSON, optional)
          </label>
          <textarea
            bind:value={mcpParams}
            placeholder="{`{\"name\": \"example_tool\", \"arguments\": {\"param\": \"value\"}}`}"
            rows="6"
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm"
            disabled={executingMcpCommand}
          ></textarea>
          <p class="text-xs text-gray-500 mt-1">
            Leave empty for methods without parameters. Must be valid JSON if provided.
          </p>
        </div>

        <div class="flex gap-3">
          <button
            class="btn-primary"
            on:click={executeMcpCommand}
            disabled={!mcpMethod.trim() || executingMcpCommand}
          >
            {executingMcpCommand ? '⏳ Executing...' : '🚀 Execute MCP Command'}
          </button>
          
          <button
            class="btn-secondary"
            on:click={clearForm}
            disabled={executingMcpCommand}
          >
            🗑️ Clear
          </button>
        </div>
      </div>
    </div>

    <!-- Results Section -->
    {#if mcpResult}
      <div class="card mb-8">
        <div class="flex items-center justify-between mb-6">
          <h3 class="text-xl font-semibold text-gray-700">Command Results</h3>
          <span class="text-sm text-gray-500">{mcpResult.timestamp}</span>
        </div>
        
        <div class="space-y-4">
          <!-- Status Badge -->
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium text-gray-600">Status:</span>
            <span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {mcpResult.status === 'success' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
              {mcpResult.status === 'success' ? '✅' : '❌'} {mcpResult.status.toUpperCase()}
            </span>
          </div>

          <!-- Method and Action -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <span class="text-sm font-medium text-gray-600">Method:</span>
              <code class="ml-2 px-2 py-1 bg-gray-100 rounded text-sm">{mcpResult.method}</code>
            </div>
            <div>
              <span class="text-sm font-medium text-gray-600">Action:</span>
              <code class="ml-2 px-2 py-1 bg-gray-100 rounded text-sm">{mcpResult.action}</code>
            </div>
          </div>

          <!-- Error Message (if any) -->
          {#if mcpResult.status === 'error' && mcpResult.message}
            <div class="p-4 bg-red-50 rounded-md border-l-4 border-red-200">
              <div class="text-sm font-medium text-red-800 mb-1">Error Message</div>
              <div class="text-sm text-red-700">{mcpResult.message}</div>
            </div>
          {/if}

          <!-- Response Data -->
          {#if mcpResult.response}
            <div>
              <div class="text-sm font-medium text-gray-600 mb-2">Response Data:</div>
              <div class="bg-gray-50 rounded-md p-4 border">
                <pre class="text-sm font-mono text-gray-800 whitespace-pre-wrap overflow-x-auto">{JSON.stringify(mcpResult.response, null, 2)}</pre>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {:else}
      <div class="card text-center py-12">
        <div class="text-4xl mb-4">🔧</div>
        <h3 class="text-lg font-medium text-gray-700 mb-2">Ready to Execute</h3>
        <p class="text-gray-600">Enter an MCP method above and click "Execute MCP Command" to see results here</p>
      </div>
    {/if}

    <!-- Help & Documentation -->
    <div class="card">
      <h3 class="text-xl font-semibold text-gray-700 mb-4">📚 MCP Protocol Reference</h3>
      <div class="space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <h4 class="text-lg font-medium text-gray-700 mb-3">Common Methods</h4>
            <div class="space-y-2">
              <div class="text-sm">
                <code class="bg-blue-100 text-blue-800 px-2 py-1 rounded">tools/list</code>
                <span class="ml-2 text-gray-600">List all available tools</span>
              </div>
              <div class="text-sm">
                <code class="bg-blue-100 text-blue-800 px-2 py-1 rounded">tools/call</code>
                <span class="ml-2 text-gray-600">Execute a specific tool</span>
              </div>
              <div class="text-sm">
                <code class="bg-blue-100 text-blue-800 px-2 py-1 rounded">capabilities</code>
                <span class="ml-2 text-gray-600">Get server capabilities</span>
              </div>
              <div class="text-sm">
                <code class="bg-blue-100 text-blue-800 px-2 py-1 rounded">resources/list</code>
                <span class="ml-2 text-gray-600">List available resources</span>
              </div>
            </div>
          </div>
          
          <div>
            <h4 class="text-lg font-medium text-gray-700 mb-3">Response Formats</h4>
            <div class="space-y-2">
              <div class="text-sm">
                <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800 mr-2">✅ SUCCESS</span>
                <span class="text-gray-600">Command executed successfully</span>
              </div>
              <div class="text-sm">
                <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800 mr-2">❌ ERROR</span>
                <span class="text-gray-600">Command failed or invalid</span>
              </div>
            </div>
          </div>
        </div>
        
        <div class="mt-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
          <h5 class="text-sm font-medium text-blue-800 mb-2">💡 Usage Tips</h5>
          <ul class="text-sm text-blue-700 space-y-1">
            <li>• Use the quick templates above to get started with common commands</li>
            <li>• All parameters must be valid JSON format</li>
            <li>• Leave parameters empty for methods that don't require them</li>
            <li>• Check the response data for detailed results and error information</li>
            <li>• Use tools/list first to see what tools are available for tools/call</li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</div>