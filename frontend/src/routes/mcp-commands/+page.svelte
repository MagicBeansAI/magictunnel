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

  // Comprehensive MCP 2025-06-18 method templates
  const methodTemplates = [
    // === CORE PROTOCOL ===
    {
      name: 'Initialize',
      method: 'initialize',
      params: '{\n  "protocolVersion": "2025-06-18",\n  "capabilities": {\n    "tools": {},\n    "resources": {},\n    "prompts": {},\n    "sampling": {},\n    "elicitation": {}\n  },\n  "clientInfo": {\n    "name": "MagicTunnel Dashboard",\n    "version": "0.3.14"\n  }\n}',
      description: 'Initialize MCP connection with latest protocol',
      category: 'Core'
    },
    {
      name: 'Get Capabilities',
      method: 'capabilities',
      params: '',
      description: 'Get server capabilities and protocol version',
      category: 'Core'
    },
    {
      name: 'Ping',
      method: 'ping',
      params: '',
      description: 'Test server connectivity and response time',
      category: 'Core'
    },

    // === TOOLS ===
    {
      name: 'List Tools',
      method: 'tools/list',
      params: '',
      description: 'Get all available tools',
      category: 'Tools'
    },
    {
      name: 'Call Tool',
      method: 'tools/call',
      params: '{\n  "name": "example_tool",\n  "arguments": {\n    "param": "value"\n  }\n}',
      description: 'Execute a specific tool',
      category: 'Tools'
    },
    {
      name: 'Smart Tool Discovery',
      method: 'smart_tool_discovery',
      params: '{\n  "request": "ping google.com",\n  "max_tools": 3,\n  "confidence_threshold": 0.7\n}',
      description: 'Find tools using natural language (MagicTunnel specific)',
      category: 'Tools'
    },

    // === RESOURCES ===
    {
      name: 'List Resources',
      method: 'resources/list',
      params: '',
      description: 'Get all available resources',
      category: 'Resources'
    },
    {
      name: 'Read Resource',
      method: 'resources/read',
      params: '{\n  "uri": "file:///path/to/resource"\n}',
      description: 'Read content from a specific resource',
      category: 'Resources'
    },
    {
      name: 'Subscribe to Resource',
      method: 'resources/subscribe',
      params: '{\n  "uri": "file:///path/to/resource"\n}',
      description: 'Subscribe to resource change notifications',
      category: 'Resources'
    },
    {
      name: 'Unsubscribe from Resource',
      method: 'resources/unsubscribe',
      params: '{\n  "uri": "file:///path/to/resource"\n}',
      description: 'Unsubscribe from resource notifications',
      category: 'Resources'
    },

    // === PROMPTS ===
    {
      name: 'List Prompts',
      method: 'prompts/list',
      params: '',
      description: 'Get all available prompt templates',
      category: 'Prompts'
    },
    {
      name: 'Get Prompt',
      method: 'prompts/get',
      params: '{\n  "name": "example_prompt",\n  "arguments": {\n    "variable": "value"\n  }\n}',
      description: 'Get a specific prompt template with variables',
      category: 'Prompts'
    },

    // === SAMPLING (MCP 2025-06-18) ===
    {
      name: 'Create Sample Message',
      method: 'sampling/createMessage',
      params: '{\n  "messages": [\n    {\n      "role": "user",\n      "content": {\n        "type": "text",\n        "text": "Help me analyze this data"\n      }\n    }\n  ],\n  "systemPrompt": "You are a helpful data analyst",\n  "maxTokens": 1000,\n  "temperature": 0.7\n}',
      description: 'Request LLM message generation',
      category: 'Sampling'
    },

    // === ELICITATION (MCP 2025-06-18) ===
    {
      name: 'Request User Input',
      method: 'elicitation/request',
      params: '{\n  "message": "What file would you like to analyze?",\n  "requestedSchema": {\n    "type": "object",\n    "properties": {\n      "filename": {\n        "type": "string",\n        "description": "Path to the file to analyze"\n      }\n    },\n    "required": ["filename"]\n  }\n}',
      description: 'Request structured user input',
      category: 'Elicitation'
    },

    // === ROOTS (MCP 2025-06-18) ===
    {
      name: 'List Roots',
      method: 'roots/list',
      params: '',
      description: 'Get filesystem and URI access boundaries',
      category: 'Roots'
    },

    // === NOTIFICATIONS ===
    {
      name: 'Progress Notification',
      method: 'notifications/progress',
      params: '{\n  "progressToken": "task-123",\n  "progress": 50,\n  "total": 100\n}',
      description: 'Send progress update notification',
      category: 'Notifications'
    },
    {
      name: 'Resource Updated',
      method: 'notifications/resources/updated',
      params: '{\n  "uri": "file:///path/to/resource"\n}',
      description: 'Notify about resource changes',
      category: 'Notifications'
    },
    {
      name: 'Tool List Changed',
      method: 'notifications/tools/list_changed',
      params: '',
      description: 'Notify about tool list changes',
      category: 'Notifications'
    },

    // === MAGICTUNNEL SPECIFIC ===
    {
      name: 'System Health',
      method: 'system/health',
      params: '',
      description: 'Get system health and metrics',
      category: 'System'
    },
    {
      name: 'List External Servers',
      method: 'external_mcp/list',
      params: '',
      description: 'List connected external MCP servers',
      category: 'System'
    },
    {
      name: 'Registry Status',
      method: 'registry/status',
      params: '',
      description: 'Get registry service status',
      category: 'System'
    },
    {
      name: 'Enhancement Status',
      method: 'enhancement/status',
      params: '',
      description: 'Get tool enhancement service status',
      category: 'System'
    }
  ];

  function useTemplate(template: typeof methodTemplates[0]) {
    mcpMethod = template.method;
    mcpParams = template.params;
    mcpResult = null;
  }

  // Group templates by category
  $: groupedTemplates = methodTemplates.reduce((groups, template) => {
    const category = template.category || 'Other';
    if (!groups[category]) {
      groups[category] = [];
    }
    groups[category].push(template);
    return groups;
  }, {} as Record<string, typeof methodTemplates>);

  // Category display order and icons
  const categoryInfo = {
    'Core': { icon: '⚙️', description: 'Essential protocol operations' },
    'Tools': { icon: '🔧', description: 'Tool discovery and execution' },
    'Resources': { icon: '📁', description: 'Resource access and management' },
    'Prompts': { icon: '💬', description: 'Prompt templates and variables' },
    'Sampling': { icon: '🧠', description: 'LLM message generation (MCP 2025)' },
    'Elicitation': { icon: '❓', description: 'Structured user input requests (MCP 2025)' },
    'Notifications': { icon: '🔔', description: 'Event notifications and progress' },
    'System': { icon: '📊', description: 'MagicTunnel system commands' }
  };
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
      <h3 class="text-xl font-semibold text-gray-700 mb-4">🚀 MCP Command Templates</h3>
      <p class="text-gray-600 mb-6">Complete MCP 2025-06-18 protocol command templates organized by category. Click any template to load it.</p>
      
      <div class="space-y-8">
        {#each Object.keys(categoryInfo) as category}
          {#if groupedTemplates[category]}
            <div class="space-y-4">
              <!-- Category Header -->
              <div class="flex items-center gap-3 border-b border-gray-200 pb-2">
                <span class="text-2xl">{categoryInfo[category].icon}</span>
                <div>
                  <h4 class="text-lg font-semibold text-gray-800">{category}</h4>
                  <p class="text-sm text-gray-600">{categoryInfo[category].description}</p>
                </div>
                <div class="ml-auto text-sm text-gray-500">
                  {groupedTemplates[category].length} command{groupedTemplates[category].length !== 1 ? 's' : ''}
                </div>
              </div>
              
              <!-- Category Templates -->
              <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                {#each groupedTemplates[category] as template}
                  <button
                    class="text-left p-4 bg-white border border-gray-200 hover:border-blue-300 hover:shadow-md rounded-lg transition-all duration-200 group"
                    on:click={() => useTemplate(template)}
                    disabled={executingMcpCommand}
                  >
                    <div class="font-medium text-gray-800 mb-1 group-hover:text-blue-700">{template.name}</div>
                    <div class="text-sm font-mono text-blue-600 mb-2 bg-blue-50 px-2 py-1 rounded">{template.method}</div>
                    <div class="text-xs text-gray-500 line-clamp-2">{template.description}</div>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
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
      <h3 class="text-xl font-semibold text-gray-700 mb-4">📚 MCP 2025-06-18 Protocol Reference</h3>
      <div class="space-y-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <h4 class="text-lg font-medium text-gray-700 mb-3">Protocol Highlights</h4>
            <div class="space-y-2">
              <div class="text-sm">
                <code class="bg-purple-100 text-purple-800 px-2 py-1 rounded">sampling/createMessage</code>
                <span class="ml-2 text-gray-600">New LLM message generation</span>
              </div>
              <div class="text-sm">
                <code class="bg-purple-100 text-purple-800 px-2 py-1 rounded">elicitation/request</code>
                <span class="ml-2 text-gray-600">New structured user input</span>
              </div>
              <div class="text-sm">
                <code class="bg-purple-100 text-purple-800 px-2 py-1 rounded">roots/list</code>
                <span class="ml-2 text-gray-600">New filesystem boundaries</span>
              </div>
              <div class="text-sm">
                <code class="bg-green-100 text-green-800 px-2 py-1 rounded">smart_tool_discovery</code>
                <span class="ml-2 text-gray-600">MagicTunnel AI discovery</span>
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
              <div class="text-sm">
                <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800 mr-2">⚠️ PARTIAL</span>
                <span class="text-gray-600">Partial success with warnings</span>
              </div>
            </div>
          </div>
        </div>

        <!-- Protocol Categories Overview -->
        <div class="bg-gradient-to-r from-blue-50 to-purple-50 rounded-lg p-6 border border-blue-200">
          <h4 class="text-lg font-medium text-gray-800 mb-4">🎯 Command Categories Overview</h4>
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div class="text-center">
              <div class="text-2xl mb-2">⚙️</div>
              <div class="font-medium text-gray-800">Core Protocol</div>
              <div class="text-xs text-gray-600">Initialize, capabilities, ping</div>
            </div>
            <div class="text-center">
              <div class="text-2xl mb-2">🔧</div>
              <div class="font-medium text-gray-800">Tools</div>
              <div class="text-xs text-gray-600">Discovery, execution, smart AI</div>
            </div>
            <div class="text-center">
              <div class="text-2xl mb-2">🧠</div>
              <div class="font-medium text-gray-800">LLM Services</div>
              <div class="text-xs text-gray-600">Sampling, elicitation (2025)</div>
            </div>
            <div class="text-center">
              <div class="text-2xl mb-2">📊</div>
              <div class="font-medium text-gray-800">System</div>
              <div class="text-xs text-gray-600">Health, metrics, management</div>
            </div>
          </div>
        </div>
        
        <div class="p-4 bg-blue-50 rounded-lg border border-blue-200">
          <h5 class="text-sm font-medium text-blue-800 mb-2">💡 Usage Tips</h5>
          <ul class="text-sm text-blue-700 space-y-1">
            <li>• <strong>Start with Core:</strong> Use initialize and capabilities to establish connection</li>
            <li>• <strong>Explore Tools:</strong> Try tools/list then tools/call with specific tools</li>
            <li>• <strong>Test Smart Discovery:</strong> Use natural language with smart_tool_discovery</li>
            <li>• <strong>Try MCP 2025 Features:</strong> Sampling and elicitation for advanced workflows</li>
            <li>• <strong>System Commands:</strong> Monitor health and performance with system/* commands</li>
            <li>• <strong>JSON Format:</strong> All parameters must be valid JSON (or leave empty)</li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</div>