<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  
  export let pattern: string = '';
  export let patternTestResults: any = null;
  export let treeData: any = null;
  
  const dispatch = createEventDispatcher();
  
  let patternType: 'regex' | 'glob' | 'exact' = 'regex';
  let testAction: 'allow' | 'deny' = 'allow';
  let testing = false;
  
  // Batch testing mode
  let testMode: 'single' | 'batch' = 'single';
  let batchPatterns = '';
  let batchTestInputs = '';
  let batchResults: any[] = [];
  
  // Handle pattern test
  async function handleTest() {
    if (testMode === 'single') {
      if (!pattern.trim()) return;
      testing = true;
      dispatch('test-pattern');
      testing = false;
    } else {
      await handleBatchTest();
    }
  }
  
  // Handle batch pattern testing
  async function handleBatchTest() {
    if (!batchPatterns.trim() || !batchTestInputs.trim()) return;
    
    testing = true;
    batchResults = [];
    
    try {
      const patterns = batchPatterns.split('\n').filter(p => p.trim());
      const inputs = batchTestInputs.split('\n').filter(i => i.trim());
      
      const results = [];
      
      for (const testPattern of patterns) {
        for (const testInput of inputs) {
          const matches = doesNodeMatch(
            { name: testInput }, 
            testPattern.trim(), 
            patternType
          );
          
          results.push({
            pattern: testPattern.trim(),
            input: testInput.trim(),
            matches,
            action: matches ? testAction : (testAction === 'allow' ? 'deny' : 'allow')
          });
        }
      }
      
      batchResults = results;
    } catch (error) {
      console.error('Batch test error:', error);
    } finally {
      testing = false;
    }
  }
  
  // Clear pattern and results
  function clearPattern() {
    if (testMode === 'single') {
      pattern = '';
      patternTestResults = null;
    } else {
      batchPatterns = '';
      batchTestInputs = '';
      batchResults = [];
    }
  }
  
  // Get matches from tree data
  function getMatchingNodes(): any[] {
    if (!treeData || !patternTestResults) return [];
    
    const matches: any[] = [];
    
    function checkNode(node: any) {
      // Check if this node matches the pattern
      if (doesNodeMatch(node, pattern, patternType)) {
        matches.push({
          id: node.id,
          name: node.name,
          type: node.type,
          path: getNodePath(node),
          level: node.level
        });
      }
      
      // Recursively check children
      if (node.children) {
        node.children.forEach(checkNode);
      }
    }
    
    if (treeData.children) {
      treeData.children.forEach(checkNode);
    }
    
    return matches;
  }
  
  // Check if a node matches the pattern
  function doesNodeMatch(node: any, testPattern: string, type: string): boolean {
    if (!testPattern.trim()) return false;
    
    const name = node.name.toLowerCase();
    const pattern = testPattern.toLowerCase();
    
    switch (type) {
      case 'exact':
        return name === pattern;
      case 'glob':
        // Convert glob to regex
        const globRegex = new RegExp(
          pattern
            .replace(/\*/g, '.*')
            .replace(/\?/g, '.')
        );
        return globRegex.test(name);
      case 'regex':
        try {
          const regex = new RegExp(pattern);
          return regex.test(name);
        } catch {
          return false;
        }
      default:
        return false;
    }
  }
  
  // Get node path for display
  function getNodePath(node: any): string {
    const path = [];
    let current = node;
    
    while (current && current.type !== 'root') {
      path.unshift(current.name);
      current = current.parent;
    }
    
    return path.join(' > ');
  }
  
  $: matchingNodes = getMatchingNodes();
  
  // Example patterns
  const examplePatterns = {
    regex: [
      '^file.*',
      '.*_test$',
      '(read|write)_.*',
      'debug|test|dev'
    ],
    glob: [
      'file*',
      '*_test',
      'General*',
      'Network*'
    ],
    exact: [
      'file_read',
      'General Operations',
      'ping',
      'curl'
    ]
  };
</script>

