<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type LLMServiceStatus, type SamplingToolsResponse, type ElicitationToolsResponse, type EnhancementToolsResponse } from '$lib/api';
  import LLMServiceCard from '$lib/components/LLMServiceCard.svelte';
  import ToolEnhancementPanel from '$lib/components/ToolEnhancementPanel.svelte';

  // Service status data
  let samplingStatus: LLMServiceStatus | null = null;
  let elicitationStatus: LLMServiceStatus | null = null;
  let enhancementStatus: LLMServiceStatus | null = null;

  // Tools data
  let samplingTools: SamplingToolsResponse | null = null;
  let elicitationTools: ElicitationToolsResponse | null = null;
  let enhancementTools: EnhancementToolsResponse | null = null;

  // UI state
  let loading = true;
  let error = '';
  let activeTab = 'overview'; // overview, sampling, elicitation, enhancement
  let refreshInterval: ReturnType<typeof setTimeout> | null = null;

  async function loadLLMServicesData() {
    loading = true;
    error = '';

    try {
      // Load service status in parallel
      const [samplingStatusResult, elicitationStatusResult, enhancementStatusResult] = await Promise.all([
        api.getSamplingStatus().catch(() => null),
        api.getElicitationStatus().catch(() => null),
        api.getEnhancementStatus().catch(() => null)
      ]);

      samplingStatus = samplingStatusResult;
      elicitationStatus = elicitationStatusResult;
      enhancementStatus = enhancementStatusResult;

      // Load tools data in parallel
      const [samplingToolsResult, elicitationToolsResult, enhancementToolsResult] = await Promise.all([
        api.getSamplingTools().catch(() => null),
        api.getElicitationTools().catch(() => null),
        api.getEnhancementTools().catch(() => null)
      ]);

      samplingTools = samplingToolsResult;
      elicitationTools = elicitationToolsResult;
      enhancementTools = enhancementToolsResult;

    } catch (err) {
      error = `Failed to load LLM services data: ${err}`;
      console.error('LLM services loading error:', err);
    } finally {
      loading = false;
    }
  }

  function startAutoRefresh() {
    refreshInterval = setInterval(loadLLMServicesData, 30000); // Refresh every 30 seconds
  }

  function stopAutoRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  onMount(() => {
    loadLLMServicesData();
    startAutoRefresh();

    return () => {
      stopAutoRefresh();
    };
  });

  // Tab switching
  function switchTab(tab: string) {
    activeTab = tab;
  }

  // Get overall service health
  function getOverallHealth(): 'healthy' | 'degraded' | 'error' {
    const services = [samplingStatus, elicitationStatus, enhancementStatus].filter(s => s?.enabled);
    if (services.length === 0) return 'error';
    
    const activeServices = services.filter(s => s?.status === 'active');
    if (activeServices.length === services.length) return 'healthy';
    if (activeServices.length > 0) return 'degraded';
    return 'error';
  }
</script>

