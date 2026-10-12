<script lang="ts">
  import { fade } from "svelte/transition";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import Sidebar from "./lib/components/layout/Sidebar.svelte";
  import Header from "./lib/components/layout/Header.svelte";
  import CalendarToolbar from "./lib/components/layout/CalendarToolbar.svelte";
  import ToastContainer from "./lib/components/ToastContainer.svelte";
  import ConfirmDialog from "./lib/components/common/ConfirmDialog.svelte";
  import { getViewMode, goToToday } from "./lib/stores/calendar.svelte";
  import { startClock, stopClock, onDayChange } from "./lib/stores/now.svelte";
  import { checkForUpdates } from "./lib/stores/updates.svelte";
  import { isSearchModalOpen, refreshCalendar } from "./lib/stores/shows.svelte";
  import { isMovieSearchModalOpen, refreshMoviesCalendar } from "./lib/stores/movies.svelte";
  import { isEpisodePickerOpen } from "./lib/stores/shows.svelte";
  import { isDayDetailOpen } from "./lib/stores/shows.svelte";
  import { isShowDetailOpen } from "./lib/stores/showDetail.svelte";
  import { isMovieDetailOpen } from "./lib/stores/movies.svelte";
  import { isStatisticsModalOpen, closeStatisticsModal } from "./lib/stores/statistics.svelte";
  import { isModalOpen as isDataManagementOpen, closeDataManagement } from "./lib/stores/dataManagement.svelte";
  import { isUpdateModalOpen } from "./lib/stores/updates.svelte";
  import { isWhatsNewOpen } from "./lib/stores/whatsNew.svelte";
  import { initWhatsNew } from "./lib/stores/whatsNew.svelte";
  import { getSidebarTab, setSidebarTab } from "./lib/stores/sidebar.svelte";
  import { navItemById } from "./lib/config/navItems";
  import { showSuccess, showError } from "./lib/stores/toast.svelte";
  import {
    setupNotificationListener,
    loadNotificationSettings,
    loadUnreadCount,
  } from "./lib/stores/notifications.svelte";
  import { isSettingsOpen, closeSettings } from "./lib/stores/settings.svelte";
  import { isTierSearchModalOpen } from "./lib/stores/tiers.svelte";
  import NotificationPopupContainer from "./lib/components/notifications/NotificationPopupContainer.svelte";
  import MigrationProgress from "./lib/components/MigrationProgress.svelte";
  import { setupLibrarySyncListener } from "./lib/stores/librarySync";
  import { setupMigrationListener } from "./lib/stores/migration.svelte";
  import { logger } from "./lib/utils/logger";
  import ErrorBoundary from "./lib/components/common/ErrorBoundary.svelte";
  import { loadTrackedShows, loadArchivedShows } from "./lib/stores/shows.svelte";
  import { loadTrackedMovies, loadArchivedMovies } from "./lib/stores/movies.svelte";
  import { loadTierListShows, loadTierListMovies, loadTiers } from "./lib/stores/tiers.svelte";
  import { loadRacingSeries, loadRacingConfig } from "./lib/stores/racing.svelte";

  // Lazy load modal components only when they're opened
  let SearchModalComponent = $state<any>(null);
  let MovieSearchModalComponent = $state<any>(null);
  let EpisodePickerComponent = $state<any>(null);
  let DayDetailComponent = $state<any>(null);
  let ShowDetailComponent = $state<any>(null);
  let MovieDetailComponent = $state<any>(null);
  let UpdateModalComponent = $state<any>(null);
  let WhatsNewComponent = $state<any>(null);
  let TierSearchModalComponent = $state<any>(null);

  // Load components when modals open
  $effect(() => {
    if (isSearchModalOpen() && !SearchModalComponent) {
      import("./lib/components/SearchModal.svelte").then((mod) => {
        SearchModalComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  $effect(() => {
    if (isMovieSearchModalOpen() && !MovieSearchModalComponent) {
      import("./lib/components/MovieSearchModal.svelte").then((mod) => {
        MovieSearchModalComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  $effect(() => {
    if (isEpisodePickerOpen() && !EpisodePickerComponent) {
      import("./lib/components/EpisodePicker.svelte").then((mod) => {
        EpisodePickerComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  $effect(() => {
    if (isDayDetailOpen() && !DayDetailComponent) {
      import("./lib/components/DayDetail.svelte").then((mod) => {
        DayDetailComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  $effect(() => {
    if (isShowDetailOpen() && !ShowDetailComponent) {
      import("./lib/components/ShowDetail.svelte").then((mod) => {
        ShowDetailComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  $effect(() => {
    if (isMovieDetailOpen() && !MovieDetailComponent) {
      import("./lib/components/MovieDetail.svelte").then((mod) => {
        MovieDetailComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });





  $effect(() => {
    if (isUpdateModalOpen() && !UpdateModalComponent) {
      import("./lib/components/UpdateModal.svelte").then((mod) => {
        UpdateModalComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  $effect(() => {
    if (isWhatsNewOpen() && !WhatsNewComponent) {
      import("./lib/components/WhatsNew.svelte").then((mod) => {
        WhatsNewComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  // Existing notification/configure actions open the matching full page.
  $effect(() => {
    if (isSettingsOpen()) { setSidebarTab("settings"); closeSettings(); }
    if (isDataManagementOpen()) { setSidebarTab("data"); closeDataManagement(); }
    if (isStatisticsModalOpen()) { setSidebarTab("statistics"); closeStatisticsModal(); }
  });

  // Load full pages when first opened, keeping startup light.
  let loadedNav = $state<Record<string, any>>({});
  let navLoadErrors = $state<Record<string, string>>({});
  const activeItem = $derived(navItemById(getSidebarTab()));
  $effect(() => {
    const item = activeItem;
    if (item.kind === "standalone" && item.load && !loadedNav[item.id] && !navLoadErrors[item.id]) {
      item.load().then((mod) => {
        loadedNav[item.id] = mod.default;
      }).catch((error) => {
        navLoadErrors[item.id] = `Could not open ${item.label}. Please try again.`;
        logger.error(`Failed to load ${item.label}`, error);
      });
    }
  });



  $effect(() => {
    if (isTierSearchModalOpen() && !TierSearchModalComponent) {
      import("./lib/components/TierSearchModal.svelte").then((mod) => {
        TierSearchModalComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  // Lazy load calendar views based on current view mode
  let MonthViewComponent = $state<any>(null);
  let WeekViewComponent = $state<any>(null);
  let AgendaViewComponent = $state<any>(null);

  $effect(() => {
    const viewMode = getViewMode();
    if (viewMode === "month" && !MonthViewComponent) {
      import("./lib/components/calendar/MonthView.svelte").then((mod) => {
        MonthViewComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    } else if (viewMode === "week" && !WeekViewComponent) {
      import("./lib/components/calendar/WeekView.svelte").then((mod) => {
        WeekViewComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    } else if (viewMode === "agenda" && !AgendaViewComponent) {
      import("./lib/components/calendar/AgendaView.svelte").then((mod) => {
        AgendaViewComponent = mod.default;
      }).catch((e) => logger.error("Failed to load component", e));
    }
  });

  // Check for updates on app start and listen for Plex scrobble events
  onMount(() => {
    // Libraries also feed calendar pickers, title search, and navigation counts.
    void Promise.all([loadTrackedShows(), loadTrackedMovies(), loadArchivedShows(), loadArchivedMovies(), loadTierListShows(), loadTierListMovies(), loadTiers(), loadRacingSeries(), loadRacingConfig()]).catch(error => showError(`Could not load your library: ${error}`));
    const rejected = (event: PromiseRejectionEvent) => {
      logger.error("Async operation failed", event.reason);
      showError(`Operation failed: ${String(event.reason)}`);
      event.preventDefault();
    };
    window.addEventListener("unhandledrejection", rejected);
    logger.debug("[TVC] App mounted, will check for updates in 2s...");
    if (!import.meta.env.DEV) {
      setTimeout(() => {
        logger.debug("[TVC] Checking for updates...");
        checkForUpdates(false).catch((err) => {
          logger.error("[TVC] Update check failed", err);
        });
      }, 2000);
    } else {
      logger.debug("[TVC] Dev build - skipping automatic update check");
    }

    // Initialize What's New (check if user needs to see changelog)
    initWhatsNew().catch((err) => {
      logger.error("[TVC] What's New init failed", err);
    });

    // Start the real-time clock so the app tracks the current date/time
    startClock();
    const unsubDayChange = onDayChange(() => {
      logger.debug("[TVC] Day rolled over, refreshing calendar");
      goToToday();
      refreshCalendar();
      refreshMoviesCalendar();
    });

    // Load notification settings and unread count, start listener
    loadNotificationSettings().catch((err) => {
      logger.error("[TVC] Notification settings load failed", err);
    });
    loadUnreadCount().catch((err) => {
      logger.error("[TVC] Notification unread count load failed", err);
    });
    const unlistenNotifications = setupNotificationListener();
    const unlistenMigration = setupMigrationListener();
    const unlistenLibrarySync = setupLibrarySyncListener();

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
      window.removeEventListener("unhandledrejection", rejected);
      stopClock();
      unsubDayChange();
      unlistenScrobble?.();
      unlistenNotifications();
      unlistenMigration();
      unlistenLibrarySync();
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
    <div class="focus-app">
      <Header />
      <div class="focus-app-body">
      <Sidebar />
      <main class="focus-main" id="main-content">
        <div class="focus-content" class:focus-calendar-page={activeItem.kind === "calendar"}>
          {#if activeItem.kind === "standalone"}
            {#if loadedNav[activeItem.id]}
              {@const Comp = loadedNav[activeItem.id]}
              {#if activeItem.id === "shows" || activeItem.id === "movies"}
                <Comp mediaType={activeItem.id} />
              {:else if activeItem.id === "settings" || activeItem.id === "data" || activeItem.id === "statistics"}
                <Comp embedded={true} />
              {:else}
                <Comp />
              {/if}
            {:else if navLoadErrors[activeItem.id]}
              <div class="focus-panel p-8 text-center" role="alert">
                <p class="text-text-muted mb-4">{navLoadErrors[activeItem.id]}</p>
                <button type="button" class="focus-button mx-auto" onclick={() => delete navLoadErrors[activeItem.id]}>Try again</button>
              </div>
            {:else}
              <div class="flex items-center justify-center h-full">
                <div class="text-text-muted">Loading {activeItem.label}…</div>
              </div>
            {/if}
          {:else}
            <CalendarToolbar />
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
                {/if}
              </div>
            {/key}
          {/if}
        </div>
      </main>
      </div>
    </div>

    {#if SearchModalComponent}
      <SearchModalComponent />
    {/if}
    {#if MovieSearchModalComponent}
      <MovieSearchModalComponent />
    {/if}
    {#if TierSearchModalComponent}
      <TierSearchModalComponent />
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


    {#if UpdateModalComponent}
      <UpdateModalComponent />
    {/if}
    {#if WhatsNewComponent}
      <WhatsNewComponent />
    {/if}

    <NotificationPopupContainer />
    <MigrationProgress />
    <ToastContainer />
    <ConfirmDialog />
  {/snippet}
</ErrorBoundary>
