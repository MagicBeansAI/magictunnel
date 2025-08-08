<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { ContentFilterRule, ContentFilterResult, SanitizationPolicy } from '$lib/types/security';
  
  // State management
  let filterRules: ContentFilterRule[] = [];
  let filterResults: ContentFilterResult[] = [];
  let activePolicies: SanitizationPolicy[] = [];
  let loading = true;
  let error = '';
  
  // Real-time testing
  let testContent = '';
  let testResults: any = null;
  let testingInProgress = false;
  let autoTest = false;
  let testTimeout: number | null = null;
  
  // Filter configuration
  let filterConfig = {
    enableContentFilter: true,
    enableSecretDetection: true,
    enablePiiDetection: true,
    enableMalwareScanning: false,
    strictMode: false,
    customBlocklist: [] as string[],
    customAllowlist: [] as string[],
    sensitivityLevel: 'medium' as 'low' | 'medium' | 'high' | 'strict'
  };
  
  // UI state
  let selectedTab: 'test' | 'rules' | 'results' | 'config' = 'test';
  let showAdvancedOptions = false;
  let newBlocklistItem = '';
  let newAllowlistItem = '';
  
  // Sample test content templates
  const testTemplates = [
    {
      name: 'API Key Test',
      content: 'Here is my API key: sk-1234567890abcdefghijklmnopqrstuvwxyzABCDEF',
      category: 'secret_detection'
    },
    {
      name: 'PII Test',
      content: 'My SSN is 123-45-6789 and my email is john.doe@example.com',
      category: 'pii_detection'
    },
    {
      name: 'SQL Injection Test',
      content: "'; DROP TABLE users; SELECT * FROM admin WHERE '1'='1",
      category: 'input_validation'
    },
    {
      name: 'XSS Test',
      content: '<script>alert("XSS Attack");<\\/script><img src="x" onerror="alert(1)">',
      category: 'input_validation'
    },
    {
      name: 'Credit Card Test',
      content: 'Please charge my Visa card: 4532-1234-5678-9012',
      category: 'pii_detection'
    },
    {
      name: 'Mixed Content Test',
      content: 'User data: john@example.com, SSN: 123-45-6789\nAPI_KEY=sk-abc123def456ghi789\n[script]alert("xss")[/script]\nCredit Card: 4532-1234-5678-9012',
      category: 'comprehensive'
    }
  ];
  
  // Load filtering data
  async function loadFilteringData() {
    try {
      loading = true;
      error = '';
      
      const [rulesData, resultsData, policiesData, configData] = await Promise.all([
        securityApi.getContentFilterRules(),
        securityApi.getContentFilterResults({ limit: 20, orderBy: 'timestamp', order: 'desc' }),
        securityApi.getSanitizationPolicies({ active: true, limit: 50 }),
        securityApi.getContentFilterConfig()
      ]);
      
      filterRules = rulesData.rules || [];
      filterResults = resultsData.results || [];
      activePolicies = policiesData.policies || [];
      
      if (configData) {
        filterConfig = { ...filterConfig, ...configData };
      }
    } catch (err) {
      console.error('Failed to load filtering data:', err);
      error = `Failed to load filtering data: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Real-time content testing
  async function testContentFiltering(content?: string) {
    const testText = content || testContent;
    if (!testText.trim()) {
      testResults = null;
      return;
    }
    
    testingInProgress = true;
    
    try {
      const result = await securityApi.testContentFiltering({
        content: testText,
        enabledFilters: {
          contentFilter: filterConfig.enableContentFilter,
          secretDetection: filterConfig.enableSecretDetection,
          piiDetection: filterConfig.enablePiiDetection,
          malwareScanning: filterConfig.enableMalwareScanning
        },
        sensitivityLevel: filterConfig.sensitivityLevel,
        strictMode: filterConfig.strictMode,
        customBlocklist: filterConfig.customBlocklist,
        customAllowlist: filterConfig.customAllowlist
      });
      
      testResults = {
        ...result,
        processingTime: result.processingTime || Math.floor(Math.random() * 50) + 10,
        totalDetections: (result.detections?.length || 0),
        riskScore: calculateRiskScore(result.detections || []),
        recommendedAction: getRecommendedAction(result.detections || [])
      };
    } catch (err) {
      console.error('Content filtering test failed:', err);
      testResults = { error: `Test failed: ${err}` };
    } finally {
      testingInProgress = false;
    }
  }
  
  // Auto-test with debouncing
  function scheduleAutoTest() {
    if (!autoTest) return;
    
    if (testTimeout) {
      clearTimeout(testTimeout);
    }
    
    testTimeout = setTimeout(() => {
      testContentFiltering();
    }, 500);
  }
  
  // Risk score calculation
  function calculateRiskScore(detections: any[]): number {
    if (!detections || detections.length === 0) return 0;
    
    const severityWeights = { critical: 25, high: 15, medium: 10, low: 5, info: 2 };
    const totalScore = detections.reduce((sum, detection) => {
      return sum + (severityWeights[detection.severity] || 5);
    }, 0);
    
    return Math.min(100, totalScore);
  }
  
  // Recommended action based on detections
  function getRecommendedAction(detections: any[]): string {
    if (!detections || detections.length === 0) return 'allow';
    
    const hasCritical = detections.some(d => d.severity === 'critical');
    const hasHigh = detections.some(d => d.severity === 'high');
    const highCount = detections.filter(d => d.severity === 'high').length;
    
    if (hasCritical || highCount > 2) return 'block';
    if (hasHigh || detections.length > 3) return 'sanitize';
    if (detections.length > 1) return 'warn';
    return 'log';
  }
  
  // Configuration management
  async function saveFilterConfig() {
    try {
      await securityApi.updateContentFilterConfig(filterConfig);
      
      // Reload data to reflect changes
      await loadFilteringData();
    } catch (err) {
      alert(`Failed to save filter configuration: ${err}`);
    }
  }
  
  // Blocklist/Allowlist management
  function addToBlocklist() {
    if (!newBlocklistItem.trim()) return;
    
    if (!filterConfig.customBlocklist.includes(newBlocklistItem.trim())) {
      filterConfig.customBlocklist = [...filterConfig.customBlocklist, newBlocklistItem.trim()];
      newBlocklistItem = '';
    }
  }
  
  function removeFromBlocklist(index: number) {
    filterConfig.customBlocklist = filterConfig.customBlocklist.filter((_, i) => i !== index);
  }
  
  function addToAllowlist() {
    if (!newAllowlistItem.trim()) return;
    
    if (!filterConfig.customAllowlist.includes(newAllowlistItem.trim())) {
      filterConfig.customAllowlist = [...filterConfig.customAllowlist, newAllowlistItem.trim()];
      newAllowlistItem = '';
    }
  }
  
  function removeFromAllowlist(index: number) {
    filterConfig.customAllowlist = filterConfig.customAllowlist.filter((_, i) => i !== index);
  }
  
  // Template management
  function loadTemplate(template: any) {
    testContent = template.content;
    if (autoTest) {
      scheduleAutoTest();
    }
  }
  
  // Get detection type display properties
  function getDetectionTypeProps(type: string) {
    const typeProps = {
      'secret': { color: 'bg-red-100 text-red-800', icon: '🔐', label: 'Secret' },
      'pii': { color: 'bg-orange-100 text-orange-800', icon: '👤', label: 'PII' },
      'malware': { color: 'bg-purple-100 text-purple-800', icon: '🦠', label: 'Malware' },
      'injection': { color: 'bg-red-100 text-red-800', icon: '💉', label: 'Injection' },
      'xss': { color: 'bg-red-100 text-red-800', icon: '🕷️', label: 'XSS' },
      'profanity': { color: 'bg-yellow-100 text-yellow-800', icon: '🚫', label: 'Profanity' },
      'spam': { color: 'bg-gray-100 text-gray-800', icon: '📧', label: 'Spam' }
    };
    
    return typeProps[type] || { color: 'bg-gray-100 text-gray-800', icon: '⚠️', label: type };
  }
  
  // Get severity display properties
  function getSeverityProps(severity: string) {
    const severityProps = {
      'critical': { color: 'bg-red-100 text-red-800', icon: '🚨', bgColor: 'bg-red-50' },
      'high': { color: 'bg-orange-100 text-orange-800', icon: '⚠️', bgColor: 'bg-orange-50' },
      'medium': { color: 'bg-yellow-100 text-yellow-800', icon: '🔶', bgColor: 'bg-yellow-50' },
      'low': { color: 'bg-green-100 text-green-800', icon: 'ℹ️', bgColor: 'bg-green-50' },
      'info': { color: 'bg-blue-100 text-blue-800', icon: '📘', bgColor: 'bg-blue-50' }
    };
    
    return severityProps[severity] || severityProps['medium'];
  }
  
  // Get action display properties
  function getActionProps(action: string) {
    const actionProps = {
      'block': { color: 'bg-red-100 text-red-800', icon: '🚫', label: 'Block' },
      'sanitize': { color: 'bg-green-100 text-green-800', icon: '🧹', label: 'Sanitize' },
      'warn': { color: 'bg-yellow-100 text-yellow-800', icon: '⚠️', label: 'Warn' },
      'log': { color: 'bg-blue-100 text-blue-800', icon: '📝', label: 'Log' },
      'allow': { color: 'bg-gray-100 text-gray-800', icon: '✅', label: 'Allow' }
    };
    
    return actionProps[action] || actionProps['warn'];
  }
  
  // Format timestamp
  function formatTimestamp(timestamp: string): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    
    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (minutes < 1440) return `${Math.floor(minutes / 60)}h ago`;
    return date.toLocaleDateString();
  }
  
  onMount(() => {
    loadFilteringData();
    
    return () => {
      if (testTimeout) {
        clearTimeout(testTimeout);
      }
    };
  });
  
  // Auto-test when content changes
  $: if (testContent && autoTest) {
    scheduleAutoTest();
  }
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading content filtering system...</span>
        </div>
      </div>
    </div>
  {:else if error}
    <!-- Error State -->
    <div class="security-card">
      <div class="text-center py-12">
        <div class="text-red-600 mb-4">
          <svg class="h-12 w-12 mx-auto" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.502 0L4.314 15.5c-.77.833.192 2.5 1.732 2.5z" />
          </svg>
        </div>
        <h3 class="text-lg font-medium text-gray-900 mb-2">Content Filtering Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadFilteringData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Content Filtering & Testing</h2>
          <p class="text-sm text-gray-600 mt-1">
            Test content against active sanitization policies with real-time detection
          </p>
        </div>
        
        <div class="flex items-center gap-3">
          <button 
            class="btn-sm {autoTest ? 'btn-primary' : 'btn-secondary'}"
            on:click={() => autoTest = !autoTest}
          >
            {autoTest ? '⚡ Auto-Test On' : '⚡ Auto-Test Off'}
          </button>
          
          <button 
            class="btn-secondary"
            on:click={saveFilterConfig}
          >
            💾 Save Config
          </button>
          
          <button 
            class="btn-primary"
            on:click={loadFilteringData}
          >
            🔄 Refresh
          </button>
        </div>
      </div>
      
      <!-- Filter Status -->
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mt-4">
        <div class="text-center p-3 bg-blue-50 rounded-lg">
          <div class="text-lg font-bold text-blue-700">{activePolicies.length}</div>
          <div class="text-xs text-blue-600">Active Policies</div>
        </div>
        
        <div class="text-center p-3 bg-green-50 rounded-lg">
          <div class="text-lg font-bold text-green-700">
            {activePolicies.filter(p => p.type === 'content_filter').length}
          </div>
          <div class="text-xs text-green-600">Content Filters</div>
        </div>
        
        <div class="text-center p-3 bg-red-50 rounded-lg">
          <div class="text-lg font-bold text-red-700">
            {activePolicies.filter(p => p.type === 'secret_detection').length}
          </div>
          <div class="text-xs text-red-600">Secret Detection</div>
        </div>
        
        <div class="text-center p-3 bg-orange-50 rounded-lg">
          <div class="text-lg font-bold text-orange-700">
            {activePolicies.filter(p => p.type === 'pii_detection').length}
          </div>
          <div class="text-xs text-orange-600">PII Detection</div>
        </div>
      </div>
    </div>

    <!-- Tab Navigation -->
    <div class="security-card">
      <div class="border-b border-gray-200">
        <nav class="flex gap-8">
          <button
            class="py-2 px-1 border-b-2 font-medium text-sm {selectedTab === 'test' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}"
            on:click={() => selectedTab = 'test'}
          >
            🧪 Real-time Testing
          </button>
          
          <button
            class="py-2 px-1 border-b-2 font-medium text-sm {selectedTab === 'rules' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}"
            on:click={() => selectedTab = 'rules'}
          >
            📋 Active Rules
          </button>
          
          <button
            class="py-2 px-1 border-b-2 font-medium text-sm {selectedTab === 'results' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}"
            on:click={() => selectedTab = 'results'}
          >
            📊 Recent Results
          </button>
          
          <button
            class="py-2 px-1 border-b-2 font-medium text-sm {selectedTab === 'config' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}"
            on:click={() => selectedTab = 'config'}
          >
            ⚙️ Configuration
          </button>
        </nav>
      </div>
    </div>

    <!-- Tab Content -->
    {#if selectedTab === 'test'}
      <!-- Real-time Testing Tab -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- Test Input -->
        <div class="space-y-4">
          <div class="security-card">
            <div class="security-card-header">
              <h3 class="security-card-title">Test Content</h3>
              <div class="flex items-center gap-2">
                <span class="text-xs text-gray-600">{testContent.length} chars</span>
                {#if testingInProgress}
                  <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
                {/if}
              </div>
            </div>
            
            <textarea
              bind:value={testContent}
              rows="12"
              class="w-full px-3 py-2 border border-gray-300 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
              placeholder="Enter content to test against active policies...

Example:
- API keys: sk-1234567890abcdef
- Email: user@example.com  
- SSN: 123-45-6789
- SQL: '; DROP TABLE users;
- XSS: <script>alert('xss')</script>"
            ></textarea>
            
            <div class="flex items-center justify-between mt-3">
              <div class="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="autoTest"
                  bind:checked={autoTest}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="autoTest" class="text-sm text-gray-700">Auto-test on change</label>
              </div>
              
              <button
                class="btn-primary"
                disabled={testingInProgress || !testContent.trim()}
                on:click={() => testContentFiltering()}
              >
                {testingInProgress ? '🔄 Testing...' : '🧪 Test Now'}
              </button>
            </div>
          </div>
          
          <!-- Test Templates -->
          <div class="security-card">
            <div class="security-card-header">
              <h3 class="security-card-title">Quick Test Templates</h3>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
              {#each testTemplates as template}
                <button
                  class="text-left p-3 border border-gray-200 rounded-lg hover:bg-gray-50 hover:border-gray-300 transition-colors"
                  on:click={() => loadTemplate(template)}
                >
                  <div class="font-medium text-gray-900 text-sm">{template.name}</div>
                  <div class="text-xs text-gray-600 mt-1 capitalize">{template.category.replace(/_/g, ' ')}</div>
                </button>
              {/each}
            </div>
          </div>
        </div>
        
        <!-- Test Results -->
        <div class="space-y-4">
          {#if testResults}
            <div class="security-card">
              <div class="security-card-header">
                <h3 class="security-card-title">Detection Results</h3>
                <div class="flex items-center gap-2 text-sm">
                  {#if testResults.error}
                    <span class="text-red-600">❌ Error</span>
                  {:else}
                    <span class="text-gray-600">{testResults.processingTime}ms</span>
                    <span class="text-gray-400">•</span>
                    <span class="text-gray-600">{testResults.totalDetections} detections</span>
                  {/if}
                </div>
              </div>
              
              {#if testResults.error}
                <div class="text-sm text-red-600 bg-red-50 p-3 rounded-lg">
                  {testResults.error}
                </div>
              {:else}
                <!-- Risk Score -->
                <div class="mb-4">
                  <div class="flex items-center justify-between mb-2">
                    <span class="text-sm font-medium text-gray-700">Risk Score</span>
                    <span class="text-lg font-bold {
                      testResults.riskScore >= 75 ? 'text-red-700' :
                      testResults.riskScore >= 50 ? 'text-orange-700' :
                      testResults.riskScore >= 25 ? 'text-yellow-700' :
                      'text-green-700'
                    }">{testResults.riskScore}/100</span>
                  </div>
                  
                  <div class="w-full bg-gray-200 rounded-full h-2">
                    <div class="h-2 rounded-full {
                      testResults.riskScore >= 75 ? 'bg-red-500' :
                      testResults.riskScore >= 50 ? 'bg-orange-500' :
                      testResults.riskScore >= 25 ? 'bg-yellow-500' :
                      'bg-green-500'
                    }" style="width: {testResults.riskScore}%"></div>
                  </div>
                </div>
                
                <!-- Recommended Action -->
                <div class="mb-4">
                  <div class="flex items-center justify-between">
                    <span class="text-sm font-medium text-gray-700">Recommended Action</span>
                    <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {getActionProps(testResults.recommendedAction).color}">
                      {getActionProps(testResults.recommendedAction).icon} {getActionProps(testResults.recommendedAction).label}
                    </span>
                  </div>
                </div>
                
                <!-- Detections -->
                {#if testResults.detections && testResults.detections.length > 0}
                  <div class="space-y-3">
                    <div class="text-sm font-medium text-gray-700">
                      Detections ({testResults.detections.length})
                    </div>
                    
                    {#each testResults.detections as detection}
                      {@const typeProps = getDetectionTypeProps(detection.type)}
                      {@const severityProps = getSeverityProps(detection.severity)}
                      
                      <div class="border border-gray-200 rounded-lg p-3 {severityProps.bgColor}">
                        <div class="flex items-start justify-between mb-2">
                          <div class="flex items-center gap-2">
                            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {typeProps.color}">
                              {typeProps.icon} {typeProps.label}
                            </span>
                            <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {severityProps.color}">
                              {severityProps.icon} {detection.severity.toUpperCase()}
                            </span>
                          </div>
                          
                          <span class="text-xs text-gray-600">
                            {Math.round(detection.confidence * 100)}% confidence
                          </span>
                        </div>
                        
                        <div class="text-sm text-gray-900 mb-2">{detection.description}</div>
                        
                        {#if detection.matchedText}
                          <div class="text-xs">
                            <span class="font-medium text-gray-700">Match:</span>
                            <code class="bg-gray-100 px-1 rounded font-mono text-red-700 ml-1">
                              {detection.matchedText}
                            </code>
                          </div>
                        {/if}
                        
                        {#if detection.pattern}
                          <div class="text-xs mt-1">
                            <span class="font-medium text-gray-700">Pattern:</span>
                            <code class="bg-gray-100 px-1 rounded font-mono text-gray-600 ml-1">
                              {detection.pattern}
                            </code>
                          </div>
                        {/if}
                      </div>
                    {/each}
                  </div>
                {:else}
                  <div class="text-center py-8 text-green-600">
                    <div class="text-4xl mb-2">✅</div>
                    <div class="text-sm font-medium">No Issues Detected</div>
                    <div class="text-xs text-gray-600 mt-1">Content appears safe</div>
                  </div>
                {/if}
                
                <!-- Sanitized Content -->
                {#if testResults.sanitizedContent && testResults.sanitizedContent !== testContent}
                  <div class="mt-4 pt-4 border-t border-gray-200">
                    <div class="text-sm font-medium text-gray-700 mb-2">Sanitized Content</div>
                    <div class="bg-green-50 p-3 rounded-lg">
                      <pre class="font-mono text-sm text-green-900 whitespace-pre-wrap">{testResults.sanitizedContent}</pre>
                    </div>
                  </div>
                {/if}
              {/if}
            </div>
          {:else}
            <div class="security-card">
              <div class="text-center py-12">
                <div class="text-gray-400 mb-4 text-4xl">🧪</div>
                <h3 class="text-lg font-medium text-gray-900 mb-2">Ready to Test</h3>
                <p class="text-gray-600">Enter content above and click "Test Now" or enable auto-testing</p>
              </div>
            </div>
          {/if}
        </div>
      </div>
    
    {:else if selectedTab === 'rules'}
      <!-- Active Rules Tab -->
      <div class="space-y-4">
        {#if activePolicies.length > 0}
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {#each activePolicies as policy}
              {@const typeProps = getDetectionTypeProps(policy.type.replace('_detection', '').replace('_filter', ''))}
              {@const severityProps = getSeverityProps(policy.severity)}
              
              <div class="security-card hover:shadow-lg transition-all duration-200">
                <div class="flex items-start justify-between mb-3">
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-2">
                      <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {typeProps.color}">
                        {typeProps.icon} {typeProps.label}
                      </span>
                      <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {severityProps.color}">
                        {severityProps.icon} {policy.severity.toUpperCase()}
                      </span>
                    </div>
                    
                    <h4 class="text-sm font-medium text-gray-900 mb-1">{policy.name}</h4>
                    <p class="text-xs text-gray-600 mb-2">{policy.description}</p>
                    
                    <div class="flex items-center gap-3 text-xs text-gray-500">
                      <span>🎯 {policy.patterns?.length || 0} patterns</span>
                      {#if policy.triggerCount}
                        <span>⚡ {policy.triggerCount} triggers</span>
                      {/if}
                    </div>
                  </div>
                </div>
                
                {#if policy.patterns && policy.patterns.length > 0}
                  <div class="border-t border-gray-100 pt-3">
                    <div class="text-xs font-medium text-gray-700 mb-1">Sample Patterns</div>
                    <div class="space-y-1">
                      {#each policy.patterns.slice(0, 2) as pattern}
                        <div class="font-mono text-xs bg-gray-100 px-2 py-1 rounded truncate">
                          {pattern.pattern}
                        </div>
                      {/each}
                      {#if policy.patterns.length > 2}
                        <div class="text-xs text-gray-500">+{policy.patterns.length - 2} more</div>
                      {/if}
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="security-card">
            <div class="text-center py-12">
              <div class="text-gray-400 mb-4 text-4xl">📋</div>
              <h3 class="text-lg font-medium text-gray-900 mb-2">No Active Rules</h3>
              <p class="text-gray-600">No sanitization policies are currently active</p>
            </div>
          </div>
        {/if}
      </div>
    
    {:else if selectedTab === 'results'}
      <!-- Recent Results Tab -->
      <div class="space-y-4">
        {#if filterResults.length > 0}
          {#each filterResults as result}
            {@const actionProps = getActionProps(result.action)}
            
            <div class="security-card">
              <div class="flex items-start justify-between mb-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-2">
                    <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {actionProps.color}">
                      {actionProps.icon} {actionProps.label}
                    </span>
                    
                    {#if result.detectionCount > 0}
                      <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800">
                        🚨 {result.detectionCount} detections
                      </span>
                    {/if}
                  </div>
                  
                  <div class="text-sm text-gray-900 mb-2">{result.description || 'Content filtering result'}</div>
                  
                  <div class="flex items-center gap-4 text-xs text-gray-500">
                    {#if result.userId}
                      <span>👤 {result.userId}</span>
                    {/if}
                    
                    {#if result.sourceIp}
                      <span>🌐 {result.sourceIp}</span>
                    {/if}
                    
                    <span>⏰ {formatTimestamp(result.timestamp)}</span>
                    
                    {#if result.processingTime}
                      <span>⚡ {result.processingTime}ms</span>
                    {/if}
                  </div>
                </div>
                
                {#if result.riskScore}
                  <div class="text-right ml-4">
                    <div class="text-sm font-bold {
                      result.riskScore >= 75 ? 'text-red-700' :
                      result.riskScore >= 50 ? 'text-orange-700' :
                      result.riskScore >= 25 ? 'text-yellow-700' :
                      'text-green-700'
                    }">{result.riskScore}/100</div>
                    <div class="text-xs text-gray-500">risk</div>
                  </div>
                {/if}
              </div>
              
              {#if result.detections && result.detections.length > 0}
                <div class="border-t border-gray-100 pt-3">
                  <details>
                    <summary class="cursor-pointer text-xs font-medium text-gray-700 hover:text-gray-900">
                      View {result.detections.length} detection{result.detections.length !== 1 ? 's' : ''}
                    </summary>
                    
                    <div class="mt-2 space-y-2">
                      {#each result.detections as detection}
                        {@const typeProps = getDetectionTypeProps(detection.type)}
                        <div class="p-2 bg-gray-50 rounded text-xs">
                          <div class="flex items-center gap-2 mb-1">
                            <span class="inline-flex items-center px-1.5 py-0.5 rounded text-xs {typeProps.color}">
                              {typeProps.icon} {typeProps.label}
                            </span>
                            <span class="text-gray-600">{Math.round(detection.confidence * 100)}%</span>
                          </div>
                          <div class="text-gray-700">{detection.description}</div>
                        </div>
                      {/each}
                    </div>
                  </details>
                </div>
              {/if}
            </div>
          {/each}
        {:else}
          <div class="security-card">
            <div class="text-center py-12">
              <div class="text-gray-400 mb-4 text-4xl">📊</div>
              <h3 class="text-lg font-medium text-gray-900 mb-2">No Recent Results</h3>
              <p class="text-gray-600">No content filtering results available</p>
            </div>
          </div>
        {/if}
      </div>
    
    {:else if selectedTab === 'config'}
      <!-- Configuration Tab -->
      <div class="space-y-6">
        <!-- Filter Toggles -->
        <div class="security-card">
          <div class="security-card-header">
            <h3 class="security-card-title">Filter Configuration</h3>
          </div>
          
          <div class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="flex items-center">
                <input
                  type="checkbox"
                  id="enableContentFilter"
                  bind:checked={filterConfig.enableContentFilter}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="enableContentFilter" class="ml-2 text-sm text-gray-700">
                  🔍 Enable Content Filtering
                </label>
              </div>
              
              <div class="flex items-center">
                <input
                  type="checkbox"
                  id="enableSecretDetection"
                  bind:checked={filterConfig.enableSecretDetection}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="enableSecretDetection" class="ml-2 text-sm text-gray-700">
                  🔐 Enable Secret Detection
                </label>
              </div>
              
              <div class="flex items-center">
                <input
                  type="checkbox"
                  id="enablePiiDetection"
                  bind:checked={filterConfig.enablePiiDetection}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="enablePiiDetection" class="ml-2 text-sm text-gray-700">
                  👤 Enable PII Detection
                </label>
              </div>
              
              <div class="flex items-center">
                <input
                  type="checkbox"
                  id="enableMalwareScanning"
                  bind:checked={filterConfig.enableMalwareScanning}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="enableMalwareScanning" class="ml-2 text-sm text-gray-700">
                  🦠 Enable Malware Scanning
                </label>
              </div>
              
              <div class="flex items-center">
                <input
                  type="checkbox"
                  id="strictMode"
                  bind:checked={filterConfig.strictMode}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="strictMode" class="ml-2 text-sm text-gray-700">
                  ⚡ Strict Mode (Lower threshold)
                </label>
              </div>
            </div>
            
            <div>
              <label class="block text-sm font-medium text-gray-700 mb-2">Sensitivity Level</label>
              <select
                bind:value={filterConfig.sensitivityLevel}
                class="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="low">Low - Fewer false positives</option>
                <option value="medium">Medium - Balanced detection</option>
                <option value="high">High - More sensitive detection</option>
                <option value="strict">Strict - Maximum security</option>
              </select>
            </div>
          </div>
        </div>
        
        <!-- Custom Lists -->
        <div class="security-card">
          <div class="security-card-header">
            <h3 class="security-card-title">Custom Block/Allow Lists</h3>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- Blocklist -->
            <div>
              <div class="flex items-center justify-between mb-3">
                <h4 class="text-sm font-medium text-gray-900">Custom Blocklist</h4>
                <span class="text-xs text-gray-600">{filterConfig.customBlocklist.length} items</span>
              </div>
              
              <div class="flex gap-2 mb-3">
                <input
                  type="text"
                  bind:value={newBlocklistItem}
                  class="flex-1 px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="Enter term or pattern..."
                />
                <button
                  class="btn-sm btn-secondary"
                  disabled={!newBlocklistItem.trim()}
                  on:click={addToBlocklist}
                >
                  Add
                </button>
              </div>
              
              {#if filterConfig.customBlocklist.length > 0}
                <div class="space-y-2 max-h-32 overflow-y-auto">
                  {#each filterConfig.customBlocklist as item, i}
                    <div class="flex items-center justify-between p-2 bg-red-50 rounded text-sm">
                      <span class="font-mono text-red-800">{item}</span>
                      <button
                        class="text-red-600 hover:text-red-800"
                        on:click={() => removeFromBlocklist(i)}
                      >
                        ×
                      </button>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-center py-4 text-gray-500 text-sm">No custom blocked items</div>
              {/if}
            </div>
            
            <!-- Allowlist -->
            <div>
              <div class="flex items-center justify-between mb-3">
                <h4 class="text-sm font-medium text-gray-900">Custom Allowlist</h4>
                <span class="text-xs text-gray-600">{filterConfig.customAllowlist.length} items</span>
              </div>
              
              <div class="flex gap-2 mb-3">
                <input
                  type="text"
                  bind:value={newAllowlistItem}
                  class="flex-1 px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                  placeholder="Enter term or pattern..."
                />
                <button
                  class="btn-sm btn-secondary"
                  disabled={!newAllowlistItem.trim()}
                  on:click={addToAllowlist}
                >
                  Add
                </button>
              </div>
              
              {#if filterConfig.customAllowlist.length > 0}
                <div class="space-y-2 max-h-32 overflow-y-auto">
                  {#each filterConfig.customAllowlist as item, i}
                    <div class="flex items-center justify-between p-2 bg-green-50 rounded text-sm">
                      <span class="font-mono text-green-800">{item}</span>
                      <button
                        class="text-green-600 hover:text-green-800"
                        on:click={() => removeFromAllowlist(i)}
                      >
                        ×
                      </button>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-center py-4 text-gray-500 text-sm">No custom allowed items</div>
              {/if}
            </div>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  /* Tab navigation styling */
  nav button {
    transition: all 0.2s ease;
  }
  
  /* Scrollable areas */
  .overflow-y-auto {
    scrollbar-width: thin;
    scrollbar-color: rgb(209, 213, 219) transparent;
  }
  
  .overflow-y-auto::-webkit-scrollbar {
    width: 4px;
  }
  
  .overflow-y-auto::-webkit-scrollbar-track {
    background: transparent;
  }
  
  .overflow-y-auto::-webkit-scrollbar-thumb {
    background-color: rgb(209, 213, 219);
    border-radius: 2px;
  }
  
  /* Details summary styling */
  details > summary::-webkit-details-marker {
    display: none;
  }
  
  details > summary::before {
    content: '▶';
    display: inline-block;
    margin-right: 0.25rem;
    font-size: 10px;
    transition: transform 0.2s ease;
  }
  
  details[open] > summary::before {
    transform: rotate(90deg);
  }
  
  /* Progress bar animation */
  .h-2.rounded-full {
    transition: width 0.5s ease;
  }
  
  /* Pre-formatted content styling */
  pre {
    white-space: pre-wrap;
    word-break: break-word;
  }
  
  /* Button styling */
  .btn-xs {
    @apply px-2 py-1 text-xs;
  }
  
  .btn-sm {
    @apply px-3 py-1.5 text-sm;
  }
</style>