<div class="space-y-4">
  <!-- Mode Toggle -->
  <div class="flex gap-2 mb-4">
    <button
      on:click={() => testMode = 'single'}
      class="px-3 py-2 text-sm font-medium rounded-md {testMode === 'single' ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
    >
      Single Pattern
    </button>
    <button
      on:click={() => testMode = 'batch'}
      class="px-3 py-2 text-sm font-medium rounded-md {testMode === 'batch' ? 'bg-blue-600 text-white' : 'bg-gray-100 text-gray-700 hover:bg-gray-200'}"
    >
      Batch Testing
    </button>
  </div>

  {#if testMode === 'single'}
    <!-- Single Pattern Input -->
    <div>
      <label for="pattern" class="block text-sm font-medium text-gray-700 mb-2">
        Test Pattern
      </label>
    <div class="space-y-2">
      <input
        id="pattern"
        type="text"
        bind:value={pattern}
        placeholder="Enter pattern to test..."
        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        on:keydown={(e) => e.key === 'Enter' && handleTest()}
      />
      
      <!-- Pattern Type -->
      <div class="flex gap-2">
        <select bind:value={patternType} class="text-xs px-2 py-1 border border-gray-300 rounded">
          <option value="regex">Regex</option>
          <option value="glob">Glob</option>
          <option value="exact">Exact</option>
        </select>
        <select bind:value={testAction} class="text-xs px-2 py-1 border border-gray-300 rounded">
          <option value="allow">Allow</option>
          <option value="deny">Deny</option>
        </select>
      </div>
    </div>
  </div>

  <!-- Action Buttons -->
  <div class="flex gap-2">
    <button
      on:click={handleTest}
      disabled={!pattern.trim() || testing}
      class="flex-1 px-3 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium"
    >
      {testing ? 'Testing...' : 'Test Pattern'}
    </button>
    <button
      on:click={clearPattern}
      class="px-3 py-2 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 text-sm"
    >
      Clear
    </button>
  </div>

  <!-- Example Patterns -->
  <div class="bg-gray-50 p-3 rounded-md">
    <h4 class="text-sm font-medium text-gray-700 mb-2">Example Patterns ({patternType})</h4>
    <div class="space-y-1">
      {#each examplePatterns[patternType] as example}
        <button
          on:click={() => pattern = example}
          class="block w-full text-left text-xs text-gray-600 hover:text-blue-600 hover:bg-white px-2 py-1 rounded transition-colors font-mono"
        >
          {example}
        </button>
      {/each}
    </div>
  </div>
  {:else}
    <!-- Batch Testing Interface -->
    <div class="space-y-4">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <!-- Batch Patterns -->
        <div>
          <label for="batch-patterns" class="block text-sm font-medium text-gray-700 mb-2">
            Patterns (one per line)
          </label>
          <textarea
            id="batch-patterns"
            bind:value={batchPatterns}
            placeholder=".*delete.*&#10;.*remove.*&#10;^admin_.*"
            rows="6"
            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 font-mono text-sm"
          ></textarea>
        </div>
        
        <!-- Test Inputs -->
        <div>
          <label for="batch-inputs" class="block text-sm font-medium text-gray-700 mb-2">
            Test Inputs (one per line)
          </label>
          <textarea
            id="batch-inputs"
            bind:value={batchTestInputs}
            placeholder="file_delete&#10;admin_user_create&#10;data_remove&#10;user_read"
            rows="6"
            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 font-mono text-sm"
          ></textarea>
        </div>
      </div>
      
      <!-- Pattern Type and Action for Batch -->
      <div class="flex gap-2">
        <select bind:value={patternType} class="text-sm px-3 py-2 border border-gray-300 rounded">
          <option value="regex">Regex</option>
          <option value="glob">Glob</option>
          <option value="exact">Exact</option>
        </select>
        <select bind:value={testAction} class="text-sm px-3 py-2 border border-gray-300 rounded">
          <option value="allow">Allow</option>
          <option value="deny">Deny</option>
        </select>
      </div>
      
      <!-- Batch Test Button -->
      <div class="flex gap-2">
        <button
          on:click={handleTest}
          disabled={!batchPatterns.trim() || !batchTestInputs.trim() || testing}
          class="flex-1 px-3 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium"
        >
          {testing ? 'Testing...' : 'Run Batch Test'}
        </button>
        <button
          on:click={clearPattern}
          class="px-3 py-2 bg-gray-100 text-gray-700 rounded-md hover:bg-gray-200 text-sm"
        >
          Clear
        </button>
      </div>
    </div>
  {/if}

  <!-- Single Pattern Results -->
  {#if testMode === 'single' && pattern && matchingNodes.length > 0}
    <div class="border border-gray-200 rounded-md">
      <div class="bg-gray-50 px-3 py-2 border-b border-gray-200">
        <h4 class="text-sm font-medium text-gray-700">
          Matching Nodes ({matchingNodes.length})
        </h4>
      </div>
      <div class="max-h-64 overflow-y-auto">
        {#each matchingNodes as match}
          <div class="flex items-center justify-between px-3 py-2 border-b border-gray-100 last:border-b-0">
            <div class="flex items-center gap-2">
              <span class="text-sm">
                {match.type === 'server' ? '🔌' : '🔧'}
              </span>
              <div>
                <div class="text-sm font-medium text-gray-900">{match.name}</div>
                <div class="text-xs text-gray-500">{match.path}</div>
              </div>
            </div>
            <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {testAction === 'allow' ? 'text-green-700 bg-green-100' : 'text-red-700 bg-red-100'}">
              {testAction === 'allow' ? '✅ Allow' : '🚫 Deny'}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {:else if testMode === 'single' && pattern && matchingNodes.length === 0}
    <div class="text-center py-4 text-gray-500 text-sm">
      No nodes match this pattern
    </div>
  {/if}
  
  <!-- Batch Test Results -->
  {#if testMode === 'batch' && batchResults.length > 0}
    <div class="border border-gray-200 rounded-md">
      <div class="bg-gray-50 px-3 py-2 border-b border-gray-200">
        <h4 class="text-sm font-medium text-gray-700">
          Batch Test Results ({batchResults.length} tests)
        </h4>
      </div>
      <div class="max-h-96 overflow-y-auto">
        {#each batchResults as result, index}
          <div class="flex items-center justify-between px-3 py-2 border-b border-gray-100 last:border-b-0">
            <div class="flex items-center gap-3 flex-1">
              <span class="text-xs text-gray-400 w-8">{index + 1}</span>
              <div class="flex-1">
                <div class="text-sm font-mono text-gray-900">{result.input}</div>
                <div class="text-xs text-gray-500 font-mono">Pattern: {result.pattern}</div>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {result.matches ? 'text-green-700 bg-green-100' : 'text-gray-700 bg-gray-100'}">
                {result.matches ? '✓ Match' : '✗ No Match'}
              </span>
              <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {result.action === 'allow' ? 'text-green-700 bg-green-100' : 'text-red-700 bg-red-100'}">
                {result.action === 'allow' ? '✅ Allow' : '🚫 Deny'}
              </span>
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Pattern Guide -->
  <div class="bg-blue-50 p-3 rounded-md">
    <h4 class="text-sm font-medium text-blue-800 mb-2">Pattern Guide</h4>
    <div class="text-xs text-blue-700 space-y-1">
      {#if patternType === 'regex'}
        <div><code>^file.*</code> - Starts with "file"</div>
        <div><code>.*_test$</code> - Ends with "_test"</div>
        <div><code>(read|write)_.*</code> - Starts with "read_" or "write_"</div>
      {:else if patternType === 'glob'}
        <div><code>file*</code> - Starts with "file"</div>
        <div><code>*_test</code> - Ends with "_test"</div>
        <div><code>*Operations</code> - Ends with "Operations"</div>
      {:else}
        <div><code>file_read</code> - Exact match for "file_read"</div>
        <div><code>ping</code> - Exact match for "ping"</div>
        <div>Case insensitive matching</div>
      {/if}
    </div>
  </div>
</div>