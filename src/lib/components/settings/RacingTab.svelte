<script lang="ts">
  import { Bell, BellOff, Link, ChevronDown, ChevronUp, ChevronRight, RefreshCw, Zap } from "lucide-svelte";
  import {
    getRacingSeries,
    getRacingConfig,
    getRacingRefreshReport,
    toggleSeries,
    updateSeriesColor,
    updateSeriesNotification,
    updateSeriesIcsUrl,
    updateRacingConfig,
    updateAllSeriesLeadTime,
    refreshSingleSeries,
    refreshRacingData,
    isRacingRefreshing,
    getSeriesColor,
    testNotification,
    type RacingSeries,
  } from "../../stores/racing.svelte";

  import { getNotificationSettings } from "../../stores/notifications.svelte";
  import { openSettings } from "../../stores/settings.svelte";
  import { formatDateTime } from "../../utils/dateFormat";

  const isDev = import.meta.env.DEV;

  const categories: Record<string, string> = {
    "open-wheel": "Open Wheel",
    endurance: "Endurance",
    "stock-car": "Stock Car",
    motorcycle: "Motorcycle",
    rally: "Rally",
    touring: "Touring",
    other: "Other",
  };

  const preferredCategories = ["open-wheel", "endurance", "stock-car", "motorcycle", "rally", "touring", "other"];
  const categoryOrder = $derived([...new Set([...preferredCategories, ...getRacingSeries().map(series => series.category)])]);
  let seriesFilter = $state<"followed" | "all">("followed");
  const followedCount = $derived(getRacingSeries().filter(series => series.enabled).length);
  const reminderStatus = $derived.by(() => {
    const config = getRacingConfig();
    const notifications = getNotificationSettings();
    if (!config || !notifications) return "Checking reminder settings…";
    if (!config.notifications_enabled) return "Race reminders are paused.";
    if (!notifications.enabled || !notifications.racing_enabled) return "Race reminders are paused in Notifications settings.";
    return "Race reminders are on for followed series with reminders enabled.";
  });

  let expandedSeries = $state<Set<string>>(new Set());
  let collapsedCategories = $state<Set<string>>(new Set());
  let globalNotifications = $state(true);

  $effect(() => {
    const config = getRacingConfig();
    if (config) {
      globalNotifications = config.notifications_enabled;
    }
  });

  function getSeriesByCategory(): Record<string, RacingSeries[]> {
    const grouped: Record<string, RacingSeries[]> = {};
    for (const cat of categoryOrder) {
      grouped[cat] = getRacingSeries().filter((s) => s.category === cat && (seriesFilter === "all" || s.enabled));
    }
    return grouped;
  }

  function toggleCategory(cat: string) {
    const next = new Set(collapsedCategories);
    if (next.has(cat)) {
      next.delete(cat);
    } else {
      next.add(cat);
    }
    collapsedCategories = next;
  }

  function toggleExpanded(slug: string) {
    const next = new Set(expandedSeries);
    if (next.has(slug)) {
      next.delete(slug);
    } else {
      next.add(slug);
    }
    expandedSeries = next;
  }

  async function handleToggle(slug: string, enabled: boolean) {
    const saved = await toggleSeries(slug, enabled);
    if (saved && enabled) {
      await refreshSingleSeries(slug);
    }
  }

  async function handleColorChange(slug: string, color: string) {
    await updateSeriesColor(slug, color);
  }

  async function handleResetColor(slug: string) {
    await updateSeriesColor(slug, null);
  }

  async function handleNotifyToggle(series: RacingSeries) {
    await updateSeriesNotification(series.slug, !series.notify_enabled, series.notify_minutes);
  }

  async function handleMinutesChange(series: RacingSeries, minutes: number) {
    await updateSeriesNotification(series.slug, series.notify_enabled, minutes);
  }

  async function handleIcsUrlChange(slug: string, url: string) {
    await updateSeriesIcsUrl(slug, url || null);
  }

  async function handleSaveGlobalConfig() {
    await updateRacingConfig(globalNotifications);
  }

  async function handleChangeAllLeadTimes(minutes: number) {
    await updateAllSeriesLeadTime(minutes);
  }

  const minuteOptions = [5, 10, 15, 30, 45, 60, 90, 120];
</script>

