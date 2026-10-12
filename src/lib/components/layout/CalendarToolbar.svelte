<script lang="ts">
  import { ChevronLeft, ChevronRight, Plus, Tv, Film, CircleCheck } from "lucide-svelte";
  import { startOfWeek, endOfWeek } from "date-fns";
  import { getCurrentDate, getViewMode, setViewMode, getCalendarFilter, setCalendarFilter, previousPeriod, nextPeriod, goToToday } from "../../stores/calendar.svelte";
  import { openSearchModal } from "../../stores/shows.svelte";
  import { openMovieSearchModal } from "../../stores/movies.svelte";
  import { formatMonthYearLong, formatWeekRange } from "../../utils/dateFormat";

  const dateTitle = $derived(getViewMode() === "week"
    ? formatWeekRange(startOfWeek(getCurrentDate(), { weekStartsOn: 1 }), endOfWeek(getCurrentDate(), { weekStartsOn: 1 }))
    : formatMonthYearLong(getCurrentDate()));
</script>

<div class="focus-page-heading">
  <div><p class="focus-eyebrow">Your library</p><h1>Calendar</h1><p class="focus-description">Your planned watching, in one place.</p></div>
  <div class="focus-heading-actions">
    <button type="button" class="focus-button" onclick={openMovieSearchModal}><Film size={16} />Add a movie</button>
    <button type="button" class="focus-button focus-primary" onclick={openSearchModal}><Plus size={16} />Add a show</button>
  </div>
</div>
<div class="focus-toolbar">
  <div class="focus-date-navigation">
    <h2>{getViewMode() === "agenda" ? "Upcoming watching" : dateTitle}</h2>
    {#if getViewMode() !== "agenda"}
    <button type="button" class="focus-button" onclick={goToToday}>Today</button>
    <button type="button" class="focus-icon-button" onclick={previousPeriod} aria-label="Previous period"><ChevronLeft size={19} /></button>
    <button type="button" class="focus-icon-button" onclick={nextPeriod} aria-label="Next period"><ChevronRight size={19} /></button>
    {/if}
  </div>
  <div class="focus-segment" role="group" aria-label="Calendar view">
    {#each ["month", "week", "agenda"] as mode}
      <button type="button" aria-pressed={getViewMode() === mode} onclick={() => setViewMode(mode as "month" | "week" | "agenda")}>{mode[0].toUpperCase() + mode.slice(1)}</button>
    {/each}
  </div>
</div>
<div class="focus-toolbar focus-calendar-filters">
  <div class="focus-filter-group" role="group" aria-label="Calendar titles">
    <span class="text-text-muted text-sm">Show</span>
    {#each [{ value: "all", label: "All titles" }, { value: "shows", label: "TV shows" }, { value: "movies", label: "Movies" }] as option}
      <button type="button" aria-pressed={getCalendarFilter() === option.value} onclick={() => setCalendarFilter(option.value as "all" | "shows" | "movies")}>{option.label}</button>
    {/each}
  </div>
  <div class="focus-calendar-legend text-xs text-text-muted"><span><Tv size={14} />TV episode</span><span><Film size={14} />Movie</span><span><CircleCheck size={14} />Watched</span></div>
</div>
