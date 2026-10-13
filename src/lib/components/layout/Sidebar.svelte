<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { PanelLeftClose, PanelLeft, RefreshCw, Sparkles, CircleCheck, CircleAlert, CloudDownload } from "lucide-svelte";
  import { navItems } from "../../config/navItems";
  import { getTrackedShows } from "../../stores/shows.svelte";
  import { getTrackedMovies } from "../../stores/movies.svelte";
  import { isSidebarCollapsed, toggleSidebar, getSidebarTab, setSidebarTab, getSidebarWidth, setSidebarWidth, getIsResizing, setIsResizing, loadSidebarWidth, saveSidebarWidth, SIDEBAR_COLLAPSED_WIDTH } from "../../stores/sidebar.svelte";
  import { triggerUpdateCheck, isCheckingForUpdates } from "../../stores/updates.svelte";
  import { openWhatsNew, getAppVersion, hasUnseenChanges } from "../../stores/whatsNew.svelte";
  import { setActiveTab as setDataTab } from "../../stores/dataManagement.svelte";
  import type { LibrarySyncStatus } from "../../stores/librarySync";
  import { allLibrariesHaveSynced, unacceptedSyncFailures, acceptedSyncFailures } from "../../stores/librarySync";
  import { logger } from "../../utils/logger";

  let syncStatus = $state<LibrarySyncStatus[]>([]);
  let syncError = $state(false);
  const failed = $derived(syncStatus.reduce((sum, library) => sum + unacceptedSyncFailures(library.report).length, 0));
  const accepted = $derived(syncStatus.some(library => acceptedSyncFailures(library.report).length > 0));
  const stopped = $derived(syncStatus.some(library => library.report?.stopped || library.last_full_report?.stopped));
  const needsAttention = $derived(failed > 0 || syncStatus.some(library => library.interrupted || library.report?.error || library.report?.stopped || library.last_full_report?.stopped || library.last_full_report?.finished_at === null));
  const hasSynced = $derived(allLibrariesHaveSynced(syncStatus));
  const syncLabel = $derived(syncError ? "Sync status unavailable" : needsAttention ? (failed ? `${failed} title${failed === 1 ? " needs" : "s need"} syncing` : stopped ? "Sync stopped" : "Sync needs attention") : accepted ? "Sync failures accepted" : hasSynced ? "Library synced" : "Library sync");

  onMount(() => {
    let disposed = false;
    let requestId = 0;
    const refresh = async () => {
      const request = ++requestId;
      try {
        const result = await invoke<LibrarySyncStatus[]>("get_library_sync_status");
        if (!disposed && request === requestId) { syncStatus = result; syncError = false; }
      } catch { if (!disposed && request === requestId) syncError = true; }
    };
    void loadSidebarWidth();
    void refresh();
    const listeners = ["library-sync-finished", "library-sync-status-changed"].map(event =>
      listen(event, () => { void refresh(); }).catch(error => {
        if (!disposed) syncError = true;
        logger.error("Live sync status could not be connected; open Data to check sync", error);
        return undefined;
      })
    );
    return () => { disposed = true; for (const listener of listeners) void listener.then(stop => stop?.()); };
  });

  function showSync() { setDataTab("sync"); setSidebarTab("data"); }
  function startResize(event: PointerEvent) {
    event.preventDefault();
    setIsResizing(true);
    const startX = event.clientX;
    const startWidth = getSidebarWidth();
    const previousCursor = document.body.style.cursor;
    const previousSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    const move = (event: PointerEvent) => setSidebarWidth(startWidth + event.clientX - startX);
    const stop = () => {
      setIsResizing(false);
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousSelect;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      void saveSidebarWidth();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
  }
</script>

<aside class="focus-sidebar" class:collapsed={isSidebarCollapsed()} class:resizing={getIsResizing()} style:width={`${isSidebarCollapsed() ? SIDEBAR_COLLAPSED_WIDTH : getSidebarWidth()}px`}>
  <div class="focus-nav-heading">
    {#if !isSidebarCollapsed()}<span class="focus-eyebrow">Your library</span>{/if}
    <button type="button" onclick={toggleSidebar} class="focus-icon-button" aria-label={isSidebarCollapsed() ? "Expand navigation" : "Collapse navigation"} title={isSidebarCollapsed() ? "Expand navigation" : "Collapse navigation"}>
      {#if isSidebarCollapsed()}<PanelLeft size={17} />{:else}<PanelLeftClose size={17} />{/if}
    </button>
  </div>
  <nav aria-label="Main navigation" class="focus-navigation">
    {#each navItems as item}
      {@const Icon = item.icon}
      {@const count = item.id === 'shows' ? getTrackedShows().length : item.id === 'movies' ? getTrackedMovies().length : null}
      <button type="button" onclick={() => setSidebarTab(item.id)} class="focus-nav-item" aria-current={getSidebarTab() === item.id ? 'page' : undefined} aria-label={item.label} title={isSidebarCollapsed() ? item.label : undefined}>
        <Icon size={18} strokeWidth={1.7} />
        {#if !isSidebarCollapsed()}<span>{item.label}</span>{#if count !== null}<span class="focus-nav-count">{count}</span>{/if}{/if}
      </button>
    {/each}
  </nav>
  <div class="focus-nav-footer">
    <button type="button" class="focus-sync-link" class:attention={needsAttention || syncError} onclick={showSync} aria-label={syncLabel} title={isSidebarCollapsed() ? syncLabel : undefined}>
      {#if needsAttention || syncError}<CircleAlert size={16} />{:else if hasSynced}<CircleCheck size={16} />{:else}<CloudDownload size={16} />{/if}
      {#if !isSidebarCollapsed()}<span>{syncLabel}</span>{/if}
    </button>
    {#if !isSidebarCollapsed()}<p class="text-xs text-text-muted">Local library · TMDB</p>{/if}
    <button type="button" class="focus-footer-button" onclick={triggerUpdateCheck} disabled={isCheckingForUpdates()} aria-label="Check for updates" title={isSidebarCollapsed() ? "Check for updates" : undefined}>
      <RefreshCw size={14} class={isCheckingForUpdates() ? 'animate-spin' : ''} />
      {#if !isSidebarCollapsed()}<span>{isCheckingForUpdates() ? "Checking…" : "Check for updates"}</span>{/if}
    </button>
    <button type="button" class="focus-footer-button" onclick={openWhatsNew} aria-label="What's new" title="What's new">
      <Sparkles size={14} class={hasUnseenChanges() ? 'text-accent' : ''} />
      {#if !isSidebarCollapsed()}<span>v{getAppVersion() || '…'}</span>{#if hasUnseenChanges()}<span class="focus-unread-dot"></span>{/if}{/if}
    </button>
  </div>
  {#if !isSidebarCollapsed()}<button type="button" class="focus-sidebar-resize" aria-label="Resize navigation" onpointerdown={startResize}></button>{/if}
</aside>
