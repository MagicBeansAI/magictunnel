<script lang="ts">
  import { onMount } from 'svelte';
  import { securityApi } from '$lib/api/security';
  import type { SecretDetectionRule, SecretDetectionResult, SanitizationPolicy } from '$lib/types/security';
  
  // State management
  let secretRules: SecretDetectionRule[] = [];
  let detectionResults: SecretDetectionResult[] = [];
  let secretPolicies: SanitizationPolicy[] = [];
  let loading = true;
  let error = '';
  
  // Secret scanning
  let scanContent = '';
  let scanResults: any = null;
  let scanInProgress = false;
  let autoScan = true;
  let scanTimeout: number | null = null;
  
  // Pattern management
  let showPatternEditor = false;
  let editingPattern: SecretDetectionRule | null = null;
  let newPattern = {
    name: '',
    description: '',
    pattern: '',
    secretType: 'api_key' as any,
    severity: 'high' as any,
    active: true,
    confidence_threshold: 0.8
  };
  
  // Secret type configurations
  const secretTypes = [
    {
      type: 'api_key',
      name: 'API Keys',
      icon: '🔑',
      color: 'bg-blue-100 text-blue-800',
      patterns: [
        { name: 'Generic API Key', pattern: '(?i)api[_-]?key[\\s]*[:=][\\s]*[\'"]?([a-zA-Z0-9]{20,})' },
        { name: 'Bearer Token', pattern: 'Bearer\\s+([A-Za-z0-9\\-_=]+)' },
        { name: 'API Secret', pattern: '(?i)api[_-]?secret[\\s]*[:=][\\s]*[\'"]?([a-zA-Z0-9]{20,})' }
      ]
    },
    {
      type: 'oauth_token',
      name: 'OAuth Tokens',
      icon: '🎫',
      color: 'bg-purple-100 text-purple-800',
      patterns: [
        { name: 'OAuth Access Token', pattern: 'access_token[\\s]*[:=][\\s]*[\'"]?([a-zA-Z0-9\\-_\\.]+)' },
        { name: 'OAuth Refresh Token', pattern: 'refresh_token[\\s]*[:=][\\s]*[\'"]?([a-zA-Z0-9\\-_\\.]+)' }
      ]
    },
    {
      type: 'database_credential',
      name: 'Database Credentials',
      icon: '🗄️',
      color: 'bg-red-100 text-red-800',
      patterns: [
        { name: 'Database Password', pattern: '(?i)(db|database)[_-]?(password|pwd)[\\s]*[:=][\\s]*[\'"]?([^\\s\\n\'"]+)' },
        { name: 'MongoDB URI', pattern: 'mongodb://[^\\s]+:[^\\s]+@[^\\s]+' },
        { name: 'PostgreSQL URI', pattern: 'postgres://[^\\s]+:[^\\s]+@[^\\s]+' }
      ]
    },
    {
      type: 'cloud_key',
      name: 'Cloud Service Keys',
      icon: '☁️',
      color: 'bg-cyan-100 text-cyan-800',
      patterns: [
        { name: 'AWS Access Key', pattern: 'AKIA[0-9A-Z]{16}' },
        { name: 'AWS Secret Key', pattern: '[A-Za-z0-9/+=]{40}' },
        { name: 'Google Cloud Key', pattern: 'AIza[0-9A-Za-z\\-_]{35}' },
        { name: 'Azure Key', pattern: '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' }
      ]
    },
    {
      type: 'private_key',
      name: 'Private Keys',
      icon: '🔐',
      color: 'bg-red-100 text-red-800',
      patterns: [
        { name: 'RSA Private Key', pattern: '-----BEGIN RSA PRIVATE KEY-----[\\s\\S]*?-----END RSA PRIVATE KEY-----' },
        { name: 'SSH Private Key', pattern: '-----BEGIN OPENSSH PRIVATE KEY-----[\\s\\S]*?-----END OPENSSH PRIVATE KEY-----' },
        { name: 'EC Private Key', pattern: '-----BEGIN EC PRIVATE KEY-----[\\s\\S]*?-----END EC PRIVATE KEY-----' }
      ]
    },
    {
      type: 'service_token',
      name: 'Service Tokens',
      icon: '🎯',
      color: 'bg-green-100 text-green-800',
      patterns: [
        { name: 'GitHub Token', pattern: 'ghp_[a-zA-Z0-9]{36}' },
        { name: 'GitLab Token', pattern: 'glpat-[a-zA-Z0-9\\-_]{20}' },
        { name: 'Slack Token', pattern: 'xox[baprs]-[a-zA-Z0-9\\-]+' },
        { name: 'Discord Token', pattern: '[MN][A-Za-z\\d]{23}\\.[\\w-]{6}\\.[\\w-]{27}' }
      ]
    },
    {
      type: 'cryptocurrency',
      name: 'Cryptocurrency',
      icon: '₿',
      color: 'bg-orange-100 text-orange-800',
      patterns: [
        { name: 'Bitcoin Address', pattern: '[13][a-km-zA-HJ-NP-Z1-9]{25,34}' },
        { name: 'Ethereum Address', pattern: '0x[a-fA-F0-9]{40}' },
        { name: 'Bitcoin Private Key', pattern: '[5KL][1-9A-HJ-NP-Za-km-z]{50,51}' }
      ]
    },
    {
      type: 'payment',
      name: 'Payment Info',
      icon: '💳',
      color: 'bg-yellow-100 text-yellow-800',
      patterns: [
        { name: 'Credit Card Number', pattern: '\\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|3[0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\\b' },
        { name: 'Stripe Secret Key', pattern: 'sk_live_[a-zA-Z0-9]{24}' },
        { name: 'PayPal Client Secret', pattern: 'paypal.*[\'"][a-zA-Z0-9_-]{32,}[\'"]' }
      ]
    }
  ];
  
  // Test content templates
  const secretTestTemplates = [
    {
      name: 'API Keys Mix',
      content: `const config = {
  apiKey: "sk-1234567890abcdefghijklmnopqrstuvwxyzABCDEF",
  secret: "api_secret_abc123def456ghi789jkl012mno345",
  bearerToken: "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"
};`,
      type: 'api_key'
    },
    {
      name: 'Database Credentials',
      content: `DATABASE_URL=postgres://user:secretpass123@localhost:5432/mydb
MONGO_URI=mongodb://admin:password456@cluster.mongodb.net/database
db_password = "supersecret789"`,
      type: 'database_credential'
    },
    {
      name: 'Cloud Service Keys',
      content: `AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
GOOGLE_API_KEY=AIzaSyDaGmWKa4JsXZ-HjGw12345678901234567890`,
      type: 'cloud_key'
    },
    {
      name: 'Private Keys',
      content: `-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA1234567890abcdefghijklmnopqrstuvwxyz
ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcdefghijklm
-----END RSA PRIVATE KEY-----`,
      type: 'private_key'
    },
    {
      name: 'Service Tokens',
      content: `GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyz
SLACK_TOKEN=xoxb-1234567890-1234567890-abc123def456
DISCORD_BOT_TOKEN=MTA1234567890.ABCDEF.1234567890abcdefghij`,
      type: 'service_token'
    },
    {
      name: 'Mixed Secrets',
      content: `# Configuration file with various secrets
API_KEY="sk-1234567890abcdefghijklmnopqrstuvwxyzABCDEF"
DATABASE_URL="postgres://user:pass@localhost:5432/db"
STRIPE_SECRET_KEY="sk_live_1234567890abcdefghijklmnop"
JWT_SECRET="jwt_secret_key_very_long_string_here"
GITHUB_TOKEN="ghp_abcdefghijklmnopqrstuvwxyz123456"
AWS_ACCESS_KEY_ID="AKIAIOSFODNN7EXAMPLE"
BITCOIN_ADDRESS="1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"`
    }
  ];
  
  // Load secret detection data
  async function loadSecretDetectionData() {
    try {
      loading = true;
      error = '';
      
      const [rulesData, resultsData, policiesData] = await Promise.all([
        securityApi.getSecretDetectionRules(),
        securityApi.getSecretDetectionResults({ limit: 20, orderBy: 'timestamp', order: 'desc' }),
        securityApi.getSanitizationPolicies({ type: 'secret_detection', limit: 50 })
      ]);
      
      secretRules = rulesData.rules || [];
      detectionResults = resultsData.results || [];
      secretPolicies = policiesData.policies || [];
    } catch (err) {
      console.error('Failed to load secret detection data:', err);
      error = `Failed to load secret detection data: ${err}`;
    } finally {
      loading = false;
    }
  }
  
  // Secret scanning with debouncing
  async function scanForSecrets(content?: string) {
    const scanText = content || scanContent;
    if (!scanText.trim()) {
      scanResults = null;
      return;
    }
    
    scanInProgress = true;
    
    try {
      const result = await securityApi.scanForSecrets({
        content: scanText,
        enabledTypes: secretTypes.map(t => t.type),
        confidenceThreshold: 0.7,
        includeContext: true
      });
      
      // Enhance results with additional analysis
      const enhancedDetections = (result.detections || []).map(detection => ({
        ...detection,
        riskLevel: getRiskLevel(detection),
        recommendation: getRecommendation(detection),
        context: extractContext(scanText, detection.position)
      }));
      
      scanResults = {
        ...result,
        detections: enhancedDetections,
        riskScore: calculateSecretRiskScore(enhancedDetections),
        summary: generateSummary(enhancedDetections)
      };
    } catch (err) {
      console.error('Secret scanning failed:', err);
      scanResults = { error: `Scan failed: ${err}` };
    } finally {
      scanInProgress = false;
    }
  }
  
  // Auto-scan with debouncing
  function scheduleAutoScan() {
    if (!autoScan) return;
    
    if (scanTimeout) {
      clearTimeout(scanTimeout);
    }
    
    scanTimeout = setTimeout(() => {
      scanForSecrets();
    }, 500);
  }
  
  // Risk level calculation
  function getRiskLevel(detection: any): 'critical' | 'high' | 'medium' | 'low' {
    const criticalTypes = ['private_key', 'database_credential'];
    const highTypes = ['api_key', 'cloud_key', 'payment'];
    const mediumTypes = ['oauth_token', 'service_token'];
    
    if (criticalTypes.includes(detection.type)) return 'critical';
    if (highTypes.includes(detection.type)) return 'high';
    if (mediumTypes.includes(detection.type)) return 'medium';
    return 'low';
  }
  
  // Recommendation generation
  function getRecommendation(detection: any): string {
    const recommendations = {
      'api_key': 'Immediately rotate this API key and update all services using it.',
      'oauth_token': 'Revoke and regenerate this OAuth token immediately.',
      'database_credential': 'Change database password and update connection strings.',
      'cloud_key': 'Rotate cloud service credentials and review access policies.',
      'private_key': 'Replace private key immediately and update all dependent systems.',
      'service_token': 'Revoke token in service dashboard and generate new one.',
      'cryptocurrency': 'Move funds to new wallet and never expose private keys.',
      'payment': 'Contact payment provider to invalidate exposed credentials.'
    };
    
    return recommendations[detection.type] || 'Review and secure this credential immediately.';
  }
  
  // Context extraction
  function extractContext(content: string, position: { start: number, end: number }): string {
    const lines = content.split('\n');
    const contentBeforePosition = content.substring(0, position.start);
    const lineIndex = contentBeforePosition.split('\n').length - 1;
    
    const contextStart = Math.max(0, lineIndex - 2);
    const contextEnd = Math.min(lines.length, lineIndex + 3);
    
    return lines.slice(contextStart, contextEnd).join('\n');
  }
  
  // Risk score calculation
  function calculateSecretRiskScore(detections: any[]): number {
    if (!detections || detections.length === 0) return 0;
    
    const riskWeights = { critical: 30, high: 20, medium: 10, low: 5 };
    const totalRisk = detections.reduce((sum, detection) => {
      return sum + (riskWeights[detection.riskLevel] || 5);
    }, 0);
    
    return Math.min(100, totalRisk);
  }
  
  // Summary generation
  function generateSummary(detections: any[]): any {
    const typeGroups = detections.reduce((groups, detection) => {
      const type = detection.type;
      if (!groups[type]) groups[type] = [];
      groups[type].push(detection);
      return groups;
    }, {});
    
    return {
      totalSecrets: detections.length,
      uniqueTypes: Object.keys(typeGroups).length,
      typeBreakdown: typeGroups,
      highRiskCount: detections.filter(d => d.riskLevel === 'critical' || d.riskLevel === 'high').length
    };
  }
  
  // Pattern management
  function openPatternEditor(pattern?: SecretDetectionRule) {
    editingPattern = pattern || null;
    
    if (pattern) {
      newPattern = {
        name: pattern.name,
        description: pattern.description || '',
        pattern: pattern.pattern,
        secretType: pattern.secretType,
        severity: pattern.severity,
        active: pattern.active,
        confidence_threshold: pattern.confidenceThreshold || 0.8
      };
    } else {
      newPattern = {
        name: '',
        description: '',
        pattern: '',
        secretType: 'api_key',
        severity: 'high',
        active: true,
        confidence_threshold: 0.8
      };
    }
    
    showPatternEditor = true;
  }
  
  function closePatternEditor() {
    showPatternEditor = false;
    editingPattern = null;
  }
  
  async function savePattern() {
    try {
      const patternData = {
        ...newPattern,
        confidenceThreshold: newPattern.confidence_threshold
      };
      
      if (editingPattern) {
        await securityApi.updateSecretDetectionRule(editingPattern.id, patternData);
      } else {
        await securityApi.createSecretDetectionRule(patternData);
      }
      
      await loadSecretDetectionData();
      closePatternEditor();
    } catch (err) {
      alert(`Failed to save pattern: ${err}`);
    }
  }
  
  async function deletePattern(patternId: string) {
    if (!confirm('Are you sure you want to delete this pattern?')) return;
    
    try {
      await securityApi.deleteSecretDetectionRule(patternId);
      await loadSecretDetectionData();
    } catch (err) {
      alert(`Failed to delete pattern: ${err}`);
    }
  }
  
  async function togglePatternStatus(pattern: SecretDetectionRule) {
    try {
      await securityApi.updateSecretDetectionRule(pattern.id, {
        active: !pattern.active
      });
      await loadSecretDetectionData();
    } catch (err) {
      alert(`Failed to update pattern status: ${err}`);
    }
  }
  
  // Template loading
  function loadTestTemplate(template: any) {
    scanContent = template.content;
    if (autoScan) {
      scheduleAutoScan();
    }
  }
  
  // Utility functions
  function getSecretTypeProps(type: string) {
    const secretType = secretTypes.find(t => t.type === type);
    return secretType || { icon: '🔍', color: 'bg-gray-100 text-gray-800', name: type };
  }
  
  function getSeverityProps(severity: string) {
    const severityProps = {
      'critical': { color: 'bg-red-100 text-red-800', icon: '🚨', bgColor: 'bg-red-50' },
      'high': { color: 'bg-orange-100 text-orange-800', icon: '⚠️', bgColor: 'bg-orange-50' },
      'medium': { color: 'bg-yellow-100 text-yellow-800', icon: '🔶', bgColor: 'bg-yellow-50' },
      'low': { color: 'bg-green-100 text-green-800', icon: 'ℹ️', bgColor: 'bg-green-50' }
    };
    
    return severityProps[severity] || severityProps['medium'];
  }
  
  function getRiskLevelColor(riskLevel: string): string {
    const colors = {
      'critical': 'text-red-700 bg-red-100',
      'high': 'text-orange-700 bg-orange-100',
      'medium': 'text-yellow-700 bg-yellow-100',
      'low': 'text-green-700 bg-green-100'
    };
    return colors[riskLevel] || 'text-gray-700 bg-gray-100';
  }
  
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
    loadSecretDetectionData();
    
    return () => {
      if (scanTimeout) {
        clearTimeout(scanTimeout);
      }
    };
  });
  
  // Auto-scan when content changes
  $: if (scanContent && autoScan) {
    scheduleAutoScan();
  }
