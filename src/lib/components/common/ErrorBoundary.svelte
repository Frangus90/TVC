<script lang="ts">
  import { AlertTriangle, RefreshCw } from "lucide-svelte";
  import { logger } from "../../utils/logger";
  import type { Snippet } from "svelte";
  let { children, fallback, onError }: {
    children?: Snippet;
    fallback?: Snippet<[Error, () => void]>;
    onError?: (error: Error, info: unknown) => void;
  } = $props();
  const asError = (value: unknown) => value instanceof Error ? value : new Error(String(value));
</script>

<svelte:boundary onerror={(error) => {
  logger.error("Component render failed", error);
  onError?.(asError(error), {});
}}>
  {#if children}{@render children()}{/if}
  {#snippet failed(error, reset)}
    {#if fallback}
      {@render fallback(asError(error), reset)}
    {:else}
      <div class="flex flex-col items-center justify-center gap-4 p-8 text-center" role="alert">
        <AlertTriangle class="w-16 h-16 text-red-400" />
        <h3 class="text-lg font-semibold text-text">Something went wrong</h3>
        <p class="text-sm text-text-muted">{asError(error).message}</p>
        <button onclick={reset} class="px-4 py-2 bg-accent text-white rounded-lg flex items-center gap-2">
          <RefreshCw class="w-4 h-4" /> Try Again
        </button>
      </div>
    {/if}
  {/snippet}
</svelte:boundary>
