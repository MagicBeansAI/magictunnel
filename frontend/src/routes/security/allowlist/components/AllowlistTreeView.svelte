<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { AllowlistRule } from '$lib/types/security';
  
  export let treeData: any;
  
  const dispatch = createEventDispatcher();
  
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
  
  // Get rule status and styling
  function getRuleStatus(node: TreeNode): { status: string; color: string; icon: string; inherited: boolean } {
    if (node.rule) {
      if (!node.rule.enabled) {
        return { status: 'Disabled', color: 'text-gray-500 bg-gray-100', icon: '⏸️', inherited: false };
      }
      switch (node.rule.action) {
        case 'allow':
          return { status: 'Allow', color: 'text-green-700 bg-green-100', icon: '✅', inherited: false };
        case 'deny':
          return { status: 'Deny', color: 'text-red-700 bg-red-100', icon: '🚫', inherited: false };
      }
    }
    
    // Check for inherited rules from parent
    if (node.parent && node.parent.rule) {
      const parentStatus = getRuleStatus(node.parent);
      return { 
        status: `Inherited ${parentStatus.status}`, 
        color: `${parentStatus.color} border-dashed`, 
        icon: parentStatus.icon, 
        inherited: true 
      };
    }
    
    return { status: 'Default', color: 'text-blue-700 bg-blue-100', icon: '⚙️', inherited: false };
  }
  
  // Toggle node expansion
  function toggleExpanded(node: TreeNode) {
    node.expanded = !node.expanded;
    treeData = treeData; // Trigger reactivity
  }
  
  // Handle rule change
  function handleRuleChange(node: TreeNode, action: 'allow' | 'deny' | 'remove') {
    dispatch('rule-change', {
      nodeId: node.id,
      action,
      rule: node.rule
    });
  }
  
  // Get indentation for tree levels
  function getIndentation(level: number): string {
    return `${level * 20}px`;
  }
  
  // Get connector lines for tree structure
  function getConnectorClass(node: TreeNode, isLast: boolean): string {
    if (node.level === 0) return '';
    
    let classes = 'relative before:absolute before:left-0 before:top-0 before:w-4 before:h-6 before:border-l before:border-b before:border-gray-300';
    
    if (!isLast) {
      classes += ' after:absolute after:left-0 after:top-6 after:w-0 after:h-full after:border-l after:border-gray-300';
    }
    
    return classes;
  }
  
  // Render tree node recursively
  function renderNode(node: TreeNode, index: number, siblings: TreeNode[]): any {
    const isLast = index === siblings.length - 1;
    const status = getRuleStatus(node);
    const hasChildren = node.children.length > 0;
    
    return {
      node,
      isLast,
      status,
      hasChildren,
      indentation: getIndentation(node.level),
      connectorClass: getConnectorClass(node, isLast)
    };
  }
</script>

