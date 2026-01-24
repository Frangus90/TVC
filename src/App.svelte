<script lang="ts">
  import { fade } from "svelte/transition";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import Sidebar from "./lib/components/layout/Sidebar.svelte";
  import Header from "./lib/components/layout/Header.svelte";
  import ToastContainer from "./lib/components/ToastContainer.svelte";
  import ConfirmDialog from "./lib/components/common/ConfirmDialog.svelte";
  import DragGhost from "./lib/components/common/DragGhost.svelte";
  import { getViewMode } from "./lib/stores/calendar.svelte";
  import { checkForUpdates } from "./lib/stores/updates.svelte";
  import { isSearchModalOpen, refreshCalendar } from "./lib/stores/shows.svelte";
  import { isMovieSearchModalOpen, refreshMoviesCalendar } from "./lib/stores/movies.svelte";
  import { isEpisodePickerOpen } from "./lib/stores/shows.svelte";
  import { isDayDetailOpen } from "./lib/stores/shows.svelte";
  import { isShowDetailOpen } from "./lib/stores/showDetail.svelte";
  import { isMovieDetailOpen } from "./lib/stores/movies.svelte";
  import { isStatisticsModalOpen } from "./lib/stores/statistics.svelte";
  import { isModalOpen as isDataManagementOpen } from "./lib/stores/dataManagement.svelte";
  import { isUpdateModalOpen } from "./lib/stores/updates.svelte";
  import { isModalOpen as isArrSettingsOpen } from "./lib/stores/arr.svelte";
  import { isModalOpen as isPlexSettingsOpen } from "./lib/stores/plex.svelte";
  import { showSuccess } from "./lib/stores/toast.svelte";
  import { logger } from "./lib/utils/logger";
  import ErrorBoundary from "./lib/components/common/ErrorBoundary.svelte";

  // Lazy load modal components only when they're opened
  let SearchModalComponent = $state<any>(null);
  let MovieSearchModalComponent = $state<any>(null);
  let EpisodePickerComponent = $state<any>(null);
  let DayDetailComponent = $state<any>(null);
  let ShowDetailComponent = $state<any>(null);
  let MovieDetailComponent = $state<any>(null);
  let StatisticsDashboardComponent = $state<any>(null);
  let DataManagementComponent = $state<any>(null);
  let UpdateModalComponent = $state<any>(null);
  let ArrServersComponent = $state<any>(null);
  let PlexSettingsComponent = $state<any>(null);

  // Load components when modals open
  $effect(() => {
    if (isSearchModalOpen() && !SearchModalComponent) {
      import("./lib/components/SearchModal.svelte").then((mod) => {
        SearchModalComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isMovieSearchModalOpen() && !MovieSearchModalComponent) {
      import("./lib/components/MovieSearchModal.svelte").then((mod) => {
        MovieSearchModalComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isEpisodePickerOpen() && !EpisodePickerComponent) {
      import("./lib/components/EpisodePicker.svelte").then((mod) => {
        EpisodePickerComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isDayDetailOpen() && !DayDetailComponent) {
      import("./lib/components/DayDetail.svelte").then((mod) => {
        DayDetailComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isShowDetailOpen() && !ShowDetailComponent) {
      import("./lib/components/ShowDetail.svelte").then((mod) => {
        ShowDetailComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isMovieDetailOpen() && !MovieDetailComponent) {
      import("./lib/components/MovieDetail.svelte").then((mod) => {
        MovieDetailComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isStatisticsModalOpen() && !StatisticsDashboardComponent) {
      import("./lib/components/StatisticsDashboard.svelte").then((mod) => {
        StatisticsDashboardComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isDataManagementOpen() && !DataManagementComponent) {
      import("./lib/components/DataManagement.svelte").then((mod) => {
        DataManagementComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isUpdateModalOpen() && !UpdateModalComponent) {
      import("./lib/components/UpdateModal.svelte").then((mod) => {
        UpdateModalComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isArrSettingsOpen() && !ArrServersComponent) {
      import("./lib/components/ArrServers.svelte").then((mod) => {
        ArrServersComponent = mod.default;
      });
    }
  });

  $effect(() => {
    if (isPlexSettingsOpen() && !PlexSettingsComponent) {
      import("./lib/components/PlexSettings.svelte").then((mod) => {
        PlexSettingsComponent = mod.default;
      });
    }
  });

  // Lazy load calendar views based on current view mode
  let MonthViewComponent = $state<any>(null);
  let WeekViewComponent = $state<any>(null);
  let AgendaViewComponent = $state<any>(null);
  let TierViewComponent = $state<any>(null);

  $effect(() => {
    const viewMode = getViewMode();
    if (viewMode === "month" && !MonthViewComponent) {
      import("./lib/components/calendar/MonthView.svelte").then((mod) => {
        MonthViewComponent = mod.default;
      });
    } else if (viewMode === "week" && !WeekViewComponent) {
      import("./lib/components/calendar/WeekView.svelte").then((mod) => {
        WeekViewComponent = mod.default;
      });
    } else if (viewMode === "agenda" && !AgendaViewComponent) {
      import("./lib/components/calendar/AgendaView.svelte").then((mod) => {
        AgendaViewComponent = mod.default;
      });
    } else if (viewMode === "tier" && !TierViewComponent) {
      import("./lib/components/calendar/TierView.svelte").then((mod) => {
        TierViewComponent = mod.default;
      });
    }
  });

  // Check for updates on app start and listen for Plex scrobble events
  onMount(() => {
    logger.debug("[TVC] App mounted, will check for updates in 2s...");
    setTimeout(() => {
      logger.debug("[TVC] Checking for updates...");
      checkForUpdates(false).catch((err) => {
        logger.error("[TVC] Update check failed", err);
      });
    }, 2000);

    // Listen for Plex scrobble events to refresh calendar
    let unlistenScrobble: UnlistenFn | undefined;
    listen<{ media_type: string; entity_id: number }>("plex-scrobble", (event) => {
      logger.debug("[TVC] Plex scrobble event received", event.payload);
      const { media_type } = event.payload;

      // Show toast notification
      showSuccess(`Plex: Marked ${media_type} as watched`);

      // Refresh the appropriate calendar data
      if (media_type === "movie") {
        refreshMoviesCalendar();
      } else if (media_type === "episode") {
        refreshCalendar();
      }
    }).then((unlisten) => {
      unlistenScrobble = unlisten;
    });

    return () => {
      unlistenScrobble?.();
    };
  });

  // CTRL+Shift+I to open dev tools (like browser)
  async function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.shiftKey && event.key === "I") {
      event.preventDefault();
      logger.debug("[TVC] Opening dev tools...");
      try {
        const webview = getCurrentWebview();
        await invoke("plugin:webview|internal_toggle_devtools", {
          label: webview.label,
        });
      } catch (err) {
        logger.error("[TVC] Failed to open dev tools", err);
      }
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<ErrorBoundary>
  {#snippet children()}
    <div class="flex h-screen bg-background text-text">
      <Sidebar />

      <main class="flex-1 flex flex-col overflow-hidden">
        <Header />

        <div class="flex-1 overflow-auto p-6">
          {#key getViewMode()}
            <div in:fade={{ duration: 150, delay: 50 }} out:fade={{ duration: 100 }}>
              {#if getViewMode() === "month"}
                {#if MonthViewComponent}
                  <MonthViewComponent />
                {:else}
                  <div class="flex items-center justify-center h-full">
                    <div class="text-text-muted">Loading calendar...</div>
                  </div>
                {/if}
              {:else if getViewMode() === "week"}
                {#if WeekViewComponent}
                  <WeekViewComponent />
                {:else}
                  <div class="flex items-center justify-center h-full">
                    <div class="text-text-muted">Loading calendar...</div>
                  </div>
                {/if}
              {:else if getViewMode() === "agenda"}
                {#if AgendaViewComponent}
                  <AgendaViewComponent />
                {:else}
                  <div class="flex items-center justify-center h-full">
                    <div class="text-text-muted">Loading calendar...</div>
                  </div>
                {/if}
              {:else if getViewMode() === "tier"}
                {#if TierViewComponent}
                  <TierViewComponent />
                {:else}
                  <div class="flex items-center justify-center h-full">
                    <div class="text-text-muted">Loading tier list...</div>
                  </div>
                {/if}
              {/if}
            </div>
          {/key}
        </div>
      </main>
    </div>

    {#if SearchModalComponent}
      <SearchModalComponent />
    {/if}
    {#if MovieSearchModalComponent}
      <MovieSearchModalComponent />
    {/if}
    {#if EpisodePickerComponent}
      <EpisodePickerComponent />
    {/if}
    {#if DayDetailComponent}
      <DayDetailComponent />
    {/if}
    {#if ShowDetailComponent}
      <ShowDetailComponent />
    {/if}
    {#if MovieDetailComponent}
      <MovieDetailComponent />
    {/if}
    {#if StatisticsDashboardComponent}
      <StatisticsDashboardComponent />
    {/if}
    {#if DataManagementComponent}
      <DataManagementComponent />
    {/if}
    {#if UpdateModalComponent}
      <UpdateModalComponent />
    {/if}
    {#if ArrServersComponent}
      <ArrServersComponent />
    {/if}
    {#if PlexSettingsComponent}
      <PlexSettingsComponent />
    {/if}
    <ToastContainer />
    <ConfirmDialog />
    <DragGhost />
  {/snippet}
</ErrorBoundary>
