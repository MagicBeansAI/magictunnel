<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type MakefileCommandsResponse, type MakefileCommand, type MakefileExecuteResponse } from '$lib/api';

  // Makefile commands state
  let makefileCommands: MakefileCommandsResponse | null = null;
  let makefileLoading = true;
  let executingCommand: string | null = null;
  let lastCommandResult: MakefileExecuteResponse | null = null;
  let expandedCommands: Set<string> = new Set();

  // Makefile commands functions
  async function loadMakefileCommands() {
    makefileLoading = true;
    try {
      makefileCommands = await api.getMakefileCommands();
    } catch (err) {
      console.error('Failed to load Makefile commands:', err);
      makefileCommands = null;
    }
    makefileLoading = false;
  }

  async function executeMakefileCommand(command: string) {
    if (executingCommand) return; // Prevent multiple executions
    
    executingCommand = command;
    try {
      const result = await api.executeMakefileCommand(command);
      lastCommandResult = result;
    } catch (err) {
      console.error('Makefile command execution failed:', err);
      lastCommandResult = {
        action: 'execute_makefile',
        command,
        status: 'error',
        message: `Failed to execute command: ${err}`,
        timestamp: new Date().toISOString()
      };
    }
    executingCommand = null;
  }

  function toggleCommandScript(commandName: string) {
    const newExpanded = new Set(expandedCommands);
    if (newExpanded.has(commandName)) {
      newExpanded.delete(commandName);
    } else {
      newExpanded.add(commandName);
    }
    expandedCommands = newExpanded;
  }

  // Custom restart workflow functions
  function getCategoryIcon(category: string): string {
    switch (category) {
      case 'build': return '🔨';
      case 'test': return '🧪';
      case 'quality': return '✨';
      case 'maintenance': return '🔧';
      case 'docs': return '📚';
      default: return '⚙️';
    }
  }

  function getCommandStatusColor(status: string): string {
    switch (status) {
      case 'success': return 'bg-green-100 text-green-800';
      case 'error': return 'bg-red-100 text-red-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  }

  onMount(() => {
    loadMakefileCommands();
  });
</script>

<svelte:head>
  <title>Build Commands - MagicTunnel</title>
  <meta name="description" content="Build, test, and management operations for MagicTunnel development workflow" />
</svelte:head>

<div class="min-h-screen bg-gray-50">
  <div class="container mx-auto px-4 py-8">
    <!-- Header -->
    <header class="mb-8">
      <div class="mb-2">
        <h1 class="text-4xl font-bold text-primary-700">🔨 Build Commands</h1>
      </div>
      <p class="text-gray-600">Build, test, and management operations for MagicTunnel development workflow</p>
    </header>

    <!-- Build & Management Commands -->
    <div class="card mb-8">
      <div class="flex items-center justify-between mb-6">
        <h3 class="text-xl font-semibold text-gray-700">Available Commands</h3>
        <div class="flex items-center gap-4">
          <div class="text-sm text-gray-500">
            {makefileCommands?.total || 0} commands available
          </div>
          <button 
            class="px-3 py-1 text-sm bg-blue-500 hover:bg-blue-600 text-white rounded-md transition-colors"
            on:click={loadMakefileCommands}
            disabled={makefileLoading}
          >
            {makefileLoading ? '🔄' : '↻'} Refresh
          </button>
        </div>
      </div>

      {#if makefileLoading}
        <div class="text-center py-8">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500 mx-auto mb-4"></div>
          <div class="text-gray-500">🔄 Loading commands...</div>
        </div>
      {:else if !makefileCommands?.commands?.length}
        <div class="text-center py-8">
          <div class="text-gray-500 mb-2">📋 No Makefile commands found</div>
          <div class="text-sm text-gray-400">
            Makefile commands enable build, test, and management operations
          </div>
        </div>
      {:else}
        <!-- Command Execution Result -->
        {#if lastCommandResult}
          <div class="mb-6 p-4 border border-gray-200 rounded-lg bg-gray-50">
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-sm font-medium text-gray-700">Last Command Result</h4>
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {getCommandStatusColor(lastCommandResult.status)}">
                {lastCommandResult.status}
              </span>
            </div>
            <div class="text-lg font-mono font-semibold text-gray-900 mb-2">
              {lastCommandResult.command}
            </div>
            <div class="text-sm text-gray-600 mb-2">{lastCommandResult.message}</div>
            {#if lastCommandResult.output}
              <details class="text-xs">
                <summary class="cursor-pointer text-gray-500 hover:text-gray-700">View Output</summary>
                <pre class="mt-2 p-2 bg-white rounded border text-gray-800 whitespace-pre-wrap max-h-40 overflow-y-auto">{lastCommandResult.output}</pre>
              </details>
            {/if}
            {#if lastCommandResult.exit_code !== undefined}
              <div class="text-xs text-gray-500 mt-2">
                Exit code: {lastCommandResult.exit_code}
              </div>
            {/if}
          </div>
        {/if}

        <!-- Commands by Category -->
        {#if makefileCommands.categories}
          <div class="space-y-6">
            {#each Object.entries(makefileCommands.categories) as [category, commands]}
              <div>
                <h4 class="text-lg font-medium text-gray-700 mb-3 flex items-center">
                  <span class="mr-2">{getCategoryIcon(category)}</span>
                  {category.charAt(0).toUpperCase() + category.slice(1)} Commands
                  <span class="ml-2 text-sm text-gray-500">({commands.length})</span>
                </h4>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {#each commands as command}
                    {#if makefileCommands.commands}
                      {@const fullCommand = makefileCommands.commands.find(c => c.name === command)}
                      {#if fullCommand}
                        <div class="border border-gray-200 rounded-lg p-4 hover:bg-gray-50 transition-colors">
                          <!-- Prominent Command Name -->
                          <div class="text-xl font-bold text-gray-900 mb-2">
                            {fullCommand.name}
                          </div>
                          
                          <!-- Command Details -->
                          <div class="text-sm font-mono text-gray-600 mb-3 bg-gray-100 px-2 py-1 rounded">
                            make {fullCommand.name}
                          </div>
                          
                          <!-- Description -->
                          <p class="text-sm text-gray-600 mb-3 min-h-[2.5rem]">
                            {fullCommand.description}
                          </p>
                          
                          <!-- Show Commands Section -->
                          {#if fullCommand.script}
                            <div class="mb-3">
                              <button
                                class="text-xs text-blue-600 hover:text-blue-800 underline"
                                on:click={() => toggleCommandScript(fullCommand.name)}
                              >
                                {expandedCommands.has(fullCommand.name) ? '🔼 Hide Commands' : '🔽 Show Commands'}
                              </button>
                              
                              {#if expandedCommands.has(fullCommand.name)}
                                <div class="mt-2 p-3 bg-gray-50 rounded-md border-l-4 border-blue-200">
                                  <div class="text-xs text-gray-500 mb-1">Command Script:</div>
                                  <pre class="text-xs font-mono text-gray-800 whitespace-pre-wrap overflow-x-auto">{fullCommand.script}</pre>
                                </div>
                              {/if}
                            </div>
                          {/if}
                          
                          <!-- Status indicators and action -->
                          <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                              {#if !fullCommand.safe_for_production}
                                <span class="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-800" title="Requires confirmation">
                                  ⚠️
                                </span>
                              {/if}
                              {#if fullCommand.requires_env?.length}
                                <span class="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800" title="Requires: {fullCommand.requires_env.join(', ')}">
                                  🔧
                                </span>
                              {/if}
                            </div>
                            
                            <button
                              class="btn-sm btn-primary"
                              disabled={executingCommand === fullCommand.name}
                              on:click={() => executeMakefileCommand(fullCommand.name)}
                            >
                              {executingCommand === fullCommand.name ? '⏳' : '▶️'} Execute
                            </button>
                          </div>
                        </div>
                      {/if}
                    {/if}
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Help & Documentation -->
    <div class="card">
      <h3 class="text-xl font-semibold text-gray-700 mb-4">📚 Command Information</h3>
      <div class="space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <h4 class="text-lg font-medium text-gray-700 mb-3">Command Categories</h4>
            <div class="space-y-2">
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">🔨</span>
                <div>
                  <span class="font-medium">Build Commands</span>
                  <div class="text-sm text-gray-500">Compile, package, and prepare releases</div>
                </div>
              </div>
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">🧪</span>
                <div>
                  <span class="font-medium">Test Commands</span>
                  <div class="text-sm text-gray-500">Run test suites and validation</div>
                </div>
              </div>
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">✨</span>
                <div>
                  <span class="font-medium">Quality Commands</span>
                  <div class="text-sm text-gray-500">Code formatting, linting, and analysis</div>
                </div>
              </div>
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">🔧</span>
                <div>
                  <span class="font-medium">Maintenance Commands</span>
                  <div class="text-sm text-gray-500">Cleanup, updates, and system maintenance</div>
                </div>
              </div>
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">📚</span>
                <div>
                  <span class="font-medium">Documentation Commands</span>
                  <div class="text-sm text-gray-500">Generate and update documentation</div>
                </div>
              </div>
            </div>
          </div>
          
          <div>
            <h4 class="text-lg font-medium text-gray-700 mb-3">Status Indicators</h4>
            <div class="space-y-2">
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">⚠️</span>
                <div>
                  <span class="font-medium">Requires Confirmation</span>
                  <div class="text-sm text-gray-500">Command may affect production or require user input</div>
                </div>
              </div>
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">🔧</span>
                <div>
                  <span class="font-medium">Environment Dependencies</span>
                  <div class="text-sm text-gray-500">Command requires specific environment variables</div>
                </div>
              </div>
              <div class="flex items-center text-gray-600">
                <span class="text-lg mr-3">⏳</span>
                <div>
                  <span class="font-medium">Executing</span>
                  <div class="text-sm text-gray-500">Command is currently running</div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <div class="mt-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
          <h5 class="text-sm font-medium text-blue-800 mb-2">💡 Usage Tips</h5>
          <ul class="text-sm text-blue-700 space-y-1">
            <li>• Click "Show Commands" to view the actual script behind each command</li>
            <li>• Commands with ⚠️ may require confirmation dialogs or affect production systems</li>
            <li>• Commands with 🔧 require specific environment variables to be set</li>
            <li>• Check the "Last Command Result" section for execution details and output</li>
            <li>• Use the refresh button to reload available commands after Makefile changes</li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</div>