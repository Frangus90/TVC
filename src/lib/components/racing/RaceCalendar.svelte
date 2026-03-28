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
    parseISO,
  } from "date-fns";
  import { Clock } from "lucide-svelte";
  import { getCurrentDate } from "../../stores/calendar.svelte";
  import {
    getRacingEvents,
    loadRacingEventsForRange,
    getEnabledSeries,
    getSeriesBySlug,
    getSeriesColor,
    isRacingLoading,
    openRacingSettings,
    getRefreshSignal,
    type RacingEvent,
  } from "../../stores/racing.svelte";
  import { formatDate } from "../../utils/dateFormat";

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
      return ev.start_time.startsWith(dateStr);
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

  function formatLocalTime(isoStr: string): string {
    try {
      const date = parseISO(isoStr);
      return format(date, "HH:mm");
    } catch {
      return "";
    }
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

  const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const weekdaysShort = ["M", "T", "W", "T", "F", "S", "S"];
</script>

<div class="h-full flex flex-col">
  <!-- Series filter chips + actions -->
  <div class="px-4 py-2 border-b border-border">
    <div class="flex items-center gap-1.5 flex-wrap">
      {#if getEnabledSeries().length === 0}
        <span class="text-sm text-text-muted">No series enabled.</span>
        <button
          onclick={openRacingSettings}
          class="text-sm text-accent hover:text-accent/80 transition-colors"
        >
          Configure series
        </button>
      {:else}
        {#each getEnabledSeries() as series}
          <button
            onclick={() => toggleSeriesFilter(series.slug)}
            class="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium transition-all whitespace-nowrap
              {activeSeriesFilter === null || activeSeriesFilter === series.slug
                ? 'opacity-100'
                : 'opacity-40'}"
            style="background-color: {getSeriesColor(series)}20; color: {getSeriesColor(series)}; border: 1px solid {getSeriesColor(series)}40;"
          >
            <span
              class="w-2 h-2 rounded-full flex-shrink-0"
              style="background-color: {getSeriesColor(series)};"
            ></span>
            {series.name}
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Weekday headers -->
  <div class="grid grid-cols-7 border-b border-border">
    {#each weekdays as day, i}
      <div class="py-2 text-center text-sm font-medium text-text-muted">
        <span class="hidden sm:inline">{day}</span>
        <span class="sm:hidden">{weekdaysShort[i]}</span>
      </div>
    {/each}
  </div>

  <!-- Calendar grid -->
  <div class="flex-1 grid grid-cols-7 auto-rows-fr">
    {#each calendarDays as day}
      {@const isCurrentMonth = isSameMonth(day, getCurrentDate())}
      {@const today = isToday(day)}
      {@const dayEvents = getEventsForDay(day)}
      <div
        role="region"
        aria-label="Calendar day {formatDate(day)}"
        data-date={format(day, "yyyy-MM-dd")}
        class="border-b border-r border-border p-2 min-h-[100px] overflow-hidden group hover:bg-surface-hover/30 transition-colors {isCurrentMonth
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
            {@const time = formatLocalTime(event.start_time)}
            {@const sessionLabel = getSessionLabel(event.session_name)}
            <button
              onclick={(e) => { e.stopPropagation(); handleDayClick(day); }}
              class="w-full text-left px-2 py-1 rounded text-xs transition-all hover:brightness-125 flex items-center gap-1.5"
              style="background-color: {color}20; border-left: 3px solid {color};"
              title="{event.event_title}{event.session_name ? ': ' + event.session_name : ''}{event.circuit ? ' at ' + event.circuit : ''}"
            >
              <span class="font-semibold truncate" style="color: {color};">{getSeriesAbbrev(event.series_slug)}</span>
              {#if sessionLabel}
                <span class="text-text-muted truncate">{sessionLabel}</span>
              {/if}
              {#if time}
                <span class="text-text-muted/70 ml-auto flex-shrink-0">{time}</span>
              {/if}
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
  <div class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[500px] max-w-[95vw] max-h-[80vh] flex flex-col">
    <!-- Header -->
    <div class="p-5 border-b border-border flex items-center justify-between">
      <div>
        <h3 class="font-semibold text-lg text-text">
          {format(dayDetailDate, "EEEE, MMMM d, yyyy")}
        </h3>
        <p class="text-sm text-text-muted">{dayDetailEvents.length} session{dayDetailEvents.length !== 1 ? 's' : ''}</p>
      </div>
      <button
        onclick={closeDayDetail}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors text-text-muted"
      >
        &times;
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
              style="background-color: {color}25; color: {color};"
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
              {formatLocalTime(event.start_time)}
              {#if event.end_time}
                - {formatLocalTime(event.end_time)}
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