<div class="space-y-6">
  {#if getRacingRefreshReport()}
    {@const report = getRacingRefreshReport()!}
    <div class="text-sm text-text-muted space-y-2">
      <p>Last attempt: {formatDateTime(report.attempted_at)} — {report.succeeded} series refreshed.</p>
      <p>Last complete success: {getRacingConfig()?.last_refreshed ? formatDateTime(getRacingConfig()!.last_refreshed!) : "Never"}</p>
      {#each report.failures as failure}
        <div class="text-red-400">{failure.name}: {failure.error}
          <button class="underline ml-2" onclick={() => refreshSingleSeries(failure.slug)}>Retry</button>
        </div>
      {/each}
    </div>
  {/if}
  <!-- Global settings -->
  <div class="space-y-3">
    <h3 class="text-sm font-semibold text-text-muted uppercase tracking-wider">Global Settings</h3>
    <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
      <div class="flex items-center gap-3">
        {#if globalNotifications}
          <Bell class="w-4 h-4 text-accent" />
        {:else}
          <BellOff class="w-4 h-4 text-text-muted" />
        {/if}
        <div>
          <p class="text-sm font-medium text-text">Race reminders</p>
          <p class="text-xs text-text-muted">Get reminded before your followed sessions start</p>
        </div>
      </div>
      <button
        onclick={() => { globalNotifications = !globalNotifications; handleSaveGlobalConfig(); }}
        class="relative w-10 h-5 rounded-full transition-colors {globalNotifications ? 'bg-accent' : 'bg-border'}"
        role="switch" aria-checked={globalNotifications} aria-label="Race reminders"
      >
        <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {globalNotifications ? 'translate-x-5' : ''}"></span>
      </button>
    </div>

    <div class="flex flex-wrap items-center gap-2 text-xs text-text-muted">
      <span aria-live="polite">{reminderStatus}</span>
      <button type="button" onclick={() => openSettings('notifications')} class="text-accent hover:underline">Notifications settings</button>
    </div>
    <div class="flex items-center justify-between p-3 rounded-lg bg-background border border-border">
      <div>
        <p class="text-sm font-medium text-text">Change enabled reminder times</p>
        <p class="text-xs text-text-muted">Set the same lead time for followed series whose reminders are on</p>
      </div>
      <select
        value=""
        aria-label="Lead time for enabled racing reminders"
        onchange={(e) => { const v = (e.target as HTMLSelectElement).value; if (v) { handleChangeAllLeadTimes(parseInt(v)); (e.target as HTMLSelectElement).value = ""; } }}
        class="bg-surface border border-border rounded-lg px-3 py-1.5 text-sm text-text"
      >
        <option value="" disabled>Select...</option>
        {#each minuteOptions as mins}
          <option value={mins}>{mins} min</option>
        {/each}
      </select>
    </div>

    <div class="flex items-center gap-2">
      <button
        onclick={() => refreshRacingData()}
        disabled={isRacingRefreshing()}
        class="flex items-center gap-2 px-4 py-2 rounded-lg bg-accent/10 text-accent hover:bg-accent/20 transition-colors text-sm font-medium disabled:opacity-50"
      >
        <RefreshCw class="w-4 h-4 {isRacingRefreshing() ? 'animate-spin' : ''}" />
        {isRacingRefreshing() ? "Refreshing..." : "Refresh All Data"}
      </button>

      {#if isDev}
        <button
          onclick={() => testNotification()}
          class="flex items-center gap-2 px-4 py-2 rounded-lg bg-yellow-500/10 text-yellow-500 hover:bg-yellow-500/20 transition-colors text-sm font-medium"
        >
          <Zap class="w-4 h-4" />
          Test Notification
        </button>
      {/if}
    </div>
  </div>

  <div class="focus-segment" role="group" aria-label="Racing series filter">
    <button type="button" aria-pressed={seriesFilter === 'followed'} onclick={() => seriesFilter = 'followed'}>Followed ({followedCount})</button>
    <button type="button" aria-pressed={seriesFilter === 'all'} onclick={() => seriesFilter = 'all'}>All series ({getRacingSeries().length})</button>
  </div>
  {#if seriesFilter === 'followed' && !followedCount}
    <p class="text-sm text-text-muted">No followed series. <button type="button" onclick={() => seriesFilter = 'all'} class="text-accent hover:underline">Choose from all series</button></p>
  {/if}
  <!-- Series by category -->
  {#each categoryOrder as category}
    {@const seriesInCat = getSeriesByCategory()[category]}
    {#if seriesInCat && seriesInCat.length > 0}
      <div class="space-y-2">
        <button
          aria-expanded={!collapsedCategories.has(category)}
          onclick={() => toggleCategory(category)}
          class="flex items-center gap-2 text-sm font-semibold text-text-muted uppercase tracking-wider hover:text-text transition-colors w-full text-left"
        >
          {#if collapsedCategories.has(category)}
            <ChevronRight class="w-3.5 h-3.5" />
          {:else}
            <ChevronDown class="w-3.5 h-3.5" />
          {/if}
          {categories[category] || category}
          <span class="text-xs font-normal normal-case">({seriesInCat.length})</span>
        </button>

        {#if !collapsedCategories.has(category)}
        {#each seriesInCat as series}
          <div class="rounded-lg border border-border overflow-hidden">
            <!-- Series row -->
            <div class="flex items-center gap-3 p-3 bg-background">
              <!-- Color dot -->
              <input
                type="color"
                value={getSeriesColor(series)}
                onchange={(e) => handleColorChange(series.slug, (e.target as HTMLInputElement).value)}
                class="w-5 h-5 rounded-full border-0 cursor-pointer bg-transparent"
                title="Change series color"
                style="padding: 0;"
              />

              <!-- Name -->
              <span class="flex-1 text-sm font-medium text-text">{series.name}</span>

              <!-- Notification toggle -->
              {#if series.enabled}
                <button
                  onclick={() => handleNotifyToggle(series)}
                  class="p-1.5 rounded transition-colors {series.notify_enabled
                    ? 'text-accent hover:bg-accent/10'
                    : 'text-text-muted hover:bg-surface-hover'}"
                  aria-label={`Reminders for ${series.name}`} aria-pressed={series.notify_enabled}
                  title={series.notify_enabled ? "Notifications on" : "Notifications off"}
                >
                  {#if series.notify_enabled}
                    <Bell class="w-3.5 h-3.5" />
                  {:else}
                    <BellOff class="w-3.5 h-3.5" />
                  {/if}
                </button>

                <!-- Minutes selector -->
                {#if series.notify_enabled}
                  <select
                    aria-label={`Reminder lead time for ${series.name}`}
                    value={series.notify_minutes}
                    onchange={(e) => handleMinutesChange(series, parseInt((e.target as HTMLSelectElement).value))}
                    class="bg-surface border border-border rounded px-2 py-1 text-xs text-text w-20"
                  >
                    {#each minuteOptions as mins}
                      <option value={mins}>{mins} min</option>
                    {/each}
                  </select>
                {/if}
              {/if}

              <!-- Advanced expand -->
              <button
                onclick={() => toggleExpanded(series.slug)}
                class="p-1.5 rounded hover:bg-surface-hover transition-colors text-text-muted"
                aria-label={`Advanced options for ${series.name}`} aria-expanded={expandedSeries.has(series.slug)}
                title="Advanced options"
              >
                {#if expandedSeries.has(series.slug)}
                  <ChevronUp class="w-3.5 h-3.5" />
                {:else}
                  <ChevronDown class="w-3.5 h-3.5" />
                {/if}
              </button>

              <!-- Enable toggle -->
              <button
                onclick={() => handleToggle(series.slug, !series.enabled)}
                class="relative w-10 h-5 rounded-full transition-colors {series.enabled ? 'bg-accent' : 'bg-border'}"
                role="switch" aria-checked={series.enabled}
                aria-label="Follow {series.name}"
              >
                <span class="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform {series.enabled ? 'translate-x-5' : ''}"></span>
              </button>
            </div>

            <!-- Advanced section -->
            {#if expandedSeries.has(series.slug)}
              <div class="p-3 bg-surface border-t border-border space-y-2">
                <div>
                  <label class="text-xs text-text-muted flex items-center gap-1 mb-1">
                    <Link class="w-3 h-3" />
                    Custom ICS URL (leave empty for default)
                  </label>
                  <input
                    type="url"
                    value={series.custom_ics_url || ""}
                    onchange={(e) => handleIcsUrlChange(series.slug, (e.target as HTMLInputElement).value)}
                    placeholder={series.ics_url}
                    class="w-full bg-background border border-border rounded-lg px-3 py-1.5 text-xs text-text placeholder:text-text-muted/50"
                  />
                </div>
                {#if series.custom_color}
                  <button
                    onclick={() => handleResetColor(series.slug)}
                    class="text-xs text-text-muted hover:text-text transition-colors"
                  >
                    Reset to default color ({series.color})
                  </button>
                {/if}
                {#if series.enabled}
                  <button
                    onclick={() => refreshSingleSeries(series.slug)}
                    class="flex items-center gap-1.5 text-xs text-accent hover:text-accent/80 transition-colors"
                  >
                    <RefreshCw class="w-3 h-3" />
                    Refresh this series
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
        {/if}
      </div>
    {/if}
  {/each}
</div>
