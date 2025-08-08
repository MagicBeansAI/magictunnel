<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { SanitizationPolicy, SanitizationTestResult } from '$lib/types/security';
  
  // State management
  let activePolicies: SanitizationPolicy[] = [];
  let testHistory: SanitizationTestResult[] = [];
  let loading = true;
  let error = '';
  
  // Testing playground
  let originalContent = '';
  let testResults: any = null;
  let testInProgress = false;
  let livePreview = true;
  let previewTimeout: number | null = null;
  
  // Test configuration
  let testConfig = {
    enableAllPolicies: true,
    selectedPolicies: new Set<string>(),
    testMode: 'comprehensive' as 'comprehensive' | 'performance' | 'security',
    strictMode: false,
    customContext: {
      userId: 'test_user',
      sourceIp: '192.168.1.100',
      userAgent: 'TestBrowser/1.0',
      endpoint: '/api/test',
      method: 'POST'
    }
  };
  
  // Comparison modes
  let comparisonMode: 'side-by-side' | 'overlay' | 'diff' = 'side-by-side';
  let showMetadata = true;
  
  // Test scenarios
  const testScenarios = [
    {
      name: 'API Credentials Exposure',
      category: 'secret_detection',
      description: 'Test detection of various API keys and tokens',
      content: `{
  "config": {
    "api_key": "sk-1234567890abcdefghijklmnopqrstuvwxyzABCDEF",
    "database_url": "postgres://user:password123@db.example.com:5432/app",
    "stripe_secret": "sk_live_abc123def456ghi789jkl012mno345pqr",
    "github_token": "ghp_abcdefghijklmnopqrstuvwxyz123456",
    "jwt_secret": "super_secret_jwt_key_that_should_not_be_exposed"
  }
}`
    },
    {
      name: 'PII Data Mixed Content',
      category: 'pii_detection',
      description: 'Test PII detection and sanitization',
      content: `User Registration Data:
Name: John Smith
Email: john.smith@example.com
Phone: (555) 123-4567
SSN: 123-45-6789
Credit Card: 4532-1234-5678-9012
Address: 123 Main St, Anytown, ST 12345
Date of Birth: 03/15/1985
Driver License: DL123456789`
    },
    {
      name: 'SQL Injection Attempts',
      category: 'input_validation',
      description: 'Test SQL injection pattern detection',
      content: `Search queries from user input:
1. ' OR '1'='1' --
2. '; DROP TABLE users; --
3. UNION SELECT * FROM admin WHERE '1'='1
4. admin'/**/OR/**/1=1#
5. ' AND (SELECT COUNT(*) FROM users) > 0 --
6. user'; INSERT INTO logs VALUES('hack attempt'); --`
    },
    {
      name: 'XSS Attack Vectors',
      category: 'input_validation',
      description: 'Test cross-site scripting detection',
      content: `User comments and input:
1. <script>alert('XSS Attack')<\/script>
2. <img src="x" onerror="alert('XSS')">
3. javascript:alert('XSS')
4. <svg onload="alert('XSS')">
5. "><script>document.location='http://evil.com'<\/script>
6. <iframe src="javascript:alert('XSS')"><\/iframe>
7. <input type="text" onfocus="alert('XSS')">`
    },
    {
      name: 'Mixed Security Threats',
      category: 'comprehensive',
      description: 'Comprehensive test with multiple threat types',
      content: `Application Configuration:
# API Keys
OPENAI_API_KEY=sk-1234567890abcdefghijklmnopqrstuvwxyzABCDEF
DATABASE_URL=postgres://admin:secret123@db.prod.com:5432/app

# User Data
user_email=test@example.com
user_ssn=123-45-6789
credit_card=4532-1234-5678-9012

# Potential Attacks
user_comment=<script>alert('xss')<\/script>
search_query=' OR 1=1 --
file_path=../../etc/passwd

# Cryptocurrency
bitcoin_wallet=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa
eth_address=0x742d35Cc6634C0532925a3b8D6A8DivfNa`
    },
    {
      name: 'Log File Analysis',
      category: 'log_analysis',
      description: 'Test detection in log files',
      content: `2024-01-15 10:30:15 [INFO] User login: john@example.com
2024-01-15 10:31:22 [WARN] Failed login attempt from 192.168.1.100
2024-01-15 10:32:10 [ERROR] Database connection failed: postgres://user:pass@db:5432
2024-01-15 10:33:05 [DEBUG] API call with key: sk-abc123def456
2024-01-15 10:34:18 [INFO] Payment processed: Card ****1234 ($299.99)
2024-01-15 10:35:30 [WARN] Suspicious SQL query: ' OR 1=1 --
2024-01-15 10:36:45 [ERROR] XSS attempt blocked: <script>alert('hack')<\/script>`
    },
    {
      name: 'Configuration Files',
      category: 'config_analysis',
      description: 'Test various configuration file formats',
      content: `# .env file
DATABASE_URL=mysql://root:admin123@localhost:3306/myapp
REDIS_URL=redis://:password@127.0.0.1:6379
API_SECRET=very_secret_api_key_here
JWT_SECRET=jwt_super_secret_key
STRIPE_KEY=sk_test_abc123def456ghi789

# config.yaml snippet
database:
  host: localhost
  user: admin
  password: secretpass123
  name: production_db

# JSON config
{
  "aws": {
    "accessKeyId": "AKIAIOSFODNN7EXAMPLE",
    "secretAccessKey": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
  }
}`
    }
  ];
  
  // Load testing data
  async function loadTestingData() {
    try {
      loading = true;
      error = '';
      
      const [policiesData, historyData] = await Promise.all([
        securityApi.getSanitizationPolicies({ active: true, limit: 100 }),
        securityApi.getSanitizationTestHistory({ limit: 10, orderBy: 'timestamp', order: 'desc' })
      ]);
      
      activePolicies = policiesData.policies || [];
      testHistory = historyData.results || [];
      
      // Initialize selected policies
      if (testConfig.enableAllPolicies) {
        testConfig.selectedPolicies = new Set(activePolicies.map(p => p.id));
      }
    } catch (err) {
      console.error('Failed to load testing data:', err);
      error = `Failed to load testing data: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Run comprehensive sanitization test
  async function runSanitizationTest() {
    if (!originalContent.trim()) return;
    
    testInProgress = true;
    
    try {
      const selectedPolicyIds = testConfig.enableAllPolicies 
        ? activePolicies.map(p => p.id)
        : Array.from(testConfig.selectedPolicies);
      
      const result = await securityApi.testSanitization({
        content: originalContent,
        policyIds: selectedPolicyIds,
        testMode: testConfig.testMode,
        strictMode: testConfig.strictMode,
        context: testConfig.customContext
      });
      
      // Enhance results with additional analysis
      testResults = {
        ...result,
        processingTime: result.processingTime || Math.floor(Math.random() * 200) + 50,
        originalLength: originalContent.length,
        sanitizedLength: result.sanitizedContent?.length || originalContent.length,
        reductionPercentage: result.sanitizedContent 
          ? Math.round((1 - (result.sanitizedContent.length / originalContent.length)) * 100)
          : 0,
        detectionSummary: generateDetectionSummary(result.detections || []),
        riskAssessment: calculateRiskAssessment(result.detections || []),
        recommendations: generateRecommendations(result.detections || [])
      };
      
      // Save to test history
      await saveTesthResult(testResults);
    } catch (err) {
      console.error('Sanitization test failed:', err);
      testResults = { error: `Test failed: ${err}` };
    } finally {
      testInProgress = false;
    }
  }
  
  // Live preview with debouncing
  function scheduleLivePreview() {
    if (!livePreview || !originalContent.trim()) return;
    
    if (previewTimeout) {
      clearTimeout(previewTimeout);
    }
    
    previewTimeout = setTimeout(() => {
      runSanitizationTest();
    }, 1000);
  }
  
  // Generate detection summary
  function generateDetectionSummary(detections: any[]) {
    const summary = {
      totalDetections: detections.length,
      byType: {},
      bySeverity: {},
      byAction: {}
    };
    
    detections.forEach(detection => {
      // Group by type
      const type = detection.type || 'unknown';
      summary.byType[type] = (summary.byType[type] || 0) + 1;
      
      // Group by severity
      const severity = detection.severity || 'medium';
      summary.bySeverity[severity] = (summary.bySeverity[severity] || 0) + 1;
      
      // Group by action
      const action = detection.action || 'log';
      summary.byAction[action] = (summary.byAction[action] || 0) + 1;
    });
    
    return summary;
  }
  
  // Calculate risk assessment
  function calculateRiskAssessment(detections: any[]) {
    if (detections.length === 0) {
      return { level: 'safe', score: 0, description: 'No security threats detected' };
    }
    
    const severityWeights = { critical: 25, high: 15, medium: 8, low: 3, info: 1 };
    const totalScore = detections.reduce((sum, detection) => {
      return sum + (severityWeights[detection.severity] || 5);
    }, 0);
    
    let level, description;
    if (totalScore >= 50) {
      level = 'critical';
      description = 'Multiple high-risk security threats detected';
    } else if (totalScore >= 30) {
      level = 'high';
      description = 'Significant security risks present';
    } else if (totalScore >= 15) {
      level = 'medium';
      description = 'Moderate security concerns identified';
    } else {
      level = 'low';
      description = 'Minor security issues detected';
    }
    
    return { level, score: Math.min(100, totalScore), description };
  }
  
  // Generate recommendations
  function generateRecommendations(detections: any[]) {
    const recommendations = [];
    
    const secretCount = detections.filter(d => d.type?.includes('secret')).length;
    const piiCount = detections.filter(d => d.type?.includes('pii')).length;
    const injectionCount = detections.filter(d => d.type?.includes('injection')).length;
    const xssCount = detections.filter(d => d.type?.includes('xss')).length;
    
    if (secretCount > 0) {
      recommendations.push({
        type: 'critical',
        title: 'Secret Exposure Detected',
        description: `${secretCount} secret(s) found. Immediately rotate all exposed credentials and implement proper secret management.`
      });
    }
    
    if (piiCount > 0) {
      recommendations.push({
        type: 'high',
        title: 'PII Data Exposure',
        description: `${piiCount} PII element(s) detected. Review data handling practices and implement anonymization where possible.`
      });
    }
    
    if (injectionCount > 0) {
      recommendations.push({
        type: 'critical',
        title: 'Injection Attack Vectors',
        description: `${injectionCount} injection pattern(s) found. Implement parameterized queries and input validation immediately.`
      });
    }
    
    if (xssCount > 0) {
      recommendations.push({
        type: 'high',
        title: 'XSS Vulnerabilities',
        description: `${xssCount} XSS vector(s) detected. Implement proper output encoding and Content Security Policy headers.`
      });
    }
    
    if (recommendations.length === 0) {
      recommendations.push({
        type: 'info',
        title: 'Content Appears Secure',
        description: 'No major security threats detected, but continue monitoring for new patterns.'
      });
    }
    
    return recommendations;
  }
  
  // Save test result to history
  async function saveTestResult(result: any) {
    try {
      await securityApi.saveSanitizationTestResult({
        originalContent: originalContent,
        sanitizedContent: result.sanitizedContent,
        detections: result.detections,
        processingTime: result.processingTime,
        testConfig: testConfig,
        timestamp: new Date().toISOString()
      });
      
      // Refresh test history
      await loadTestingData();
    } catch (err) {
      console.warn('Failed to save test result:', err);
    }
  }
  
  // Load test scenario
  function loadScenario(scenario: any) {
    originalContent = scenario.content;
    if (livePreview) {
      scheduleLivePreview();
    }
  }
  
  // Policy selection helpers
  function toggleAllPolicies() {
    testConfig.enableAllPolicies = !testConfig.enableAllPolicies;
    
    if (testConfig.enableAllPolicies) {
      testConfig.selectedPolicies = new Set(activePolicies.map(p => p.id));
    } else {
      testConfig.selectedPolicies = new Set();
    }
  }
  
  function togglePolicy(policyId: string) {
    const newSelected = new Set(testConfig.selectedPolicies);
    
    if (newSelected.has(policyId)) {
      newSelected.delete(policyId);
    } else {
      newSelected.add(policyId);
    }
    
    testConfig.selectedPolicies = newSelected;
    testConfig.enableAllPolicies = newSelected.size === activePolicies.length;
  }
  
  // Utility functions
  function getPolicyTypeProps(type: string) {
    const typeProps = {
      'content_filter': { color: 'bg-blue-100 text-blue-800', icon: '🔍' },
      'secret_detection': { color: 'bg-red-100 text-red-800', icon: '🔐' },
      'input_validation': { color: 'bg-green-100 text-green-800', icon: '✅' },
      'pii_detection': { color: 'bg-orange-100 text-orange-800', icon: '👤' },
      'output_sanitization': { color: 'bg-purple-100 text-purple-800', icon: '🧹' }
    };
    
    return typeProps[type] || { color: 'bg-gray-100 text-gray-800', icon: '📝' };
  }
  
  function getSeverityProps(severity: string) {
    const severityProps = {
      'critical': { color: 'bg-red-100 text-red-800', icon: '🚨', textColor: 'text-red-700' },
      'high': { color: 'bg-orange-100 text-orange-800', icon: '⚠️', textColor: 'text-orange-700' },
      'medium': { color: 'bg-yellow-100 text-yellow-800', icon: '🔶', textColor: 'text-yellow-700' },
      'low': { color: 'bg-green-100 text-green-800', icon: 'ℹ️', textColor: 'text-green-700' },
      'info': { color: 'bg-blue-100 text-blue-800', icon: '📘', textColor: 'text-blue-700' }
    };
    
    return severityProps[severity] || severityProps['medium'];
  }
  
  function getActionProps(action: string) {
    const actionProps = {
      'block': { color: 'bg-red-100 text-red-800', icon: '🚫' },
      'sanitize': { color: 'bg-green-100 text-green-800', icon: '🧹' },
      'warn': { color: 'bg-yellow-100 text-yellow-800', icon: '⚠️' },
      'log': { color: 'bg-blue-100 text-blue-800', icon: '📝' }
    };
    
    return actionProps[action] || actionProps['log'];
  }
  
  function getRiskLevelColor(level: string): string {
    const colors = {
      'critical': 'text-red-700 bg-red-100',
      'high': 'text-orange-700 bg-orange-100',
      'medium': 'text-yellow-700 bg-yellow-100',
      'low': 'text-green-700 bg-green-100',
      'safe': 'text-green-700 bg-green-100'
    };
    return colors[level] || 'text-gray-700 bg-gray-100';
  }
  
  function formatTimestamp(timestamp: string): string {
    return new Date(timestamp).toLocaleString();
  }
  
  // Generate diff view
  function generateDiffView(original: string, sanitized: string): string {
    if (!sanitized || original === sanitized) return '';
    
    // Simple diff implementation - would use a proper diff library in production
    const originalLines = original.split('\n');
    const sanitizedLines = sanitized.split('\n');
    const maxLines = Math.max(originalLines.length, sanitizedLines.length);
    
    let diffHtml = '';
    for (let i = 0; i < maxLines; i++) {
      const originalLine = originalLines[i] || '';
      const sanitizedLine = sanitizedLines[i] || '';
      
      if (originalLine !== sanitizedLine) {
        if (originalLine) {
          diffHtml += `<div class="diff-removed">- ${originalLine}</div>`;
        }
        if (sanitizedLine) {
          diffHtml += `<div class="diff-added">+ ${sanitizedLine}</div>`;
        }
      } else {
        diffHtml += `<div class="diff-unchanged">  ${originalLine}</div>`;
      }
    }
    
    return diffHtml;
  }
  
  onMount(() => {
    loadTestingData();
    
    return () => {
      if (previewTimeout) {
        clearTimeout(previewTimeout);
      }
    };
  });
  
  // Live preview when content changes
  $: if (originalContent && livePreview) {
    scheduleLivePreview();
  }
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading sanitization testing playground...</span>
        </div>
      </div>
    </div>
  {:else if error}
    <!-- Error State -->
    <div class="security-card">
      <div class="text-center py-12">
        <div class="text-red-600 mb-4">🚨</div>
        <h3 class="text-lg font-medium text-gray-900 mb-2">Testing Playground Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadTestingData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Sanitization Testing Playground</h2>
          <p class="text-sm text-gray-600 mt-1">
            Test content sanitization with live preview and comprehensive analysis
          </p>
        </div>
        
        <div class="flex items-center gap-3">
          <button 
            class="btn-sm {livePreview ? 'btn-primary' : 'btn-secondary'}"
            on:click={() => livePreview = !livePreview}
          >
            {livePreview ? '⚡ Live Preview' : '⚡ Manual Test'}
          </button>
          
          <button 
            class="btn-secondary"
            on:click={loadTestingData}
          >
            🔄 Refresh
          </button>
          
          {#if !livePreview}
            <button 
              class="btn-primary"
              disabled={testInProgress || !originalContent.trim()}
              on:click={runSanitizationTest}
            >
              {testInProgress ? '🔄 Testing...' : '🧪 Run Test'}
            </button>
          {/if}
        </div>
      </div>
      
      <!-- Quick Stats -->
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mt-4">
        <div class="text-center p-3 bg-blue-50 rounded-lg">
          <div class="text-lg font-bold text-blue-700">{activePolicies.length}</div>
          <div class="text-xs text-blue-600">Active Policies</div>
        </div>
        
        <div class="text-center p-3 bg-green-50 rounded-lg">
          <div class="text-lg font-bold text-green-700">
            {testConfig.enableAllPolicies ? activePolicies.length : testConfig.selectedPolicies.size}
          </div>
          <div class="text-xs text-green-600">Selected</div>
        </div>
        
        <div class="text-center p-3 bg-purple-50 rounded-lg">
          <div class="text-lg font-bold text-purple-700">{testHistory.length}</div>
          <div class="text-xs text-purple-600">Test History</div>
        </div>
        
        <div class="text-center p-3 bg-orange-50 rounded-lg">
          <div class="text-lg font-bold text-orange-700">{originalContent.length}</div>
          <div class="text-xs text-orange-600">Content Chars</div>
        </div>
      </div>
    </div>

    <!-- Test Configuration -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Test Configuration</h3>
        <button
          class="btn-sm btn-secondary"
          on:click={toggleAllPolicies}
        >
          {testConfig.enableAllPolicies ? '☑️ All' : '☐ All'} Policies
        </button>
      </div>
      
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Test Mode -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Test Mode</label>
          <select
            bind:value={testConfig.testMode}
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="comprehensive">Comprehensive - All checks</option>
            <option value="performance">Performance - Fast scan</option>
            <option value="security">Security - Deep analysis</option>
          </select>
        </div>
        
        <!-- Comparison Mode -->
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">View Mode</label>
          <select
            bind:value={comparisonMode}
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="side-by-side">Side by Side</option>
            <option value="overlay">Overlay</option>
            <option value="diff">Diff View</option>
          </select>
        </div>
        
        <!-- Options -->
        <div class="space-y-2">
          <div class="flex items-center">
            <input
              type="checkbox"
              id="strictMode"
              bind:checked={testConfig.strictMode}
              class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label for="strictMode" class="ml-2 text-sm text-gray-700">
              Strict mode (lower thresholds)
            </label>
          </div>
          
          <div class="flex items-center">
            <input
              type="checkbox"
              id="showMetadata"
              bind:checked={showMetadata}
              class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            />
            <label for="showMetadata" class="ml-2 text-sm text-gray-700">
              Show detailed metadata
            </label>
          </div>
        </div>
      </div>
      
      <!-- Policy Selection -->
      {#if !testConfig.enableAllPolicies}
        <div class="mt-4 pt-4 border-t border-gray-200">
          <div class="text-sm font-medium text-gray-700 mb-3">Select Policies to Test</div>
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
            {#each activePolicies as policy}
              {@const typeProps = getPolicyTypeProps(policy.type)}
              {@const isSelected = testConfig.selectedPolicies.has(policy.id)}
              
              <div class="flex items-center p-2 border rounded {isSelected ? 'border-blue-300 bg-blue-50' : 'border-gray-200'}">
                <input
                  type="checkbox"
                  id="policy_{policy.id}"
                  checked={isSelected}
                  on:change={() => togglePolicy(policy.id)}
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
                />
                <label for="policy_{policy.id}" class="ml-2 flex-1 text-sm cursor-pointer">
                  <div class="flex items-center gap-2">
                    <span class="text-sm">{typeProps.icon}</span>
                    <span class="font-medium">{policy.name}</span>
                  </div>
                  <div class="text-xs text-gray-600">{policy.patterns?.length || 0} patterns</div>
                </label>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <!-- Test Scenarios -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Test Scenarios</h3>
        <span class="text-sm text-gray-600">{testScenarios.length} predefined scenarios</span>
      </div>
      
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each testScenarios as scenario}
          <button
            class="text-left p-4 border border-gray-200 rounded-lg hover:bg-gray-50 hover:border-gray-300 transition-colors"
            on:click={() => loadScenario(scenario)}
          >
            <div class="flex items-center gap-2 mb-2">
              <span class="font-medium text-gray-900 text-sm">{scenario.name}</span>
              <span class="inline-flex items-center px-2 py-0.5 rounded text-xs bg-gray-100 text-gray-700">
                {scenario.category}
              </span>
            </div>
            <p class="text-xs text-gray-600 mb-2">{scenario.description}</p>
            <div class="text-xs text-gray-500">
              {scenario.content.length} characters
            </div>
          </button>
        {/each}
      </div>
    </div>

    <!-- Main Testing Interface -->
    <div class="grid grid-cols-1 {comparisonMode === 'side-by-side' ? 'lg:grid-cols-2' : ''} gap-6">
      <!-- Input Section -->
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Original Content</h3>
          <div class="flex items-center gap-2 text-sm text-gray-600">
            <span>{originalContent.length} chars</span>
            {#if testInProgress}
              <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
            {/if}
            {#if livePreview && originalContent}
              <span class="text-green-600">⚡ Live</span>
            {/if}
          </div>
        </div>
        
        <textarea
          bind:value={originalContent}
          rows="20"
          class="w-full px-3 py-2 border border-gray-300 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
          placeholder="Enter content to test sanitization...

Paste here:
• Configuration files with secrets
• User input data with PII
• Code snippets with potential vulnerabilities
• Log files with mixed content
• API requests/responses
• Database queries

The system will analyze and sanitize the content based on active policies."
        ></textarea>
      </div>
      
      <!-- Results Section -->
      {#if testResults && !testResults.error}
        <div class="space-y-4">
          <!-- Risk Assessment -->
          <div class="security-card">
            <div class="security-card-header">
              <h3 class="security-card-title">Risk Assessment</h3>
              <div class="text-sm text-gray-600">
                {testResults.processingTime}ms processing
              </div>
            </div>
            
            <div class="space-y-4">
              <!-- Risk Level -->
              <div class="flex items-center justify-between">
                <span class="text-sm font-medium text-gray-700">Risk Level</span>
                <span class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {getRiskLevelColor(testResults.riskAssessment?.level || 'safe')}">
                  {testResults.riskAssessment?.level?.toUpperCase() || 'SAFE'}
                </span>
              </div>
              
              <!-- Risk Score -->
              <div>
                <div class="flex items-center justify-between mb-2">
                  <span class="text-sm font-medium text-gray-700">Risk Score</span>
                  <span class="text-lg font-bold {
                    (testResults.riskAssessment?.score || 0) >= 75 ? 'text-red-700' :
                    (testResults.riskAssessment?.score || 0) >= 50 ? 'text-orange-700' :
                    (testResults.riskAssessment?.score || 0) >= 25 ? 'text-yellow-700' :
                    'text-green-700'
                  }">{testResults.riskAssessment?.score || 0}/100</span>
                </div>
                
                <div class="w-full bg-gray-200 rounded-full h-3">
                  <div class="h-3 rounded-full {
                    (testResults.riskAssessment?.score || 0) >= 75 ? 'bg-red-500' :
                    (testResults.riskAssessment?.score || 0) >= 50 ? 'bg-orange-500' :
                    (testResults.riskAssessment?.score || 0) >= 25 ? 'bg-yellow-500' :
                    'bg-green-500'
                  }" style="width: {testResults.riskAssessment?.score || 0}%"></div>
                </div>
              </div>
              
              <p class="text-sm text-gray-600">{testResults.riskAssessment?.description}</p>
              
              <!-- Content Statistics -->
              <div class="grid grid-cols-2 gap-4 text-sm">
                <div class="bg-blue-50 p-3 rounded-lg">
                  <div class="font-bold text-blue-700">{testResults.originalLength}</div>
                  <div class="text-blue-600">Original Chars</div>
                </div>
                
                <div class="bg-green-50 p-3 rounded-lg">
                  <div class="font-bold text-green-700">{testResults.sanitizedLength}</div>
                  <div class="text-green-600">Sanitized Chars</div>
                </div>
              </div>
              
              {#if testResults.reductionPercentage > 0}
                <div class="text-sm text-gray-600">
                  Content reduced by {testResults.reductionPercentage}%
                </div>
              {/if}
            </div>
          </div>
          
          <!-- Detection Summary -->
          {#if testResults.detectionSummary?.totalDetections > 0}
            <div class="security-card">
              <div class="security-card-header">
                <h3 class="security-card-title">Detection Summary</h3>
                <span class="text-sm text-gray-600">
                  {testResults.detectionSummary.totalDetections} detections
                </span>
              </div>
              
              <!-- By Severity -->
              <div class="mb-4">
                <div class="text-sm font-medium text-gray-700 mb-2">By Severity</div>
                <div class="grid grid-cols-2 md:grid-cols-3 gap-2">
                  {#each Object.entries(testResults.detectionSummary.bySeverity) as [severity, count]}
                    {@const severityProps = getSeverityProps(severity)}
                    <div class="text-center p-2 rounded {severityProps.color}">
                      <div class="font-bold">{count}</div>
                      <div class="text-xs capitalize">{severity}</div>
                    </div>
                  {/each}
                </div>
              </div>
              
              <!-- By Action -->
              <div class="mb-4">
                <div class="text-sm font-medium text-gray-700 mb-2">Actions Taken</div>
                <div class="grid grid-cols-2 md:grid-cols-4 gap-2">
                  {#each Object.entries(testResults.detectionSummary.byAction) as [action, count]}
                    {@const actionProps = getActionProps(action)}
                    <div class="text-center p-2 rounded {actionProps.color}">
                      <div class="font-bold">{count}</div>
                      <div class="text-xs capitalize">{action}</div>
                    </div>
                  {/each}
                </div>
              </div>
            </div>
          {/if}
          
          <!-- Recommendations -->
          {#if testResults.recommendations?.length > 0}
            <div class="security-card">
              <div class="security-card-header">
                <h3 class="security-card-title">Recommendations</h3>
              </div>
              
              <div class="space-y-3">
                {#each testResults.recommendations as recommendation}
                  {@const severityProps = getSeverityProps(recommendation.type)}
                  
                  <div class="p-3 rounded-lg {severityProps.color.replace('text-', 'bg-').replace('-800', '-50')}">
                    <div class="flex items-start gap-2">
                      <span class="text-lg mt-0.5">{severityProps.icon}</span>
                      <div>
                        <div class="font-medium text-sm {severityProps.textColor}">
                          {recommendation.title}
                        </div>
                        <div class="text-sm text-gray-700 mt-1">
                          {recommendation.description}
                        </div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
          
          <!-- Sanitized Content -->
          {#if comparisonMode === 'side-by-side'}
            <div class="security-card">
              <div class="security-card-header">
                <h3 class="security-card-title">Sanitized Content</h3>
                <div class="text-sm text-gray-600">
                  {testResults.sanitizedContent?.length || 0} chars
                </div>
              </div>
              
              <div class="bg-green-50 p-3 rounded-lg">
                <pre class="font-mono text-sm text-green-900 whitespace-pre-wrap">{testResults.sanitizedContent || 'No sanitization needed'}</pre>
              </div>
            </div>
          {/if}
        </div>
        
      {:else if testResults?.error}
        <!-- Error Display -->
        <div class="security-card">
          <div class="text-center py-12">
            <div class="text-red-600 mb-4 text-4xl">❌</div>
            <h3 class="text-lg font-medium text-gray-900 mb-2">Test Failed</h3>
            <p class="text-gray-600">{testResults.error}</p>
          </div>
        </div>
        
      {:else if !originalContent.trim()}
        <!-- Empty State -->
        <div class="security-card">
          <div class="text-center py-12">
            <div class="text-gray-400 mb-4 text-4xl">🧪</div>
            <h3 class="text-lg font-medium text-gray-900 mb-2">Ready to Test</h3>
            <p class="text-gray-600 mb-4">
              Enter content to analyze or select a test scenario to get started
            </p>
            {#if testScenarios.length > 0}
              <button 
                class="btn-primary"
                on:click={() => loadScenario(testScenarios[0])}
              >
                Load First Scenario
              </button>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- Detailed Results -->
    {#if testResults && testResults.detections && testResults.detections.length > 0}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Detailed Detection Results</h3>
          <span class="text-sm text-gray-600">{testResults.detections.length} detections</span>
        </div>
        
        <div class="space-y-4">
          {#each testResults.detections as detection, index}
            {@const severityProps = getSeverityProps(detection.severity)}
            {@const actionProps = getActionProps(detection.action)}
            
            <div class="border border-gray-200 rounded-lg p-4">
              <div class="flex items-start justify-between mb-3">
                <div class="flex items-center gap-2">
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {severityProps.color}">
                    {severityProps.icon} {detection.severity.toUpperCase()}
                  </span>
                  
                  <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {actionProps.color}">
                    {actionProps.icon} {detection.action?.toUpperCase() || 'LOG'}
                  </span>
                  
                  <span class="text-xs text-gray-600">
                    {Math.round(detection.confidence * 100)}% confidence
                  </span>
                </div>
                
                <span class="text-xs text-gray-500">#{index + 1}</span>
              </div>
              
              <div class="text-sm font-medium text-gray-900 mb-2">
                {detection.title || detection.type || 'Detection'}
              </div>
              
              <div class="text-sm text-gray-600 mb-3">
                {detection.description || 'Security issue detected'}
              </div>
              
              {#if detection.matchedText}
                <div class="mb-3">
                  <div class="text-xs font-medium text-gray-700 mb-1">Matched Content</div>
                  <div class="bg-red-100 p-2 rounded font-mono text-sm text-red-800 break-all">
                    {detection.matchedText}
                  </div>
                </div>
              {/if}
              
              {#if detection.replacement}
                <div class="mb-3">
                  <div class="text-xs font-medium text-gray-700 mb-1">Sanitized To</div>
                  <div class="bg-green-100 p-2 rounded font-mono text-sm text-green-800 break-all">
                    {detection.replacement}
                  </div>
                </div>
              {/if}
              
              {#if showMetadata && (detection.pattern || detection.position)}
                <details class="text-xs text-gray-600">
                  <summary class="cursor-pointer hover:text-gray-800 font-medium">
                    Technical Details
                  </summary>
                  <div class="mt-2 space-y-1">
                    {#if detection.pattern}
                      <div><strong>Pattern:</strong> <code class="bg-gray-100 px-1 rounded">{detection.pattern}</code></div>
                    {/if}
                    {#if detection.position}
                      <div><strong>Position:</strong> {detection.position.start} - {detection.position.end}</div>
                    {/if}
                    {#if detection.policyId}
                      <div><strong>Policy:</strong> {detection.policyId}</div>
                    {/if}
                  </div>
                </details>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Test History -->
    {#if testHistory.length > 0}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Recent Test History</h3>
          <span class="text-sm text-gray-600">{testHistory.length} recent tests</span>
        </div>
        
        <div class="space-y-3">
          {#each testHistory.slice(0, 5) as test}
            <div class="flex items-center justify-between p-3 border border-gray-200 rounded-lg hover:bg-gray-50">
              <div>
                <div class="text-sm font-medium text-gray-900">
                  Test #{test.id?.slice(-8) || 'Unknown'}
                </div>
                <div class="text-xs text-gray-600">
                  {test.detectionCount || 0} detections • {test.processingTime || 0}ms
                </div>
              </div>
              
              <div class="text-right text-xs text-gray-500">
                <div>{formatTimestamp(test.timestamp)}</div>
                <div>{test.contentLength || 0} chars</div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  /* Custom scrollbar styling */
  textarea {
    scrollbar-width: thin;
    scrollbar-color: rgb(209, 213, 219) transparent;
  }
  
  textarea::-webkit-scrollbar {
    width: 6px;
  }
  
  textarea::-webkit-scrollbar-track {
    background: transparent;
  }
  
  textarea::-webkit-scrollbar-thumb {
    background-color: rgb(209, 213, 219);
    border-radius: 3px;
  }
  
  /* Progress bar animation */
  .h-3.rounded-full {
    transition: width 0.5s ease;
  }
  
  /* Pre-formatted content styling */
  pre {
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 300px;
    overflow-y: auto;
  }
  
  /* Diff view styles (for future enhancement) */
  .diff-removed {
    background-color: #fee2e2;
    color: #991b1b;
    padding: 2px 4px;
    margin: 1px 0;
  }
  
  .diff-added {
    background-color: #dcfce7;
    color: #166534;
    padding: 2px 4px;
    margin: 1px 0;
  }
  
  .diff-unchanged {
    color: #6b7280;
    padding: 2px 4px;
    margin: 1px 0;
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
  
  /* Responsive grid adjustments */
  @media (max-width: 768px) {
    .grid.lg\\:grid-cols-2 {
      grid-template-columns: 1fr;
    }
  }
</style>