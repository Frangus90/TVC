<script lang="ts">
  import {
    startOfMonth,
    endOfMonth,
    startOfWeek,
    endOfWeek,
    eachDayOfInterval,
    format,
    isToday,
    isSameMonth,
  } from "date-fns";
  import { Check, Clock, ChevronLeft, ChevronRight, RefreshCw, Settings, X } from "lucide-svelte";
  import { getCurrentDate, getViewMode, setViewMode, previousPeriod, nextPeriod, goToToday } from "../../stores/calendar.svelte";
  import {
    getRacingEvents,
    loadRacingEventsForRange,
    getEnabledSeries,
    getSeriesBySlug,
    getSeriesColor,
    isRacingLoading,
    getRefreshSignal,
    refreshRacingData,
    isRacingRefreshing,
    type RacingEvent,
  } from "../../stores/racing.svelte";
  import { openSettings } from "../../stores/settings.svelte";
  import { formatDate, formatDateKey, formatLongDate, formatMonthYearLong, formatTime } from "../../utils/dateFormat";
  import { modalFocus } from "../../utils/modalFocus";

  // Day detail state
  let dayDetailOpen = $state(false);
  let dayDetailDate = $state<Date | null>(null);
  let dayDetailEvents = $state<RacingEvent[]>([]);

  // Series filter: null means show all
  let activeSeriesFilter = $state<string | null>(null);

  // Load racing events when month changes or data is refreshed
  $effect(() => {
    const monthStart = startOfMonth(getCurrentDate());
    const monthEnd = endOfMonth(getCurrentDate());
    const calendarStart = startOfWeek(monthStart, { weekStartsOn: 1 });
    const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 1 });

    // Track refresh signal so this effect re-fires after data changes
    getRefreshSignal();

    const startStr = format(calendarStart, "yyyy-MM-dd");
    const endStr = format(calendarEnd, "yyyy-MM-dd");
    loadRacingEventsForRange(startStr, endStr);
  });

  let calendarDays = $derived.by(() => {
    const monthStart = startOfMonth(getCurrentDate());
    const monthEnd = endOfMonth(getCurrentDate());
    const calendarStart = startOfWeek(monthStart, { weekStartsOn: 1 });
    const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 1 });
    return eachDayOfInterval({ start: calendarStart, end: calendarEnd });
  });

  function getEventsForDay(day: Date): RacingEvent[] {
    const dateStr = format(day, "yyyy-MM-dd");
    return getRacingEvents().filter((ev) => {
      if (activeSeriesFilter && ev.series_slug !== activeSeriesFilter) return false;
      return formatDateKey(ev.start_time) === dateStr;
    });
  }

  function getEventColor(event: RacingEvent): string {
    const series = getSeriesBySlug(event.series_slug);
    return series ? getSeriesColor(series) : "#6b7280";
  }

  function getSeriesAbbrev(slug: string): string {
    const abbrevs: Record<string, string> = {
      f1: "F1",
      f2: "F2",
      f3: "F3",
      motogp: "MGP",
      moto2: "M2",
      moto3: "M3",
      "formula-e": "FE",
      wec: "WEC",
      indycar: "IND",
      "nascar-cup": "NAS",
      wrc: "WRC",
      "f1-sprint": "F1S",
      "imsa-wsc": "IMSA",
      dtm: "DTM",
      "super-formula": "SF",
      supergt: "SGT",
      "world-sbk": "SBK",
      "british-gt": "BGT",
      elms: "ELMS",
      "gt-world": "GTW",
      "nascar-xfinity": "NXF",
    };
    return abbrevs[slug] || slug.toUpperCase().slice(0, 3);
  }

  function getSessionLabel(session: string | null): string {
    if (!session) return "";
    const s = session.toLowerCase();
    if (s === "race") return "Race";
    if (s.includes("qualifying") || s === "quali") return "Qual";
    if (s === "sprint") return "Sprint";
    if (s.includes("sprint qual") || s.includes("sprint shootout")) return "Sprint Q";
    if (s === "fp1" || s === "practice 1" || s === "free practice 1") return "FP1";
    if (s === "fp2" || s === "practice 2" || s === "free practice 2") return "FP2";
    if (s === "fp3" || s === "practice 3" || s === "free practice 3") return "FP3";
    if (s === "warmup" || s === "warm up") return "Warm Up";
    if (s === "q1") return "Q1";
    if (s === "q2") return "Q2";
    return session.length > 10 ? session.slice(0, 10) : session;
  }

  function handleDayClick(day: Date) {
    const events = getEventsForDay(day);
    if (events.length === 0) return;
    dayDetailDate = day;
    dayDetailEvents = events;
    dayDetailOpen = true;
  }

  function closeDayDetail() {
    dayDetailOpen = false;
    dayDetailDate = null;
    dayDetailEvents = [];
  }

  function toggleSeriesFilter(slug: string) {
    activeSeriesFilter = activeSeriesFilter === slug ? null : slug;
  }

  function changeMonth(direction: "previous" | "next") {
    const view = getViewMode();
    setViewMode("month");
    if (direction === "previous") previousPeriod();
    else nextPeriod();
    setViewMode(view);
  }

  const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const weekdaysShort = ["M", "T", "W", "T", "F", "S", "S"];
