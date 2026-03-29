<script lang="ts">
  import { ChevronLeft, ChevronRight, Power, Bell } from "lucide-svelte";
  import { startOfWeek, endOfWeek } from "date-fns";
  import { invoke } from "@tauri-apps/api/core";
  import {
    getCurrentDate,
    getViewMode,
    setViewMode,
    getCalendarFilter,
    setCalendarFilter,
    previousPeriod,
    nextPeriod,
    goToToday,
  } from "../../stores/calendar.svelte";
  import { getSidebarTab } from "../../stores/sidebar.svelte";
  import { openConfirmDialog } from "../../stores/confirmDialog.svelte";
  import {
    getUnreadCount,
    isNotificationCenterOpen,
    toggleNotificationCenter,
  } from "../../stores/notifications.svelte";
  import NotificationCenter from "../notifications/NotificationCenter.svelte";
  import { logger } from "../../utils/logger";
  import { formatWeekRange, formatMonthYearLong } from "../../utils/dateFormat";

  async function handleExit() {
    const confirmed = await openConfirmDialog({
      title: "Exit Application",
      message: "Are you sure you want to fully exit TVC? The app will close completely and will not minimize to the system tray.",
      type: "warning",
      confirmLabel: "Exit",
      cancelLabel: "Cancel",
    });

    if (confirmed) {
      try {
        await invoke("exit_app");
      } catch (error) {
        logger.error("Failed to exit app", error);
      }
    }
  }

  const isRacing = $derived(getSidebarTab() === "racing");

  function getHeaderTitle(): string {
    const date = getCurrentDate();
    const mode = getViewMode();

    if (isRacing) {
      return formatMonthYearLong(date);
    }

    if (mode === "week") {
      const weekStart = startOfWeek(date, { weekStartsOn: 1 });
      const weekEnd = endOfWeek(date, { weekStartsOn: 1 });
      return formatWeekRange(weekStart, weekEnd);
    } else if (mode === "agenda") {
      return "Upcoming Episodes";
    } else if (mode === "tier") {
      return "Tier Rankings";
    }
    return formatMonthYearLong(date);
  }
</script>

<header class="h-14 bg-surface border-b border-border flex items-center justify-between px-4">
  <div class="flex items-center gap-2">
    {#if isRacing || (getViewMode() !== "agenda" && getViewMode() !== "tier")}
      <button
        onclick={previousPeriod}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Previous month"
        title="Previous month"
      >
        <ChevronLeft class="w-5 h-5" />
      </button>

      <button
        onclick={nextPeriod}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Next month"
        title="Next month"
      >
        <ChevronRight class="w-5 h-5" />
      </button>

      <button
        onclick={goToToday}
        class="px-3 py-1.5 text-sm rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Go to today"
        title="Go to today"
      >
        Today
      </button>
    {:else}
      <div class="w-[140px]"></div>
    {/if}
  </div>

  <h1 class="text-lg font-semibold">
    {getHeaderTitle()}
  </h1>

  <div class="flex items-center gap-2">
    {#if !isRacing}
      {#if getViewMode() !== "tier"}
        <div class="flex bg-background rounded-lg p-1">
          {#each [{ value: "all", label: "All" }, { value: "shows", label: "Shows" }, { value: "movies", label: "Movies" }] as option}
            <button
              onclick={() => setCalendarFilter(option.value as "all" | "shows" | "movies")}
              class="px-3 py-1 text-sm rounded-md transition-colors {getCalendarFilter() === option.value
                ? 'bg-surface text-text'
                : 'text-text-muted hover:text-text'}"
              aria-label="Show {option.label.toLowerCase()}"
              aria-pressed={getCalendarFilter() === option.value}
              title="Show {option.label.toLowerCase()}"
            >
              {option.label}
            </button>
          {/each}
        </div>
      {/if}
      <div class="flex bg-background rounded-lg p-1">
        {#each ["month", "week", "agenda", "tier"] as mode}
          <button
            onclick={() => setViewMode(mode as "month" | "week" | "agenda" | "tier")}
            class="px-3 py-1 text-sm rounded-md transition-colors capitalize {getViewMode() === mode
              ? 'bg-surface text-text'
              : 'text-text-muted hover:text-text'}"
            aria-label="Switch to {mode} view"
            aria-pressed={getViewMode() === mode}
            title="Switch to {mode} view"
          >
            {mode}
          </button>
        {/each}
      </div>
    {/if}
    <div class="relative">
      <button
        data-notification-bell
        onclick={toggleNotificationCenter}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors text-text-muted hover:text-text relative"
        aria-label="Notifications"
        title="Notifications"
      >
        <Bell class="w-5 h-5" />
        {#if getUnreadCount() > 0}
          <span class="absolute -top-0.5 -right-0.5 min-w-[16px] h-4 px-1 bg-red-500 text-white text-[10px] font-bold rounded-full flex items-center justify-center">
            {getUnreadCount() > 9 ? "9+" : getUnreadCount()}
          </span>
        {/if}
      </button>
      {#if isNotificationCenterOpen()}
        <NotificationCenter />
      {/if}
    </div>
    <button
      onclick={handleExit}
      class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors text-text-muted hover:text-red-400"
      aria-label="Exit application"
      title="Exit application (fully close, not minimize to tray)"
    >
      <Power class="w-5 h-5" />
    </button>
  </div>
</header>
