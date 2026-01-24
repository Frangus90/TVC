<script lang="ts">
  import { onMount } from "svelte";
  import { AlertTriangle, RefreshCw } from "lucide-svelte";
  import { logger } from "../../utils/logger";
  import type { Snippet } from "svelte";

  interface Props {
    children?: Snippet;
    fallback?: (error: Error, reset: () => void) => any;
    onError?: (error: Error, errorInfo: any) => void;
  }

  let { children, fallback, onError }: Props = $props();

  let error = $state<Error | null>(null);
  let errorInfo = $state<any>(null);

  function resetError() {
    error = null;
    errorInfo = null;
  }

  onMount(() => {
    // Catch unhandled errors in child components
    const originalErrorHandler = window.onerror;
    window.onerror = (message, source, lineno, colno, err) => {
      if (err) {
        error = err;
        errorInfo = { message, source, lineno, colno };
        logger.error("ErrorBoundary caught error", { message, source, lineno, colno, error: err });
        if (onError) {
          onError(err, errorInfo);
        }
      }
      return originalErrorHandler ? originalErrorHandler(message, source, lineno, colno, err) : false;
    };

    return () => {
      window.onerror = originalErrorHandler;
    };
  });
</script>

{#if error}
  {#if fallback}
    {@render fallback(error, resetError)}
  {:else}
    <div class="flex flex-col items-center justify-center gap-4 p-8 text-center">
      <AlertTriangle class="w-16 h-16 text-red-400" />
      <div>
        <h3 class="text-lg font-semibold text-text mb-2">Something went wrong</h3>
        <p class="text-sm text-text-muted mb-4">{error.message || "An unexpected error occurred"}</p>
        <button
          onclick={resetError}
          class="px-4 py-2 bg-accent hover:bg-accent-hover text-white rounded-lg transition-colors flex items-center gap-2 mx-auto"
          aria-label="Retry"
        >
          <RefreshCw class="w-4 h-4" />
          Try Again
        </button>
      </div>
    </div>
  {/if}
{:else}
  {#if children}
    {@render children()}
  {/if}
{/if}