</script>

<div class="focus-page focus-racing relative flex flex-col">
  <header class="focus-page-heading">
    <div class="focus-page-bar">
      <h1>Racing</h1>
      <strong class="text-sm">{formatMonthYearLong(getCurrentDate())}</strong>
      <button type="button" class="focus-button" onclick={goToToday}>Today</button>
      <button type="button" class="focus-icon-button" onclick={() => changeMonth('previous')} aria-label="Previous month"><ChevronLeft class="w-4 h-4" /></button>
      <button type="button" class="focus-icon-button" onclick={() => changeMonth('next')} aria-label="Next month"><ChevronRight class="w-4 h-4" /></button>
    </div>
    <div class="focus-page-bar">
      <button type="button" class="focus-button" onclick={() => openSettings('racing')}><Settings class="w-4 h-4" /> Configure</button>
      <button type="button" class="focus-button" onclick={() => refreshRacingData()} disabled={isRacingRefreshing()}><RefreshCw class="w-4 h-4 {isRacingRefreshing() ? 'animate-spin' : ''}" /> Refresh</button>
    </div>
  </header>
  <div class="focus-racing-calendar flex flex-col">
    <!-- Series filter chips + actions -->
    <div class="px-4 py-2 border-b border-border">
      <div class="flex items-center gap-1.5 flex-wrap">
        {#if getEnabledSeries().length === 0}
          <span class="text-sm text-text-muted">No series enabled.</span>
          <button
            onclick={() => openSettings('racing')}
            class="text-sm text-accent hover:text-accent/80 transition-colors"
          >
            Configure series
          </button>
        {:else}
          <button type="button" class="focus-series-all" aria-pressed={activeSeriesFilter === null} onclick={() => activeSeriesFilter = null}>{#if activeSeriesFilter === null}<Check size={12} aria-hidden="true" />{/if}All series</button>
          {#each getEnabledSeries() as series}
            <button
              onclick={() => toggleSeriesFilter(series.slug)}
              aria-pressed={activeSeriesFilter === null || activeSeriesFilter === series.slug}
              class="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium transition-all whitespace-nowrap"
              style="background-color: var(--color-surface-hover); color: var(--color-text); border: 1px solid {activeSeriesFilter === null || activeSeriesFilter === series.slug ? 'var(--color-text-muted)' : 'var(--color-border)'};"
            >
              <span
                class="w-2 h-2 rounded-full flex-shrink-0"
                style="background-color: {getSeriesColor(series)};"
              ></span>
              {series.name}
              {#if activeSeriesFilter === series.slug}<Check size={12} aria-hidden="true" />{/if}
            </button>
          {/each}
        {/if}
        <span class="text-xs text-text-muted ml-auto">All times local</span>
      </div>
    </div>

    <!-- Weekday headers -->
    <div class="grid grid-cols-7 border-b border-border">
      {#each weekdays as day, i}
        <div class="px-3 py-3 text-left text-xs font-semibold text-text-muted">
          <span class="hidden sm:inline">{day}</span>
          <span class="sm:hidden">{weekdaysShort[i]}</span>
        </div>
      {/each}
    </div>

    <!-- Calendar grid -->
    <div class="focus-racing-grid grid grid-cols-7">
      {#each calendarDays as day}
        {@const isCurrentMonth = isSameMonth(day, getCurrentDate())}
        {@const today = isToday(day)}
        {@const dayEvents = getEventsForDay(day)}
        <div
          role="region"
          aria-label="Calendar day {formatDate(day)}"
          data-date={format(day, "yyyy-MM-dd")}
          class="focus-racing-day border-b border-r border-border p-2 min-h-[100px] group hover:bg-surface-hover/30 transition-colors {isCurrentMonth
            ? ''
            : 'bg-background/50'}"
        >
          <div class="flex items-center justify-between mb-1">
            <button
              onclick={() => handleDayClick(day)}
              class="text-sm hover:bg-surface-hover rounded-full transition-colors {today
                ? 'bg-accent text-white w-7 h-7 flex items-center justify-center font-semibold ring-2 ring-accent/50'
                : isCurrentMonth
                  ? 'text-text w-6 h-6 flex items-center justify-center'
                  : 'text-text-muted w-6 h-6 flex items-center justify-center'}"
            >
              {format(day, "d")}
            </button>
          </div>

          <!-- Racing event pills -->
          <div class="space-y-0.5">
            {#each dayEvents.slice(0, 5) as event}
              {@const color = getEventColor(event)}
              {@const time = formatTime(event.start_time)}
              {@const sessionLabel = getSessionLabel(event.session_name)}
              <button
                onclick={(e) => { e.stopPropagation(); handleDayClick(day); }}
                class="focus-racing-session w-full text-left px-2 py-2 rounded-lg text-xs transition-all hover:brightness-125"
                style="border-left: 3px solid {color};"
                title="{event.event_title}{event.session_name ? ': ' + event.session_name : ''}{event.circuit ? ' at ' + event.circuit : ''}"
              >
                <span class="flex items-center gap-1.5">
                  <span class="font-semibold truncate text-text">{getSeriesAbbrev(event.series_slug)}</span>
                  {#if sessionLabel}<span class="text-text-muted truncate">{sessionLabel}</span>{/if}
                  {#if time}<span class="text-text-muted ml-auto flex-shrink-0">{time}</span>{/if}
                </span>
                <span class="block truncate text-text mt-1">{event.event_title}</span>
              </button>
            {/each}
            {#if dayEvents.length > 5}
              <button
                onclick={() => handleDayClick(day)}
                class="w-full text-xs text-accent hover:text-accent/80 text-center py-0.5 hover:bg-surface-hover rounded transition-colors"
              >
                +{dayEvents.length - 5} more
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>

  <!-- Loading overlay -->
  {#if isRacingLoading()}
    <div class="absolute inset-0 bg-background/50 flex items-center justify-center">
      <div class="text-text-muted text-sm">Loading events...</div>
    </div>
  {/if}
</div>

<!-- Day detail modal -->
{#if dayDetailOpen && dayDetailDate}
  <button
    class="fixed inset-0 bg-black/50 z-40"
    onclick={closeDayDetail}
    aria-label="Close"
  ></button>
  <div class="focus-panel focus-dialog fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[600px] max-w-[95vw] max-h-[80vh] flex flex-col" role="dialog" aria-modal="true" aria-labelledby="racing-day-detail-title" use:modalFocus={closeDayDetail}>
    <!-- Header -->
    <div class="p-5 border-b border-border flex items-center justify-between">
      <div>
        <h3 id="racing-day-detail-title" class="font-semibold text-lg text-text">
          {formatLongDate(dayDetailDate)}
        </h3>
        <p class="text-sm text-text-muted">{dayDetailEvents.length} session{dayDetailEvents.length !== 1 ? 's' : ''}</p>
      </div>
      <button
        onclick={closeDayDetail}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors text-text-muted"
        aria-label="Close racing day details"
      >
        <X class="w-5 h-5" />
      </button>
    </div>

    <!-- Events list -->
    <div class="flex-1 overflow-auto p-5 space-y-3">
      {#each dayDetailEvents as event}
        {@const series = getSeriesBySlug(event.series_slug)}
        {@const color = getEventColor(event)}
        <div
          class="p-3 rounded-lg border border-border hover:bg-surface-hover/50 transition-colors"
          style="border-left: 3px solid {color};"
        >
          <div class="flex items-center gap-2 mb-1">
            <span
              class="px-2 py-0.5 rounded text-xs font-bold"
              style="background-color: var(--color-surface-hover); color: var(--color-text); border-left: 3px solid {color};"
            >
              {series?.name || event.series_slug}
            </span>
            {#if event.session_name}
              <span class="text-xs text-text-muted">{event.session_name}</span>
            {/if}
          </div>
          <p class="font-medium text-text text-sm">{event.event_title}</p>
          <div class="flex items-center gap-3 mt-1.5 text-xs text-text-muted">
            {#if event.circuit}
              <span>{event.circuit}</span>
            {/if}
            <span class="flex items-center gap-1">
              <Clock class="w-3 h-3" />
              {formatTime(event.start_time)}
              {#if event.end_time}
                - {formatTime(event.end_time)}
              {/if}
            </span>
          </div>
        </div>
      {/each}
    </div>

    <!-- Footer -->
    <div class="p-4 border-t border-border">
      <button
        onclick={closeDayDetail}
        class="w-full py-2 text-text-muted hover:bg-surface-hover rounded-lg transition-colors text-sm"
      >
        Close
      </button>
    </div>
  </div>
{/if}

<style>
  .focus-racing { min-height: 0; }
  .focus-racing-calendar { overflow: hidden; background: var(--color-surface); border: 1px solid var(--color-border); border-radius: 0.875rem; min-height: 0; }
  .focus-racing-grid { grid-auto-rows: minmax(6.25rem, auto); }
  .focus-racing-day:nth-child(7n) { border-right: 0; }
  .focus-racing-day:nth-last-child(-n+7) { border-bottom: 0; }
  .focus-racing-session { background: var(--color-surface-hover); border: 1px solid var(--color-border); }
  .focus-series-all { display: inline-flex; align-items: center; gap: .35rem; padding: 0.35rem 0.8rem; border: 1px solid var(--color-border); border-radius: 2rem; font-size: 0.75rem; color: var(--color-text-muted); }
  .focus-series-all[aria-pressed="true"] { color: var(--accent-text); background: color-mix(in srgb, var(--color-accent) 8%, var(--color-surface)); border-color: var(--accent-focus); }
  :global(.compact-spacing) .focus-racing-grid { grid-auto-rows: minmax(5.75rem, auto); }
  :global(.compact-spacing) .focus-racing-day { min-height: 5.75rem; }
</style>
