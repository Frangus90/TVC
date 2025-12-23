<script lang="ts">
  import { X, CheckCircle, AlertCircle, Info } from "lucide-svelte";
  import { onMount } from "svelte";

  interface Props {
    message: string;
    type?: "success" | "error" | "info";
    duration?: number;
    onClose: () => void;
    onRetry?: () => void | Promise<void>;
  }

  let { message, type = "info", duration = 3000, onClose, onRetry }: Props = $props();

  let visible = $state(true);

  onMount(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        visible = false;
        setTimeout(onClose, 300);
      }, duration);
      return () => clearTimeout(timer);
    }
  });

  const icons = {
    success: CheckCircle,
    error: AlertCircle,
    info: Info,
  };

  const colors = {
    success: "bg-green-500/20 text-green-400 border-green-500/30",
    error: "bg-red-500/20 text-red-400 border-red-500/30",
    info: "bg-blue-500/20 text-blue-400 border-blue-500/30",
  };

  let IconComponent = $derived(icons[type]);
</script>

{#if visible}
  <div
    class="fixed bottom-4 right-4 z-50 flex items-center gap-3 px-4 py-3 rounded-lg border shadow-lg {colors[type]} transition-all animate-slide-up"
    role="alert"
  >
    <IconComponent class="w-5 h-5 flex-shrink-0" />
    <p class="text-sm font-medium flex-1">{message}</p>
    {#if onRetry && type === "error"}
      <button
        onclick={async () => {
          await onRetry();
          visible = false;
          setTimeout(onClose, 300);
        }}
        class="ml-2 px-3 py-1 text-xs font-medium rounded hover:bg-black/20 transition-colors border border-current/30"
        aria-label="Retry"
      >
        Retry
      </button>
    {/if}
    <button
      onclick={() => { visible = false; setTimeout(onClose, 300); }}
      class="ml-2 p-1 rounded hover:bg-black/10 transition-colors"
      aria-label="Close"
    >
      <X class="w-4 h-4" />
    </button>
  </div>
{/if}

<style>
  @keyframes slide-up {
    from {
      transform: translateY(100%);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .animate-slide-up {
    animation: slide-up 0.3s ease-out;
  }
</style>