{#if treeData}
  <div class="tree-view space-y-1">
    <!-- Root Node -->
    <div class="flex items-center gap-2 p-2 rounded-lg bg-gray-50 border">
      <div class="flex items-center gap-2">
        <span class="text-lg">🏠</span>
        <span class="font-semibold text-gray-900">{treeData.name}</span>
      </div>
      <div class="ml-auto">
        <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium text-blue-700 bg-blue-100">
          🏛️ System Root
        </span>
      </div>
    </div>
    
    <!-- Server Nodes -->
    {#each treeData.children as serverNode, serverIndex}
      {@const serverRender = renderNode(serverNode, serverIndex, treeData.children)}
      <div class="relative">
        <!-- Server Row -->
        <div class="flex items-center gap-2 p-2 rounded-lg hover:bg-gray-50 border border-transparent hover:border-gray-200 transition-all">
          <!-- Indentation and Tree Lines -->
          <div style="margin-left: {serverRender.indentation}" class="{serverRender.connectorClass}">
            <div class="flex items-center gap-2">
              <!-- Expand/Collapse Button -->
              {#if serverRender.hasChildren}
                <button 
                  class="w-5 h-5 flex items-center justify-center rounded hover:bg-gray-200 transition-colors"
                  on:click={() => toggleExpanded(serverNode)}
                >
                  <span class="text-xs transform transition-transform {serverNode.expanded ? 'rotate-90' : ''}">
                    ▶️
                  </span>
                </button>
              {:else}
                <div class="w-5 h-5"></div>
              {/if}
              
              <!-- Server Icon and Name -->
              <span class="text-lg">{serverNode.name.includes('Operations') ? '🏠' : '🔌'}</span>
              <span class="font-medium text-gray-900">{serverNode.name}</span>
              
              <!-- Server Status -->
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {serverRender.status.color} {serverRender.status.inherited ? 'border border-current' : ''}">
                {serverRender.status.icon} {serverRender.status.status}
                {#if serverRender.status.inherited}<span class="ml-1 opacity-60">(inherited)</span>{/if}
              </span>
            </div>
          </div>
          
          <!-- Server Actions -->
          <div class="ml-auto flex items-center gap-1">
            <button
              class="p-1 text-xs bg-green-100 hover:bg-green-200 text-green-700 rounded transition-colors"
              on:click={() => handleRuleChange(serverNode, 'allow')}
              title="Allow this server"
            >
              ✅
            </button>
            <button
              class="p-1 text-xs bg-red-100 hover:bg-red-200 text-red-700 rounded transition-colors"
              on:click={() => handleRuleChange(serverNode, 'deny')}
              title="Deny this server"
            >
              🚫
            </button>
            {#if serverNode.rule}
              <button
                class="p-1 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 rounded transition-colors"
                on:click={() => handleRuleChange(serverNode, 'remove')}
                title="Remove rule (use default)"
              >
                🗑️
              </button>
            {/if}
          </div>
        </div>
        
        <!-- Tool Nodes (when server is expanded) -->
        {#if serverNode.expanded && serverNode.children.length > 0}
          <div class="ml-4">
            {#each serverNode.children as toolNode, toolIndex}
              {@const toolRender = renderNode(toolNode, toolIndex, serverNode.children)}
              <div class="flex items-center gap-2 p-2 rounded-lg hover:bg-gray-50 border border-transparent hover:border-gray-200 transition-all">
                <!-- Tool Indentation and Tree Lines -->
                <div style="margin-left: {toolRender.indentation}" class="{toolRender.connectorClass}">
                  <div class="flex items-center gap-2">
                    <!-- Tool Icon and Name -->
                    <div class="w-5 h-5"></div> <!-- Spacer for alignment -->
                    <span class="text-md">🔧</span>
                    <span class="text-sm text-gray-700">{toolNode.name}</span>
                    
                    <!-- Tool Status -->
                    <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {toolRender.status.color} {toolRender.status.inherited ? 'border border-current' : ''}">
                      {toolRender.status.icon} {toolRender.status.status}
                      {#if toolRender.status.inherited}<span class="ml-1 opacity-60">(inherited)</span>{/if}
                    </span>
                  </div>
                </div>
                
                <!-- Tool Actions -->
                <div class="ml-auto flex items-center gap-1">
                  <button
                    class="p-1 text-xs bg-green-100 hover:bg-green-200 text-green-700 rounded transition-colors"
                    on:click={() => handleRuleChange(toolNode, 'allow')}
                    title="Allow this tool"
                  >
                    ✅
                  </button>
                  <button
                    class="p-1 text-xs bg-red-100 hover:bg-red-200 text-red-700 rounded transition-colors"
                    on:click={() => handleRuleChange(toolNode, 'deny')}
                    title="Deny this tool"
                  >
                    🚫
                  </button>
                  {#if toolNode.rule}
                    <button
                      class="p-1 text-xs bg-gray-100 hover:bg-gray-200 text-gray-700 rounded transition-colors"
                      on:click={() => handleRuleChange(toolNode, 'remove')}
                      title="Remove rule (use default)"
                    >
                      🗑️
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .tree-view {
    font-family: 'SF Mono', 'Monaco', 'Inconsolata', 'Roboto Mono', monospace;
  }
  
  /* Custom tree line styling */
  .tree-view .relative::before {
    content: '';
  }
  
  .tree-view .relative::after {
    content: '';
  }
</style>