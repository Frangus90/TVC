<script lang="ts">
  import { Flag, Play, Tv, Download, Info, Check, Trash2, Settings, BellOff } from "lucide-svelte";
  import { onMount } from "svelte";
  import { formatDistanceToNow } from "date-fns";
  import {
    getNotifications,
    getUnreadCount,
    loadNotifications,
    markRead,
    markAllRead,
    dismissNotification,
    dismissAllNotifications,
    closeNotificationCenter,
    openNotificationSettings,
  } from "../../stores/notifications.svelte";
  import type { AppNotification } from "../../stores/notifications.svelte";

  let filter = $state<"all" | "unread">("all");
  let containerEl: HTMLDivElement | undefined = $state();

  const notifications = $derived(getNotifications());
  const unreadCount = $derived(getUnreadCount());

  const filtered = $derived(
    filter === "unread"
      ? notifications.filter((n) => !n.read)
      : notifications,
  );

  const typeIcons: Record<string, typeof Flag> = {
    racing: Flag,
    plex: Play,
    premiere: Tv,
    update: Download,
    system: Info,
  };

  const typeColors: Record<string, string> = {
    racing: "text-red-400",
    plex: "text-orange-400",
    premiere: "text-accent",
    update: "text-green-400",
    system: "text-blue-400",
  };

  onMount(() => {
    loadNotifications();

    function handleClickOutside(e: MouseEvent) {
      if (containerEl && !containerEl.contains(e.target as Node)) {
        // Check if click is on the bell button (parent handles that)
        const bellBtn = (e.target as Element)?.closest("[data-notification-bell]");
        if (!bellBtn) {
          closeNotificationCenter();
        }
      }
    }

    // Delay to avoid the opening click triggering close
    setTimeout(() => {
      document.addEventListener("click", handleClickOutside);
    }, 0);

    return () => {
      document.removeEventListener("click", handleClickOutside);
    };
  });

  function formatTime(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return formatDistanceToNow(date, { addSuffix: true });
    } catch {
      return dateStr;
    }
  }

  function handleNotificationClick(notif: AppNotification) {
    if (!notif.read) {
      markRead(notif.id);
    }
  }

  function handleDismiss(e: MouseEvent, id: number) {
    e.stopPropagation();
    dismissNotification(id);
  }

  function handleOpenSettings(e: MouseEvent) {
    e.stopPropagation();
    closeNotificationCenter();
    openNotificationSettings();
  }
</script>

<div
  bind:this={containerEl}
  class="absolute top-full right-0 mt-2 w-[360px] max-h-[480px] flex flex-col bg-surface border border-border rounded-lg shadow-xl z-50 overflow-hidden"
>
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 border-b border-border">
    <h3 class="text-sm font-semibold text-text">Notifications</h3>
    <div class="flex items-center gap-1">
      {#if unreadCount > 0}
        <button
          onclick={() => markAllRead()}
          class="p-1.5 rounded hover:bg-surface-hover transition-colors text-text-muted hover:text-text"
          title="Mark all as read"
        >
          <Check class="w-4 h-4" />
        </button>
      {/if}
      {#if notifications.length > 0}
        <button
          onclick={() => dismissAllNotifications()}
          class="p-1.5 rounded hover:bg-surface-hover transition-colors text-text-muted hover:text-text"
          title="Clear all"
        >
          <Trash2 class="w-4 h-4" />
        </button>
      {/if}
      <button
        onclick={handleOpenSettings}
        class="p-1.5 rounded hover:bg-surface-hover transition-colors text-text-muted hover:text-text"
        title="Notification settings"
      >
        <Settings class="w-4 h-4" />
      </button>
    </div>
  </div>

  <!-- Filter tabs -->
  <div class="flex px-4 pt-2 pb-1 gap-2">
    {#each [{ value: "all", label: "All" }, { value: "unread", label: "Unread" }] as tab}
      <button
        onclick={() => (filter = tab.value as "all" | "unread")}
        class="px-2.5 py-1 text-xs rounded-md transition-colors {filter === tab.value
          ? 'bg-surface-hover text-text'
          : 'text-text-muted hover:text-text'}"
      >
        {tab.label}
        {#if tab.value === "unread" && unreadCount > 0}
          <span class="ml-1 text-accent">{unreadCount}</span>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Notification list -->
  <div class="flex-1 overflow-y-auto">
    {#if filtered.length === 0}
      <div class="flex flex-col items-center justify-center py-12 text-text-muted">
        <BellOff class="w-8 h-8 mb-2 opacity-50" />
        <p class="text-sm">
          {filter === "unread" ? "No unread notifications" : "No notifications yet"}
        </p>
      </div>
    {:else}
      {#each filtered as notif (notif.id)}
        {@const IconComponent = typeIcons[notif.type] || Info}
        <div
          class="w-full flex items-start gap-3 px-4 py-3 hover:bg-surface-hover transition-colors text-left border-b border-border/50 last:border-b-0 cursor-pointer group {!notif.read ? 'bg-surface-hover/30' : ''}"
          role="button"
          tabindex="0"
          onclick={() => handleNotificationClick(notif)}
          onkeydown={(e) => { if (e.key === 'Enter') handleNotificationClick(notif); }}
        >
          <!-- Unread indicator -->
          <div class="flex-shrink-0 mt-1.5">
            {#if !notif.read}
              <div class="w-2 h-2 rounded-full bg-accent"></div>
            {:else}
              <div class="w-2 h-2"></div>
            {/if}
          </div>

          <!-- Type icon -->
          <div class="flex-shrink-0 mt-0.5 {typeColors[notif.type] || 'text-text-muted'}">
            <IconComponent class="w-4 h-4" />
          </div>

          <!-- Content -->
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-text truncate">{notif.title}</p>
            <p class="text-xs text-text-muted mt-0.5 line-clamp-2">{notif.body}</p>
            <p class="text-xs text-text-muted/60 mt-1">{formatTime(notif.created_at)}</p>
          </div>

          <!-- Dismiss button -->
          <button
            onclick={(e) => handleDismiss(e, notif.id)}
            class="flex-shrink-0 p-1 rounded hover:bg-background transition-colors text-text-muted hover:text-text opacity-0 group-hover:opacity-100"
            title="Dismiss"
          >
            <Trash2 class="w-3.5 h-3.5" />
          </button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
