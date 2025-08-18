<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type McpServersResponse, type CapabilitiesResponse, type Capability } from '$lib/api';
  import { securityApi } from '$lib/api/security';
  import type { AllowlistRule } from '$lib/types/security';
  import { getInternalServers } from '$lib/utils/mcpServers';
  import AllowlistTreeView from './components/AllowlistTreeView.svelte';
  import PatternTester from './components/PatternTester.svelte';

  // Data
  let serversResponse: McpServersResponse | null = null;
  let capabilitiesResponse: CapabilitiesResponse | null = null;
  let loading = true;
  let error = '';
  let allowlistEnabled = false;
  let configLoading = true;

  // Tree data structure
  interface TreeNode {
    id: string;
    name: string;
    type: 'root' | 'server' | 'tool';
    rule: AllowlistRule | null;
    children: TreeNode[];
    parent?: TreeNode;
    expanded: boolean;
    level: number;
  }

  let treeData: TreeNode | null = null;
  let selectedPattern = '';
  let patternTestResults: any = null;

  // Check if Allowlist is enabled
  async function checkAllowlistConfig() {
    try {
      configLoading = true;
      const config = await securityApi.getSecurityConfig();
      allowlistEnabled = config.allowlist?.enabled || false;
    } catch (err) {
      console.error('Failed to load security config:', err);
      allowlistEnabled = false;
    } finally {
      configLoading = false;
    }
  }

  // Load all data
  async function loadData() {
    try {
      loading = true;
      error = '';
      
      // Check if Allowlist is enabled first
      if (!allowlistEnabled) {
        loading = false;
        return;
      }
      
      const [servers, capabilities] = await Promise.all([
        api.getMcpServers(),
        api.getCapabilities()
      ]);
      
      serversResponse = servers;
      capabilitiesResponse = capabilities;
      
      await buildTreeData();
      console.log('Allowlist tree data loaded:', treeData);
    } catch (err) {
      console.error('Failed to load allowlist data:', err);
      error = err instanceof Error ? err.message : 'Failed to load allowlist data';
    } finally {
      loading = false;
    }
  }

  async function buildTreeData() {
    if (!serversResponse || !capabilitiesResponse) return;

    // Create root node
    const root: TreeNode = {
      id: 'root',
      name: 'MagicTunnel Allowlist',
      type: 'root',
      rule: null,
      children: [],
      expanded: true,
      level: 0
    };

    // Add external MCP servers
    console.log('Building tree from servers:', serversResponse.servers);
    for (const server of serversResponse.servers) {
      console.log('Processing server:', server.name, 'capabilities:', server.capabilities);
      const serverNode: TreeNode = {
        id: `server:${server.name}`,
        name: server.name,
        type: 'server',
        rule: null,
        children: [],
        parent: root,
        expanded: false,
        level: 1
      };

      // Load server allowlist rule
      try {
        serverNode.rule = await api.getServerAllowlistRule(server.name);
      } catch (err) {
        console.warn(`Failed to load allowlist rule for server ${server.name}:`, err);
      }

      // Add tools for this external server  
      const capabilities = server.capabilities || [];
      console.log(`Server ${server.name} has ${capabilities.length} capabilities:`, capabilities);
      
      if (capabilities && capabilities.length > 0) {
        for (const tool of capabilities) {
          const toolNode: TreeNode = {
            id: `tool:${server.name}:${tool.name}`,
            name: tool.name,
            type: 'tool',
            rule: null,
            children: [],
            parent: serverNode,
            expanded: false,
            level: 2
          };

          // Load tool allowlist rule
          try {
            toolNode.rule = await api.getToolAllowlistRule(tool.name);
          } catch (err) {
            console.warn(`Failed to load allowlist rule for tool ${tool.name}:`, err);
          }

          serverNode.children.push(toolNode);
        }
      }

      root.children.push(serverNode);
    }

    // Add internal servers (capability groups)
    const internalServers = getInternalServers(capabilitiesResponse);
    for (const internalServer of internalServers) {
      const serverNode: TreeNode = {
        id: `server:${internalServer.name}`,
        name: internalServer.name,
        type: 'server',
        rule: null,
        children: [],
        parent: root,
        expanded: false,
        level: 1
      };

      // Load server allowlist rule
      try {
        serverNode.rule = await api.getServerAllowlistRule(internalServer.name);
      } catch (err) {
        console.warn(`Failed to load allowlist rule for server ${internalServer.name}:`, err);
      }

      // Add tools for this internal server
      for (const capability of internalServer.capabilities) {
        const toolNode: TreeNode = {
          id: `tool:${internalServer.name}:${capability.name}`,
          name: capability.name,
          type: 'tool',
          rule: null,
          children: [],
          parent: serverNode,
          expanded: false,
          level: 2
        };

        // Load tool allowlist rule
        try {
          toolNode.rule = await api.getToolAllowlistRule(capability.name);
        } catch (err) {
          console.warn(`Failed to load allowlist rule for tool ${capability.name}:`, err);
        }

        serverNode.children.push(toolNode);
      }

      root.children.push(serverNode);
    }

    treeData = root;
  }


  // Handle rule changes from tree view
  async function handleRuleChange(event: CustomEvent) {
    const { nodeId, action, rule } = event.detail;
    
    try {
      if (nodeId.startsWith('server:')) {
        const serverName = nodeId.replace('server:', '');
        if (action === 'remove') {
          await api.removeServerAllowlistRule(serverName);
        } else {
          await api.setServerAllowlistRule(serverName, action, `${action} access to ${serverName}`);
        }
      } else if (nodeId.startsWith('tool:')) {
        const [, serverName, toolName] = nodeId.split(':');
        if (action === 'remove') {
          await api.removeToolAllowlistRule(toolName);
        } else {
          await api.setToolAllowlistRule(toolName, action, `${action} access to ${toolName}`);
        }
      }
      
      // Reload tree data to reflect changes
      await buildTreeData();
    } catch (err) {
      console.error('Failed to update allowlist rule:', err);
      error = `Failed to update rule: ${err}`;
    }
  }

  // Handle pattern testing
  async function handlePatternTest() {
    if (!selectedPattern.trim()) {
      patternTestResults = null;
      return;
    }

    try {
      patternTestResults = await api.testAllowlistRule({
        pattern: selectedPattern,
        pattern_type: 'regex',
        action: 'allow'
      });
    } catch (err) {
      console.error('Failed to test pattern:', err);
      error = `Failed to test pattern: ${err}`;
    }
  }

  onMount(async () => {
    await checkAllowlistConfig();
    if (allowlistEnabled) {
      loadData();
    }
  });
