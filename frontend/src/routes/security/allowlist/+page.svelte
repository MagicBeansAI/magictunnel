<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type McpServersResponse, type CapabilitiesResponse, type Capability } from '$lib/api';
  import { securityApi } from '$lib/api/security';
  import type { AllowlistRule } from '$lib/types/security';
  import { getInternalServers } from '$lib/utils/mcpServers';
  import AllowlistTreeView from './components/AllowlistTreeView.svelte';
  import PatternTester from './components/PatternTester.svelte';
  import PatternManager from './components/PatternManager.svelte';

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
  let highlightedNodeIds: Set<string> = new Set();
  let parentHighlightIds: Set<string> = new Set();
  let currentMatches: any[] = [];
  
  // Tab management
  let activeTab: 'tree' | 'patterns' = 'tree';

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
        id: `capability:${server.name}`, // External MCP servers are treated as capabilities in allowlist system
        name: server.name,
        type: 'server',
        rule: null,
        children: [],
        parent: root,
        expanded: false,
        level: 1
      };

      // Load server allowlist rule (external MCP servers are treated as capabilities in allowlist system)
      try {
        serverNode.rule = await api.getCapabilityAllowlistRule(server.name);
        console.log(`🔍 Loaded rule for external server ${server.name}:`, serverNode.rule);
      } catch (err) {
        console.warn(`Failed to load allowlist rule for external server ${server.name}:`, err);
      }

      // Add tools for this external server  
      const tools = server.tools || [];
      console.log(`Server ${server.name} has ${tools.length} tools:`, tools);
      
      if (tools && tools.length > 0) {
        for (const tool of tools) {
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

          // Load tool allowlist rule using qualified name
          const qualifiedToolName = `${server.name}.${tool.name}`;
          try {
            toolNode.rule = await api.getToolAllowlistRule(qualifiedToolName);
          } catch (err) {
            console.warn(`Failed to load allowlist rule for tool ${qualifiedToolName}:`, err);
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
        id: `capability:${internalServer.name}`,
        name: internalServer.name,
        type: 'server',
        rule: null,
        children: [],
        parent: root,
        expanded: false,
        level: 1
      };

      // Load capability allowlist rule (internal servers are actually capability groups)
      try {
        serverNode.rule = await api.getCapabilityAllowlistRule(internalServer.name);
        console.log(`🔍 Loaded rule for capability ${internalServer.name}:`, serverNode.rule);
      } catch (err) {
        console.warn(`Failed to load allowlist rule for capability ${internalServer.name}:`, err);
      }

      // Add tools for this internal capability
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

        // Load tool allowlist rule using qualified name (capability.tool_name)
        const qualifiedToolName = `${internalServer.name}.${capability.name}`;
        try {
          toolNode.rule = await api.getToolAllowlistRule(qualifiedToolName);
        } catch (err) {
          console.warn(`Failed to load allowlist rule for tool ${qualifiedToolName}:`, err);
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
    
    console.log('🔄 Rule change requested:', { nodeId, action, rule });
    
    try {
      if (nodeId.startsWith('server:')) {
        const serverName = nodeId.replace('server:', '');
        console.log('📡 Updating server rule:', serverName, action);
        if (action === 'remove') {
          await api.removeServerAllowlistRule(serverName);
        } else {
          await api.setServerAllowlistRule(serverName, action, `${action} access to ${serverName}`);
        }
      } else if (nodeId.startsWith('capability:')) {
        const capabilityName = nodeId.replace('capability:', '');
        console.log('🏗️ Updating capability rule:', capabilityName, action);
        if (action === 'remove') {
          const result = await api.removeCapabilityAllowlistRule(capabilityName);
          console.log('✅ Capability rule removed:', result);
        } else {
          const payload = { action, reason: `${action} access to ${capabilityName}` };
          console.log('📝 Setting capability rule with payload:', payload);
          const result = await api.setCapabilityAllowlistRule(capabilityName, payload);
          console.log('✅ Capability rule set:', result);
        }
      } else if (nodeId.startsWith('tool:')) {
        const [, serverName, toolName] = nodeId.split(':');
        const qualifiedToolName = `${serverName}.${toolName}`;
        console.log('🔧 Updating tool rule:', qualifiedToolName, action);
        if (action === 'remove') {
          await api.removeToolAllowlistRule(qualifiedToolName);
        } else {
          await api.setToolAllowlistRule(qualifiedToolName, action, `${action} access to ${qualifiedToolName}`);
        }
      }
      
      console.log('🔄 Reloading tree data after rule change...');
      // Reload tree data to reflect changes
      await buildTreeData();
      console.log('✅ Tree data reloaded successfully');
    } catch (err) {
      console.error('❌ Failed to update allowlist rule:', err);
      error = `Failed to update rule: ${err}`;
    }
  }

  // Handle pattern testing with hierarchical highlighting
  async function handlePatternTest(event: CustomEvent) {
    const { pattern, patternType, action, matchingNodes, directMatches, parentMatches } = event.detail;
    
    try {
      // Update pattern and results
      selectedPattern = pattern;
      currentMatches = matchingNodes || [];
      
      // Set up hierarchical highlighting
      highlightedNodeIds = new Set((directMatches || []).map(match => match.id));
      parentHighlightIds = new Set((parentMatches || []).map(match => match.id));
      
      // Test the pattern via API for validation
      patternTestResults = await api.testAllowlistRule({
        pattern,
        pattern_type: patternType,
        action
      });
      
      console.log(`Pattern testing: '${pattern}' - ${directMatches?.length || 0} direct matches, ${parentMatches?.length || 0} parent indicators`);
      console.log('Direct matches:', Array.from(highlightedNodeIds));
      console.log('Parent indicators:', Array.from(parentHighlightIds));
    } catch (err) {
      console.error('Failed to test pattern:', err);
      error = `Failed to test pattern: ${err}`;
    }
  }
  
  // Handle bulk apply
  async function handleBulkApply(event: CustomEvent) {
    const { pattern, action, matches, total } = event.detail;
    
    if (!matches || matches.length === 0) {
      error = 'No matches to apply';
      return;
    }
    
    try {
      console.log(`Bulk applying ${action} to ${total} items:`, matches);
      
      // Apply rules to each matched item
      for (const match of matches) {
        if (match.id.startsWith('server:')) {
          const serverName = match.id.replace('server:', '');
          await api.setServerAllowlistRule(serverName, action, `Bulk ${action} via pattern: ${pattern}`);
        } else if (match.id.startsWith('capability:')) {
          const capabilityName = match.id.replace('capability:', '');
          await api.setCapabilityAllowlistRule(capabilityName, { action, reason: `Bulk ${action} via pattern: ${pattern}` });
        } else if (match.id.startsWith('tool:')) {
          const [, serverName, toolName] = match.id.split(':');
          const qualifiedToolName = `${serverName}.${toolName}`;
          await api.setToolAllowlistRule(qualifiedToolName, action, `Bulk ${action} via pattern: ${pattern}`);
        }
      }
      
      // Clear highlights and rebuild tree
      highlightedNodeIds = new Set();
      parentHighlightIds = new Set();
      currentMatches = [];
      await buildTreeData();
      
      console.log(`Successfully applied ${action} to ${total} items`);
    } catch (err) {
      console.error('Failed to bulk apply rules:', err);
      error = `Failed to bulk apply: ${err}`;
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
    <!-- Tab Navigation -->
    <div class="bg-white rounded-lg shadow border mb-6">
      <div class="border-b border-gray-200">
        <nav class="flex space-x-8 px-6" aria-label="Tabs">
          <button
            on:click={() => activeTab = 'tree'}
            class="py-4 px-1 border-b-2 font-medium text-sm {activeTab === 'tree' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          >
            🌳 Tree View
          </button>
          <button
            on:click={() => activeTab = 'patterns'}
            class="py-4 px-1 border-b-2 font-medium text-sm {activeTab === 'patterns' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          >
            🎯 Pattern Management
          </button>
        </nav>
      </div>
    </div>
    
    {#if activeTab === 'tree'}
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
                {highlightedNodeIds}
                {parentHighlightIds}
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
                on:bulk-apply={handleBulkApply}
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
                    <div class="font-medium text-gray-900">🚨 Emergency Lockdown</div>
                    <div class="text-gray-600">Highest priority - overrides all</div>
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <div class="w-4 h-4 bg-blue-500 rounded-full flex items-center justify-center">
                    <span class="text-white text-xs">2</span>
                  </div>
                  <div>
                    <div class="font-medium text-gray-900">📌 Explicit Rules</div>
                    <div class="text-gray-600">Specific tool/capability allow/deny</div>
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <div class="w-4 h-4 bg-green-500 rounded-full flex items-center justify-center">
                    <span class="text-white text-xs">3</span>
                  </div>
                  <div>
                    <div class="font-medium text-gray-900">🎯 Tool Patterns</div>
                    <div class="text-gray-600">Tool-specific pattern matching</div>
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <div class="w-4 h-4 bg-purple-500 rounded-full flex items-center justify-center">
                    <span class="text-white text-xs">4</span>
                  </div>
                  <div>
                    <div class="font-medium text-gray-900">🎯 Capability Patterns</div>
                    <div class="text-gray-600">Capability-level pattern matching</div>
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <div class="w-4 h-4 bg-indigo-500 rounded-full flex items-center justify-center">
                    <span class="text-white text-xs">5</span>
                  </div>
                  <div>
                    <div class="font-medium text-gray-900">🌍 Global Patterns</div>
                    <div class="text-gray-600">System-wide pattern matching</div>
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <div class="w-4 h-4 bg-gray-400 rounded-full flex items-center justify-center">
                    <span class="text-white text-xs">6</span>
                  </div>
                  <div>
                    <div class="font-medium text-gray-900">⚙️ Default Policy</div>
                    <div class="text-gray-600">System fallback</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    {:else if activeTab === 'patterns'}
      <!-- Pattern Management Tab -->
      <PatternManager visible={activeTab === 'patterns'} />
    {/if}
  {/if}
</div>