</script>

<div class="space-y-6">
  <!-- Loading State -->
  {#if loading}
    <div class="security-card">
      <div class="flex items-center justify-center py-12">
        <div class="flex items-center gap-3 text-gray-600">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
          <span>Loading secret detection system...</span>
        </div>
      </div>
    </div>
  {:else if error}
    <!-- Error State -->
    <div class="security-card">
      <div class="text-center py-12">
        <div class="text-red-600 mb-4">🚨</div>
        <h3 class="text-lg font-medium text-gray-900 mb-2">Secret Detection Error</h3>
        <p class="text-gray-600 mb-4">{error}</p>
        <button class="btn-primary" on:click={loadSecretDetectionData}>
          🔄 Retry
        </button>
      </div>
    </div>
  {:else}
    <!-- Header Section -->
    <div class="security-card">
      <div class="security-card-header">
        <div>
          <h2 class="security-card-title">Secret Detection & Pattern Management</h2>
          <p class="text-sm text-gray-600 mt-1">
            Scan content for exposed secrets, credentials, and sensitive information
          </p>
        </div>
        
        <div class="flex items-center gap-3">
          <button 
            class="btn-sm {autoScan ? 'btn-primary' : 'btn-secondary'}"
            on:click={() => autoScan = !autoScan}
          >
            {autoScan ? '⚡ Auto-Scan On' : '⚡ Auto-Scan Off'}
          </button>
          
          <button 
            class="btn-secondary"
            on:click={() => openPatternEditor()}
          >
            ➕ Add Pattern
          </button>
          
          <button 
            class="btn-primary"
            on:click={loadSecretDetectionData}
          >
            🔄 Refresh
          </button>
        </div>
      </div>
      
      <!-- Statistics -->
      <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-8 gap-3 mt-4">
        {#each secretTypes as secretType}
          {@const count = secretRules.filter(r => r.secretType === secretType.type).length}
          <div class="text-center p-2 rounded-lg {secretType.color}">
            <div class="text-lg">{secretType.icon}</div>
            <div class="text-sm font-bold">{count}</div>
            <div class="text-xs">{secretType.name.split(' ')[0]}</div>
          </div>
        {/each}
      </div>
    </div>

    <!-- Main Content -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Scanning Interface -->
      <div class="space-y-4">
        <!-- Scan Input -->
        <div class="security-card">
          <div class="security-card-header">
            <h3 class="security-card-title">Secret Scanner</h3>
            <div class="flex items-center gap-2 text-sm">
              <span class="text-gray-600">{scanContent.length} chars</span>
              {#if scanInProgress}
                <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
              {/if}
            </div>
          </div>
          
          <textarea
            bind:value={scanContent}
            rows="14"
            class="w-full px-3 py-2 border border-gray-300 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
            placeholder="Paste content to scan for secrets...

Examples:
• Configuration files (.env, config.json)
• Source code with embedded credentials
• API responses or logs
• Documentation with example keys
• Infrastructure as Code templates"
          ></textarea>
          
          <div class="flex items-center justify-between mt-3">
            <div class="flex items-center gap-2">
              <input
                type="checkbox"
                id="autoScan"
                bind:checked={autoScan}
                class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label for="autoScan" class="text-sm text-gray-700">Auto-scan on change</label>
            </div>
            
            <button
              class="btn-primary"
              disabled={scanInProgress || !scanContent.trim()}
              on:click={() => scanForSecrets()}
            >
              {scanInProgress ? '🔄 Scanning...' : '🔍 Scan Now'}
            </button>
          </div>
        </div>
        
        <!-- Test Templates -->
        <div class="security-card">
          <div class="security-card-header">
            <h3 class="security-card-title">Test Templates</h3>
          </div>
          
          <div class="space-y-2">
            {#each secretTestTemplates as template}
              {@const typeProps = getSecretTypeProps(template.type)}
              <button
                class="w-full text-left p-3 border border-gray-200 rounded-lg hover:bg-gray-50 hover:border-gray-300 transition-colors"
                on:click={() => loadTestTemplate(template)}
              >
                <div class="flex items-center gap-2 mb-1">
                  <span class="text-lg">{typeProps.icon}</span>
                  <span class="font-medium text-gray-900 text-sm">{template.name}</span>
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs {typeProps.color}">
                    {typeProps.name}
                  </span>
                </div>
                <div class="text-xs text-gray-600 font-mono">
                  {template.content.substring(0, 80)}...
                </div>
              </button>
            {/each}
          </div>
        </div>
      </div>
      
      <!-- Scan Results -->
      <div class="space-y-4">
        {#if scanResults}
          <div class="security-card">
            <div class="security-card-header">
              <h3 class="security-card-title">Scan Results</h3>
              <div class="text-sm text-gray-600">
                {#if scanResults.error}
                  ❌ Error
                {:else}
                  {scanResults.processingTime || 0}ms • {scanResults.summary?.totalSecrets || 0} secrets
                {/if}
              </div>
            </div>
            
            {#if scanResults.error}
              <div class="text-sm text-red-600 bg-red-50 p-3 rounded-lg">
                {scanResults.error}
              </div>
            {:else}
              <!-- Risk Score -->
              <div class="mb-4">
                <div class="flex items-center justify-between mb-2">
                  <span class="text-sm font-medium text-gray-700">Risk Score</span>
                  <span class="text-lg font-bold {
                    scanResults.riskScore >= 80 ? 'text-red-700' :
                    scanResults.riskScore >= 60 ? 'text-orange-700' :
                    scanResults.riskScore >= 30 ? 'text-yellow-700' :
                    'text-green-700'
                  }">{scanResults.riskScore}/100</span>
                </div>
                
                <div class="w-full bg-gray-200 rounded-full h-3">
                  <div class="h-3 rounded-full {
                    scanResults.riskScore >= 80 ? 'bg-red-500' :
                    scanResults.riskScore >= 60 ? 'bg-orange-500' :
                    scanResults.riskScore >= 30 ? 'bg-yellow-500' :
                    'bg-green-500'
                  }" style="width: {scanResults.riskScore}%"></div>
                </div>
              </div>
              
              <!-- Summary -->
              {#if scanResults.summary}
                <div class="grid grid-cols-2 gap-4 mb-4 text-sm">
                  <div class="bg-blue-50 p-3 rounded-lg">
                    <div class="font-bold text-blue-700">{scanResults.summary.totalSecrets}</div>
                    <div class="text-blue-600">Total Secrets</div>
                  </div>
                  
                  <div class="bg-red-50 p-3 rounded-lg">
                    <div class="font-bold text-red-700">{scanResults.summary.highRiskCount}</div>
                    <div class="text-red-600">High Risk</div>
                  </div>
                </div>
              {/if}
              
              <!-- Detections -->
              {#if scanResults.detections && scanResults.detections.length > 0}
                <div class="space-y-4">
                  <div class="text-sm font-medium text-gray-700">
                    Secret Detections ({scanResults.detections.length})
                  </div>
                  
                  {#each scanResults.detections as detection}
                    {@const typeProps = getSecretTypeProps(detection.type)}
                    {@const severityProps = getSeverityProps(detection.severity)}
                    {@const riskColor = getRiskLevelColor(detection.riskLevel)}
                    
                    <div class="border border-gray-200 rounded-lg p-4 {severityProps.bgColor}">
                      <div class="flex items-start justify-between mb-3">
                        <div class="flex items-center gap-2">
                          <span class="text-xl">{typeProps.icon}</span>
                          <div>
                            <div class="font-medium text-gray-900">{detection.name || typeProps.name}</div>
                            <div class="text-xs text-gray-600">{detection.description}</div>
                          </div>
                        </div>
                        
                        <div class="flex items-center gap-2">
                          <span class="inline-flex items-center px-2 py-1 rounded-full text-xs font-medium {riskColor}">
                            {detection.riskLevel.toUpperCase()}
                          </span>
                          <span class="text-xs text-gray-600">
                            {Math.round(detection.confidence * 100)}%
                          </span>
                        </div>
                      </div>
                      
                      <!-- Detected Secret -->
                      <div class="mb-3">
                        <div class="text-xs font-medium text-gray-700 mb-1">Detected Secret</div>
                        <div class="bg-red-100 p-2 rounded font-mono text-sm text-red-800 break-all">
                          {detection.value || detection.matchedText || 'Secret value detected'}
                        </div>
                      </div>
                      
                      <!-- Context -->
                      {#if detection.context}
                        <details class="mb-3">
                          <summary class="cursor-pointer text-xs font-medium text-gray-700 hover:text-gray-900">
                            View context
                          </summary>
                          <div class="mt-2 bg-gray-100 p-2 rounded">
                            <pre class="text-xs text-gray-700 whitespace-pre-wrap">{detection.context}</pre>
                          </div>
                        </details>
                      {/if}
                      
                      <!-- Recommendation -->
                      <div class="bg-yellow-50 p-3 rounded">
                        <div class="text-xs font-medium text-yellow-800 mb-1">🔧 Recommendation</div>
                        <div class="text-xs text-yellow-700">{detection.recommendation}</div>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-center py-8 text-green-600">
                  <div class="text-4xl mb-2">✅</div>
                  <div class="text-sm font-medium">No Secrets Detected</div>
                  <div class="text-xs text-gray-600 mt-1">Content appears to be clean</div>
                </div>
              {/if}
            {/if}
          </div>
        {:else}
          <div class="security-card">
            <div class="text-center py-12">
              <div class="text-gray-400 mb-4 text-4xl">🔍</div>
              <h3 class="text-lg font-medium text-gray-900 mb-2">Ready to Scan</h3>
              <p class="text-gray-600">Enter content above or load a test template to begin scanning</p>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <!-- Detection Patterns -->
    <div class="security-card">
      <div class="security-card-header">
        <h3 class="security-card-title">Detection Patterns</h3>
        <button 
          class="btn-secondary"
          on:click={() => openPatternEditor()}
        >
          ➕ Add Pattern
        </button>
      </div>
      
      {#if secretRules.length > 0}
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {#each secretRules as rule}
            {@const typeProps = getSecretTypeProps(rule.secretType)}
            {@const severityProps = getSeverityProps(rule.severity)}
            
            <div class="border border-gray-200 rounded-lg p-4 {rule.active ? 'bg-white' : 'bg-gray-50'}">
              <div class="flex items-start justify-between mb-3">
                <div class="flex items-center gap-2">
                  <span class="text-lg">{typeProps.icon}</span>
                  <div>
                    <div class="font-medium text-gray-900 text-sm">{rule.name}</div>
                    <div class="text-xs text-gray-600">{rule.description}</div>
                  </div>
                </div>
                
                <div class="flex items-center gap-1">
                  <button
                    class="btn-xs btn-secondary"
                    on:click={() => openPatternEditor(rule)}
                    title="Edit pattern"
                  >
                    ✏️
                  </button>
                  
                  <button
                    class="btn-xs {rule.active ? 'btn-secondary' : 'btn-primary'}"
                    on:click={() => togglePatternStatus(rule)}
                    title="{rule.active ? 'Disable' : 'Enable'} pattern"
                  >
                    {rule.active ? '⏸️' : '▶️'}
                  </button>
                  
                  <button
                    class="btn-xs btn-danger"
                    on:click={() => deletePattern(rule.id)}
                    title="Delete pattern"
                  >
                    🗑️
                  </button>
                </div>
              </div>
              
              <div class="flex items-center gap-2 mb-2">
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs {typeProps.color}">
                  {typeProps.name}
                </span>
                
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs {severityProps.color}">
                  {severityProps.icon} {rule.severity.toUpperCase()}
                </span>
                
                {#if !rule.active}
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs bg-gray-100 text-gray-600">
                    ⏸️ Disabled
                  </span>
                {/if}
              </div>
              
              <div class="mb-2">
                <div class="text-xs font-medium text-gray-700 mb-1">Pattern</div>
                <div class="bg-gray-100 p-2 rounded font-mono text-xs break-all">
                  {rule.pattern}
                </div>
              </div>
              
              <div class="flex items-center justify-between text-xs text-gray-500">
                <span>Confidence: {Math.round((rule.confidenceThreshold || 0.8) * 100)}%</span>
                <span>Triggers: {rule.triggerCount || 0}</span>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="text-center py-8">
          <div class="text-gray-400 mb-4 text-4xl">📝</div>
          <h3 class="text-lg font-medium text-gray-900 mb-2">No Detection Patterns</h3>
          <p class="text-gray-600 mb-4">Add patterns to start detecting secrets in content</p>
          <button 
            class="btn-primary"
            on:click={() => openPatternEditor()}
          >
            ➕ Add First Pattern
          </button>
        </div>
      {/if}
    </div>

    <!-- Recent Detection Results -->
    {#if detectionResults.length > 0}
      <div class="security-card">
        <div class="security-card-header">
          <h3 class="security-card-title">Recent Detections</h3>
          <span class="text-sm text-gray-600">{detectionResults.length} recent results</span>
        </div>
        
        <div class="space-y-3">
          {#each detectionResults.slice(0, 10) as result}
            {@const typeProps = getSecretTypeProps(result.secretType)}
            {@const severityProps = getSeverityProps(result.severity)}
            
            <div class="flex items-center justify-between p-3 border border-gray-200 rounded-lg hover:bg-gray-50">
              <div class="flex items-center gap-3">
                <span class="text-lg">{typeProps.icon}</span>
                <div>
                  <div class="text-sm font-medium text-gray-900">{result.secretName}</div>
                  <div class="text-xs text-gray-600">{result.description}</div>
                </div>
                
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs {severityProps.color}">
                  {severityProps.icon} {result.severity.toUpperCase()}
                </span>
              </div>
              
              <div class="text-right text-xs text-gray-500">
                <div>{Math.round(result.confidence * 100)}% confidence</div>
                <div>{formatTimestamp(result.detectedAt)}</div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<!-- Pattern Editor Modal -->
{#if showPatternEditor}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
    <div class="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto">
      <!-- Modal Header -->
      <div class="sticky top-0 bg-white border-b border-gray-200 px-6 py-4 rounded-t-lg">
        <div class="flex items-center justify-between">
          <h3 class="text-lg font-semibold text-gray-900">
            {editingPattern ? 'Edit' : 'Add'} Detection Pattern
          </h3>
          <button
            class="btn-sm btn-secondary"
            on:click={closePatternEditor}
          >
            ✕ Close
          </button>
        </div>
      </div>
      
      <!-- Modal Content -->
      <div class="p-6 space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">Pattern Name</label>
            <input
              type="text"
              bind:value={newPattern.name}
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="Descriptive name for the pattern..."
            />
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">Secret Type</label>
            <select
              bind:value={newPattern.secretType}
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {#each secretTypes as type}
                <option value={type.type}>{type.icon} {type.name}</option>
              {/each}
            </select>
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">Severity</label>
            <select
              bind:value={newPattern.severity}
              class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="critical">Critical</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
            </select>
          </div>
          
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">Confidence Threshold</label>
            <input
              type="range"
              min="0.1"
              max="1"
              step="0.1"
              bind:value={newPattern.confidence_threshold}
              class="w-full"
            />
            <div class="text-xs text-gray-600 mt-1">
              {Math.round(newPattern.confidence_threshold * 100)}% confidence required
            </div>
          </div>
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Description</label>
          <textarea
            bind:value={newPattern.description}
            rows="2"
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="Describe what this pattern detects..."
          ></textarea>
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Regular Expression Pattern</label>
          <textarea
            bind:value={newPattern.pattern}
            rows="4"
            class="w-full px-3 py-2 border border-gray-300 rounded-md font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="Enter regex pattern to match secrets..."
          ></textarea>
          <div class="text-xs text-gray-600 mt-1">
            Use capturing groups to extract the actual secret value
          </div>
        </div>
        
        <div class="flex items-center">
          <input
            type="checkbox"
            id="patternActive"
            bind:checked={newPattern.active}
            class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
          />
          <label for="patternActive" class="ml-2 text-sm text-gray-700">
            Active (pattern will be used for detection)
          </label>
        </div>
        
        <!-- Quick Pattern Templates -->
        {#if newPattern.secretType}
          {@const templates = secretTypes.find(t => t.type === newPattern.secretType)?.patterns || []}
          {#if templates.length > 0}
            <div class="border-t pt-4">
              <div class="text-sm font-medium text-gray-700 mb-2">Quick Templates</div>
              <div class="grid grid-cols-1 gap-2">
                {#each templates as template}
                  <button
                    type="button"
                    class="text-left p-2 border border-gray-200 rounded hover:bg-gray-50"
                    on:click={() => {
                      newPattern.pattern = template.pattern;
                      if (!newPattern.name) newPattern.name = template.name;
                    }}
                  >
                    <div class="font-medium text-sm">{template.name}</div>
                    <div class="text-xs font-mono text-gray-600">{template.pattern}</div>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        {/if}
      </div>
      
      <!-- Modal Footer -->
      <div class="sticky bottom-0 bg-white border-t border-gray-200 px-6 py-4 rounded-b-lg">
        <div class="flex items-center justify-end gap-3">
          <button class="btn-secondary" on:click={closePatternEditor}>
            Cancel
          </button>
          
          <button 
            class="btn-primary"
            disabled={!newPattern.name.trim() || !newPattern.pattern.trim()}
            on:click={savePattern}
          >
            {editingPattern ? 'Update Pattern' : 'Create Pattern'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Modal overlay styling */
  .fixed.inset-0 {
    backdrop-filter: blur(4px);
  }
  
  /* Custom scrollbar for modal */
  .max-h-\[90vh\] {
    scrollbar-width: thin;
    scrollbar-color: rgb(209, 213, 219) transparent;
  }
  
  .max-h-\[90vh\]::-webkit-scrollbar {
    width: 6px;
  }
  
  .max-h-\[90vh\]::-webkit-scrollbar-track {
    background: transparent;
  }
  
  .max-h-\[90vh\]::-webkit-scrollbar-thumb {
    background-color: rgb(209, 213, 219);
    border-radius: 3px;
  }
  
  /* Button styling */
  .btn-xs {
    @apply px-1.5 py-0.5 text-xs;
  }
  
  /* Progress bar styling */
  .h-3.rounded-full {
    transition: width 0.5s ease;
  }
  
  /* Pre-formatted content styling */
  pre {
    white-space: pre-wrap;
    word-break: break-word;
  }
  
  /* Risk level colors */
  .text-red-700 {
    color: #b91c1c;
  }
  
  .bg-red-100 {
    background-color: #fee2e2;
  }
  
  /* Range input styling */
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    height: 6px;
    background: #e5e7eb;
    border-radius: 3px;
    outline: none;
  }
  
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    background: #3b82f6;
    border-radius: 50%;
    cursor: pointer;
  }
  
  input[type="range"]::-moz-range-thumb {
    width: 16px;
    height: 16px;
    background: #3b82f6;
    border-radius: 50%;
    cursor: pointer;
    border: none;
  }
</style>