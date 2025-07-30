<script lang="ts">
  import ToolMetricsDashboard from '$lib/components/ToolMetricsDashboard.svelte';
</script>

<svelte:head>
  <title>Tool Metrics - MagicTunnel Dashboard</title>
  <meta name="description" content="Comprehensive tool usage analytics, performance metrics, and smart discovery insights for MagicTunnel" />
</svelte:head>

<main class="min-h-screen bg-gray-50 dark:bg-gray-900 py-8">
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
    <ToolMetricsDashboard />
  </div>
</main>