<div class="min-h-screen bg-gray-50 p-6">
  <div class="max-w-7xl mx-auto">
    <!-- Header -->
    <div class="mb-8">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold text-gray-900">LLM Services</h1>
          <p class="text-gray-600 mt-2">
            Manage tool sampling, elicitation, and enhancement services powered by Large Language Models
          </p>
        </div>
        <div class="flex items-center gap-4">
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-full {getOverallHealth() === 'healthy' ? 'bg-green-500' : getOverallHealth() === 'degraded' ? 'bg-yellow-500' : 'bg-red-500'}"></div>
            <span class="text-sm text-gray-600 capitalize">{getOverallHealth()}</span>
          </div>
          <button class="btn-primary" on:click={loadLLMServicesData} disabled={loading}>
            {loading ? '🔄' : '🔄'} Refresh
          </button>
        </div>
      </div>
    </div>

    {#if error}
      <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-6">
        <div class="flex items-center">
          <span class="text-red-500 mr-2">⚠️</span>
          <span class="text-red-700">{error}</span>
        </div>
      </div>
    {/if}

    <!-- Navigation Tabs -->
    <div class="border-b border-gray-200 mb-6">
      <nav class="flex space-x-8">
        <button 
          class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'overview' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          on:click={() => switchTab('overview')}
        >
          📊 Overview
        </button>
        <button 
          class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'sampling' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          on:click={() => switchTab('sampling')}
        >
          🎯 Sampling
        </button>
        <button 
          class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'elicitation' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          on:click={() => switchTab('elicitation')}
        >
          🔍 Elicitation
        </button>
        <button 
          class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'enhancement' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          on:click={() => switchTab('enhancement')}
        >
          ⚡ Enhancement
        </button>
      </nav>
    </div>

    {#if loading}
      <div class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
        <span class="ml-3 text-gray-600">Loading LLM services...</span>
      </div>
    {:else}
      <!-- Tab Content -->
      {#if activeTab === 'overview'}
        <!-- Overview Tab -->
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
          <LLMServiceCard 
            title="Sampling Service"
            description="Enhances tool descriptions using LLM-powered analysis"
            status={samplingStatus}
            toolsData={samplingTools ? { enhanced: samplingTools.enhanced_count, total: samplingTools.total, pending: samplingTools.pending_count } : null}
            icon="🎯"
            onNavigate={() => switchTab('sampling')}
          />
          
          <LLMServiceCard 
            title="Elicitation Service"
            description="Extracts metadata and keywords from tool definitions"
            status={elicitationStatus}
            toolsData={elicitationTools ? { enhanced: elicitationTools.completed_count, total: elicitationTools.total, pending: elicitationTools.pending_count } : null}
            icon="🔍"
            onNavigate={() => switchTab('elicitation')}
          />
          
          <LLMServiceCard 
            title="Enhancement Service"
            description="Coordinates full tool enhancement pipeline"
            status={enhancementStatus}
            toolsData={enhancementTools ? { enhanced: enhancementTools.enhanced_count, total: enhancementTools.total, pending: enhancementTools.pending_count } : null}
            icon="⚡"
            onNavigate={() => switchTab('enhancement')}
          />
        </div>

        <!-- Overall Statistics -->
        <div class="bg-white rounded-lg shadow p-6">
          <h3 class="text-lg font-semibold text-gray-900 mb-4">Enhancement Pipeline Statistics</h3>
          <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div class="text-center">
              <div class="text-2xl font-bold text-blue-600">
                {(samplingTools?.total || 0) + (elicitationTools?.total || 0) + (enhancementTools?.total || 0)}
              </div>
              <div class="text-sm text-gray-600">Total Tool Instances</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-green-600">
                {(samplingTools?.enhanced_count || 0) + (elicitationTools?.completed_count || 0) + (enhancementTools?.enhanced_count || 0)}
              </div>
              <div class="text-sm text-gray-600">Enhanced/Completed</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-yellow-600">
                {(samplingTools?.pending_count || 0) + (elicitationTools?.pending_count || 0) + (enhancementTools?.pending_count || 0)}
              </div>
              <div class="text-sm text-gray-600">Pending Enhancement</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-purple-600">
                {enhancementTools?.failed_count || 0}
              </div>
              <div class="text-sm text-gray-600">Failed Enhancements</div>
            </div>
          </div>
        </div>

      {:else if activeTab === 'sampling'}
        <!-- Sampling Tab -->
        <ToolEnhancementPanel 
          serviceType="sampling"
          serviceStatus={samplingStatus}
          toolsData={samplingTools}
          onRefresh={loadLLMServicesData}
        />

      {:else if activeTab === 'elicitation'}
        <!-- Elicitation Tab -->
        <ToolEnhancementPanel 
          serviceType="elicitation"
          serviceStatus={elicitationStatus}
          toolsData={elicitationTools}
          onRefresh={loadLLMServicesData}
        />

      {:else if activeTab === 'enhancement'}
        <!-- Enhancement Tab -->
        <ToolEnhancementPanel 
          serviceType="enhancement"
          serviceStatus={enhancementStatus}
          toolsData={enhancementTools}
          onRefresh={loadLLMServicesData}
        />
      {/if}
    {/if}
  </div>
</div>

<style>
  .btn-primary {
    @apply bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed;
  }
</style>