</script>

<div class="container mx-auto px-6 py-8">
  <div class="flex justify-between items-center mb-8">
    <div>
      <h1 class="text-3xl font-bold text-gray-900">🛡️ Allowlist Management</h1>
      <p class="text-gray-600 mt-2">Control access to MCP servers and tools with hierarchical rules</p>
    </div>
    <button
      on:click={loadData}
      disabled={loading}
      class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
    >
      <span class="text-sm">🔄</span>
      {loading ? 'Refreshing...' : 'Refresh'}
    </button>
  </div>

  {#if configLoading}
    <div class="flex justify-center items-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      <span class="ml-3 text-gray-600">Loading configuration...</span>
    </div>
  {:else if !allowlistEnabled}
    <!-- Disabled State -->
    <div class="security-card">
      <div class="text-center py-12">
        <div class="text-gray-400 mb-4">
          <svg class="h-12 w-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728L5.636 5.636m12.728 12.728L5.636 5.636" />
          </svg>
        </div>
        <h3 class="text-lg font-medium text-gray-900 mb-2">Allowlist Service Disabled</h3>
        <p class="text-gray-600 mb-4">
          Tool and Server Allowlisting is currently disabled in the security configuration.
        </p>
        <div class="text-sm text-gray-500 mb-4">
          To enable Allowlisting, update the security configuration:
        </div>
        <div class="bg-gray-50 rounded-lg p-4 text-left max-w-md mx-auto">
          <code class="text-sm text-gray-800">
            security:<br/>
            &nbsp;&nbsp;allowlist:<br/>
            &nbsp;&nbsp;&nbsp;&nbsp;enabled: true
          </code>
        </div>
        <div class="mt-4">
          <button class="btn-secondary" on:click={() => window.location.href = '/security/config'}>
            📝 Security Configuration
          </button>
        </div>
      </div>
    </div>
  {:else if error}
    <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6">
      <strong class="font-bold">Error:</strong>
      <span class="block sm:inline">{error}</span>
    </div>
  {:else if loading && !treeData}
    <div class="flex justify-center items-center py-12">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      <span class="ml-3 text-gray-600">Loading allowlist data...</span>
    </div>
  {:else if treeData}
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
      <!-- Tree View (Main Panel) -->
      <div class="lg:col-span-2">
        <div class="bg-white rounded-lg shadow border">
          <div class="p-6 border-b border-gray-200">
            <h2 class="text-xl font-semibold text-gray-900">Hierarchical Allowlist Tree</h2>
            <p class="text-sm text-gray-600 mt-1">
              Manage allow/deny rules at server and tool level. Rules inherit from parent levels.
            </p>
          </div>
          <div class="p-6">
            <AllowlistTreeView 
              {treeData} 
              on:rule-change={handleRuleChange}
            />
          </div>
        </div>
      </div>

      <!-- Pattern Tester (Side Panel) -->
      <div class="lg:col-span-1">
        <div class="bg-white rounded-lg shadow border">
          <div class="p-6 border-b border-gray-200">
            <h2 class="text-xl font-semibold text-gray-900">Pattern Tester</h2>
            <p class="text-sm text-gray-600 mt-1">
              Test patterns to see which nodes they match.
            </p>
          </div>
          <div class="p-6">
            <PatternTester 
              bind:pattern={selectedPattern}
              {patternTestResults}
              {treeData}
              on:test-pattern={handlePatternTest}
            />
          </div>
        </div>

        <!-- Rule Hierarchy Info -->
        <div class="bg-white rounded-lg shadow border mt-6">
          <div class="p-6 border-b border-gray-200">
            <h2 class="text-xl font-semibold text-gray-900">Rule Hierarchy</h2>
          </div>
          <div class="p-6">
            <div class="space-y-3 text-sm">
              <div class="flex items-center gap-3">
                <div class="w-4 h-4 bg-red-500 rounded-full flex items-center justify-center">
                  <span class="text-white text-xs">1</span>
                </div>
                <div>
                  <div class="font-medium text-gray-900">Emergency Lockdown</div>
                  <div class="text-gray-600">Highest priority - overrides all</div>
                </div>
              </div>
              <div class="flex items-center gap-3">
                <div class="w-4 h-4 bg-blue-500 rounded-full flex items-center justify-center">
                  <span class="text-white text-xs">2</span>
                </div>
                <div>
                  <div class="font-medium text-gray-900">Tool Rules</div>
                  <div class="text-gray-600">Specific tool allow/deny</div>
                </div>
              </div>
              <div class="flex items-center gap-3">
                <div class="w-4 h-4 bg-green-500 rounded-full flex items-center justify-center">
                  <span class="text-white text-xs">3</span>
                </div>
                <div>
                  <div class="font-medium text-gray-900">Server Rules</div>
                  <div class="text-gray-600">Server-level allow/deny</div>
                </div>
              </div>
              <div class="flex items-center gap-3">
                <div class="w-4 h-4 bg-purple-500 rounded-full flex items-center justify-center">
                  <span class="text-white text-xs">4</span>
                </div>
                <div>
                  <div class="font-medium text-gray-900">Pattern Rules</div>
                  <div class="text-gray-600">Regex pattern matching</div>
                </div>
              </div>
              <div class="flex items-center gap-3">
                <div class="w-4 h-4 bg-gray-400 rounded-full flex items-center justify-center">
                  <span class="text-white text-xs">5</span>
                </div>
                <div>
                  <div class="font-medium text-gray-900">Default Policy</div>
                  <div class="text-gray-600">System fallback</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>