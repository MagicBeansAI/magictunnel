<script lang="ts">
  import ModeAwareLayout from '$lib/components/layout/ModeAwareLayout.svelte';
  import '../app.css';
</script>

<ModeAwareLayout>
  <slot />
</ModeAwareLayout>