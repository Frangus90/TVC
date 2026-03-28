<script lang="ts">
  import { X, Flag, Play, Tv, Download, Info } from "lucide-svelte";
  import { onMount } from "svelte";
  import type { AppNotification } from "../../stores/notifications.svelte";

  interface Props {
    notification: AppNotification;
    duration: number;
    onDismiss: () => void;
    onClick: () => void;
  }

  let { notification, duration, onDismiss, onClick }: Props = $props();

  let visible = $state(true);
  let progress = $state(100);
  let animationFrame: number | undefined;
  let startTime: number | undefined;

  const typeConfig: Record<string, { icon: typeof Flag; color: string }> = {
    racing: { icon: Flag, color: "text-red-400 bg-red-500/20 border-red-500/30" },
    plex: { icon: Play, color: "text-orange-400 bg-orange-500/20 border-orange-500/30" },
    premiere: { icon: Tv, color: "text-accent bg-accent/20 border-accent/30" },
    update: { icon: Download, color: "text-green-400 bg-green-500/20 border-green-500/30" },
    system: { icon: Info, color: "text-blue-400 bg-blue-500/20 border-blue-500/30" },
  };

  const config = $derived(typeConfig[notification.type] || typeConfig.system);
  let IconComponent = $derived(config.icon);

  onMount(() => {
    if (duration > 0) {
      startTime = performance.now();
      const animate = (now: number) => {
        const elapsed = now - startTime!;
        progress = Math.max(0, 100 - (elapsed / duration) * 100);
        if (progress > 0) {
          animationFrame = requestAnimationFrame(animate);
        }
      };
      animationFrame = requestAnimationFrame(animate);
    }

    return () => {
      if (animationFrame) cancelAnimationFrame(animationFrame);
    };
  });

  function handleDismiss(e: MouseEvent) {
    e.stopPropagation();
    visible = false;
    setTimeout(onDismiss, 200);
  }

  function handleClick() {
    visible = false;
    setTimeout(onClick, 200);
  }
</script>

{#if visible}
  <div
    class="w-[380px] rounded-lg border border-border bg-surface shadow-xl cursor-pointer transition-all animate-slide-in overflow-hidden text-left"
    role="button"
    tabindex="0"
    aria-label="View notification: {notification.title}"
    onclick={handleClick}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleClick(); } }}
  >
    <div class="flex items-start gap-3 p-4">
      <div class="flex-shrink-0 mt-0.5 p-1.5 rounded-md {config.color}">
        <IconComponent class="w-4 h-4" />
      </div>

      <div class="flex-1 min-w-0">
        <p class="text-sm font-semibold text-text truncate">{notification.title}</p>
        <p class="text-sm text-text-muted mt-0.5 line-clamp-2">{notification.body}</p>
      </div>

      <button
        onclick={handleDismiss}
        class="flex-shrink-0 p-1 rounded hover:bg-surface-hover transition-colors text-text-muted hover:text-text"
        aria-label="Dismiss notification"
      >
        <X class="w-4 h-4" />
      </button>
    </div>

    {#if duration > 0}
      <div class="h-0.5 bg-border">
        <div
          class="h-full bg-accent/50 transition-none"
          style="width: {progress}%"
        ></div>
      </div>
    {/if}
  </div>
{/if}

<style>
  @keyframes slide-in {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  .animate-slide-in {
    animation: slide-in 0.3s ease-out;
  }

  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
