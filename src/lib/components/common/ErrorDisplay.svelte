<script lang="ts">
  import { AlertCircle } from "lucide-svelte";

  export let error: string;
  export let onRetry: (() => void) | null = null;
</script>

<div class="flex flex-col items-center justify-center gap-3 p-6">
  <AlertCircle class="w-12 h-12 text-red-400" />
  <p class="text-sm text-text-muted text-center">{error}</p>
  {#if onRetry}
    <button
      onclick={onRetry}
      class="px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors"
    >
      Retry
    </button>
  {/if}
</div>


