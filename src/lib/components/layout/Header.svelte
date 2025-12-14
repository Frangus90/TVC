<script lang="ts">
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import { format, startOfWeek, endOfWeek } from "date-fns";
  import {
    getCurrentDate,
    getViewMode,
    setViewMode,
    previousPeriod,
    nextPeriod,
    goToToday,
  } from "../../stores/calendar.svelte";
  import ThemeSelector from "../ThemeSelector.svelte";

  function getHeaderTitle(): string {
    const date = getCurrentDate();
    const mode = getViewMode();

    if (mode === "week") {
      const weekStart = startOfWeek(date, { weekStartsOn: 1 });
      const weekEnd = endOfWeek(date, { weekStartsOn: 1 });
      return `${format(weekStart, "MMM d")} - ${format(weekEnd, "MMM d, yyyy")}`;
    } else if (mode === "agenda") {
      return "Upcoming Episodes";
    }
    return format(date, "MMMM yyyy");
  }
</script>

<header class="h-14 bg-surface border-b border-border flex items-center justify-between px-4">
  <div class="flex items-center gap-2">
    {#if getViewMode() !== "agenda"}
      <button
        onclick={previousPeriod}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Previous {getViewMode()}"
      >
        <ChevronLeft class="w-5 h-5" />
      </button>

      <button
        onclick={nextPeriod}
        class="p-1.5 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Next {getViewMode()}"
      >
        <ChevronRight class="w-5 h-5" />
      </button>

      <button
        onclick={goToToday}
        class="px-3 py-1.5 text-sm rounded-lg hover:bg-surface-hover transition-colors"
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
    <div class="flex bg-background rounded-lg p-1">
      {#each ["month", "week", "agenda"] as mode}
        <button
          onclick={() => setViewMode(mode as "month" | "week" | "agenda")}
          class="px-3 py-1 text-sm rounded-md transition-colors capitalize {getViewMode() === mode
            ? 'bg-surface text-text'
            : 'text-text-muted hover:text-text'}"
        >
          {mode}
        </button>
      {/each}
    </div>
    <ThemeSelector />
  </div>
</header>
