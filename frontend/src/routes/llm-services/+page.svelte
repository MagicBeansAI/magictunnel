<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type LLMServiceStatus, type SamplingToolsResponse, type ElicitationToolsResponse, type EnhancementToolsResponse, type LlmProviderListResponse } from '$lib/api';
  import LLMServiceCard from '$lib/components/LLMServiceCard.svelte';
  import ToolEnhancementPanel from '$lib/components/ToolEnhancementPanel.svelte';

  // Service status data
  let samplingStatus: LLMServiceStatus | null = null;
  let elicitationStatus: LLMServiceStatus | null = null;
  let enhancementStatus: LLMServiceStatus | null = null;

  // Tools data
  let enhancementTools: EnhancementToolsResponse | null = null;

  // Providers data
  let providersData: LlmProviderListResponse | null = null;

  // UI state
  let loading = true;
  let error = '';
  let activeTab = 'overview'; // overview, enhancement, providers
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

      // Load tools data for enhancement pipeline
      enhancementTools = await api.getEnhancementTools().catch(() => null);

      // Load LLM providers data
      providersData = await api.getLlmProviders().catch(() => null);

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
          <h1 class="text-3xl font-bold text-gray-900">Tool Enhancement Services</h1>
          <p class="text-gray-600 mt-2">
            Internal AI-powered tool enhancement pipeline - description generation, metadata extraction, and quality improvement
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
          class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'enhancement' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          on:click={() => switchTab('enhancement')}
        >
          ⚡ Enhancement Details
        </button>
        <button 
          class="py-2 px-1 border-b-2 font-medium text-sm {activeTab === 'providers' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          on:click={() => switchTab('providers')}
        >
          🔧 LLM Providers
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
        <!-- Enhancement Service Overview -->
        <div class="grid grid-cols-1 lg:grid-cols-1 gap-6 mb-8">
          <div class="bg-white rounded-lg shadow p-6">
            <div class="flex items-center justify-between mb-4">
              <div class="flex items-center">
                <div class="text-2xl mr-3">⚡</div>
                <div>
                  <h3 class="text-lg font-semibold text-gray-900">Tool Enhancement Pipeline</h3>
                  <p class="text-gray-600 text-sm">AI-powered tool description and metadata enhancement system</p>
                </div>
              </div>
              <div class="flex items-center gap-3">
                <div class="flex items-center gap-2">
                  <div class="w-3 h-3 rounded-full {enhancementStatus?.status === 'active' ? 'bg-green-500' : enhancementStatus?.status === 'disabled' ? 'bg-gray-400' : 'bg-red-500'}"></div>
                  <span class="text-sm text-gray-600 capitalize">{enhancementStatus?.status || 'unknown'}</span>
                </div>
                {#if enhancementStatus?.provider_info}
                  <div class="text-sm text-gray-500">
                    {enhancementStatus.provider_info.provider} • {enhancementStatus.provider_info.model || 'default'}
                  </div>
                {/if}
              </div>
            </div>
            
            <!-- Quick Stats -->
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div class="text-center">
                <div class="text-xl font-bold text-green-600">{enhancementTools?.enhanced_count || 0}</div>
                <div class="text-xs text-gray-600">Enhanced</div>
              </div>
              <div class="text-center">
                <div class="text-xl font-bold text-yellow-600">{enhancementTools?.pending_count || 0}</div>
                <div class="text-xs text-gray-600">Pending</div>
              </div>
              <div class="text-center">
                <div class="text-xl font-bold text-purple-600">{enhancementTools?.eligible_count || 0}</div>
                <div class="text-xs text-gray-600">Eligible</div>
              </div>
              <div class="text-center">
                <div class="text-xl font-bold text-blue-600">{enhancementTools?.total || 0}</div>
                <div class="text-xs text-gray-600">Total Tools</div>
              </div>
            </div>

            <!-- Action buttons -->
            <div class="mt-4 flex gap-2">
              <button 
                class="btn-secondary text-sm py-2 px-3"
                on:click={() => switchTab('enhancement')}
              >
                View Details
              </button>
            </div>
          </div>
        </div>

        <!-- Overall Statistics -->
        <div class="bg-white rounded-lg shadow p-6">
          <h3 class="text-lg font-semibold text-gray-900 mb-4">Tool Enhancement Statistics</h3>
          <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
            <div class="text-center">
              <div class="text-2xl font-bold text-blue-600">
                {enhancementTools?.total || 0}
              </div>
              <div class="text-sm text-gray-600">Total Tools</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-purple-600">
                {enhancementTools?.eligible_count || 0}
              </div>
              <div class="text-sm text-gray-600">Eligible for Enhancement</div>
              <div class="text-xs text-gray-500 mt-1">Excludes external MCP & smart discovery</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-green-600">
                {enhancementTools?.enhanced_count || 0}
              </div>
              <div class="text-sm text-gray-600">Enhanced</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-yellow-600">
                {enhancementTools?.pending_count || 0}
              </div>
              <div class="text-sm text-gray-600">Pending Enhancement</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-red-600">
                {enhancementTools?.failed_count || 0}
              </div>
              <div class="text-sm text-gray-600">Failed</div>
            </div>
          </div>
          
          <!-- Enhancement Progress Bar -->
          {#if enhancementTools?.eligible_count && enhancementTools.eligible_count > 0}
            <div class="mt-6">
              <div class="flex justify-between text-sm text-gray-600 mb-2">
                <span>Enhancement Progress</span>
                <span>{Math.round((enhancementTools.enhanced_count / enhancementTools.eligible_count) * 100)}% Complete</span>
              </div>
              <div class="w-full bg-gray-200 rounded-full h-2">
                <div 
                  class="bg-green-600 h-2 rounded-full transition-all duration-300" 
                  style="width: {(enhancementTools.enhanced_count / enhancementTools.eligible_count) * 100}%"
                ></div>
              </div>
            </div>
          {/if}
        </div>

      {:else if activeTab === 'enhancement'}
        <!-- Enhancement Tab -->
        <ToolEnhancementPanel 
          serviceType="enhancement"
          serviceStatus={enhancementStatus}
          toolsData={enhancementTools}
          onRefresh={loadLLMServicesData}
        />

      {:else if activeTab === 'providers'}
        <!-- Providers Tab -->
        <div class="space-y-6">
          <div class="bg-white rounded-lg shadow p-6">
            <div class="flex items-center justify-between mb-4">
              <h3 class="text-lg font-semibold text-gray-900">Configured LLM Providers</h3>
              <div class="text-sm text-gray-500">
                Total: {providersData?.providers?.length || 0}
              </div>
            </div>

            {#if providersData?.providers && providersData.providers.length > 0}
              <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
                {#each providersData.providers as provider}
                  <div class="border border-gray-200 rounded-lg p-4">
                    <div class="flex items-start justify-between mb-3">
                      <div>
                        <h4 class="font-medium text-gray-900">{provider.name}</h4>
                        <p class="text-sm text-gray-600">
                          {provider.provider_type} • {provider.models.join(', ')}
                        </p>
                      </div>
                      <div class="flex items-center gap-2">
                        <!-- Source Badge -->
                        <span class="px-2 py-1 text-xs rounded-full {provider.source === 'sampling' ? 'bg-blue-100 text-blue-800' : 'bg-purple-100 text-purple-800'}">
                          {provider.source}
                        </span>
                        <!-- Status Indicator -->
                        <div class="w-3 h-3 rounded-full {provider.status === 'configured' ? 'bg-green-500' : provider.status === 'unknown' ? 'bg-gray-400' : 'bg-red-500'}"></div>
                      </div>
                    </div>

                    <div class="space-y-2">
                      {#if provider.purpose}
                        <div class="text-sm">
                          <span class="font-medium text-gray-700">Purpose:</span>
                          <span class="text-gray-600">{provider.purpose}</span>
                        </div>
                      {/if}

                      <div class="text-sm">
                        <span class="font-medium text-gray-700">Endpoint:</span>
                        <span class="text-gray-600">{provider.endpoint || 'default'}</span>
                      </div>

                      <div class="text-sm">
                        <span class="font-medium text-gray-700">API Key:</span>
                        <span class="text-gray-600">{provider.has_api_key ? '✅ Configured' : '❌ Missing'}</span>
                      </div>

                      {#if provider.last_tested}
                        <div class="text-sm">
                          <span class="font-medium text-gray-700">Last Tested:</span>
                          <span class="text-gray-600">{new Date(provider.last_tested).toLocaleString()}</span>
                          {#if provider.last_test_result}
                            <span class="ml-2 px-2 py-0.5 text-xs rounded {provider.last_test_result.includes('success') ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                              {provider.last_test_result}
                            </span>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <div class="text-center py-8">
                <div class="text-gray-500 mb-2">🔧</div>
                <p class="text-gray-600">No LLM providers configured</p>
              </div>
            {/if}
          </div>

          <!-- Provider Summary Stats -->
          {#if providersData?.providers}
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div class="bg-white rounded-lg shadow p-4 text-center">
                <div class="text-2xl font-bold text-blue-600">
                  {providersData.providers.filter(p => p.source === 'sampling').length}
                </div>
                <div class="text-sm text-gray-600">Sampling Providers</div>
              </div>
              <div class="bg-white rounded-lg shadow p-4 text-center">
                <div class="text-2xl font-bold text-purple-600">
                  {providersData.providers.filter(p => p.source === 'discovery').length}
                </div>
                <div class="text-sm text-gray-600">Discovery Providers</div>
              </div>
              <div class="bg-white rounded-lg shadow p-4 text-center">
                <div class="text-2xl font-bold text-green-600">
                  {providersData.providers.filter(p => p.has_api_key).length}
                </div>
                <div class="text-sm text-gray-600">With API Keys</div>
              </div>
              <div class="bg-white rounded-lg shadow p-4 text-center">
                <div class="text-2xl font-bold text-gray-600">
                  {[...new Set(providersData.providers.map(p => p.provider_type))].length}
                </div>
                <div class="text-sm text-gray-600">Unique Types</div>
              </div>
            </div>
          {/if}
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .btn-primary {
    @apply bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed;
  }
  
  .btn-secondary {
    @apply bg-gray-100 hover:bg-gray-200 text-gray-700 font-medium py-2 px-4 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed;
  }
</style>