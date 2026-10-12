<script lang="ts">
  import { onMount } from "svelte";
  import { Award, Trophy, RefreshCw, ChevronLeft, ChevronDown, Check, Download } from "lucide-svelte";
  import { exportPredictionsAsImage } from "../../utils/predictionExport";
  import { predictionOutcome } from "../../utils/awardPredictions";
  import { formatDate, formatDateTime } from "../../utils/dateFormat";
  import {
    getAwardType,
    setAwardType,
    getCeremonies,
    getSelectedCeremony,
    isLoading,
    isSyncing,
    loadCeremonies,
    selectCeremony,
    clearSelectedCeremony,
    refreshAwards,
    getPrediction,
    getPredictionsMap,
    getScore,
    getLastSync,
    getAwardsSyncReport,
    setPrediction,
    clearPrediction,
    type AwardType,
    type CeremonySummary,
  } from "../../stores/awards.svelte";
  import { showSuccess, showError } from "../../stores/toast.svelte";

  type SubTab = "predict" | "history";
  let subTab = $state<SubTab>("history");
  let refreshMenuOpen = $state(false);

  function relativeTime(iso: string): string {
    const then = new Date(iso).getTime();
    if (isNaN(then)) return "";
    const s = Math.max(0, Math.floor((Date.now() - then) / 1000));
    if (s < 60) return "just now";
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m ago`;
    const h = Math.floor(m / 60);
    if (h < 24) return `${h}h ago`;
    return `${Math.floor(h / 24)}d ago`;
  }

  const detail = $derived(getSelectedCeremony());
  const pastCeremonies = $derived(getCeremonies().filter((c) => c.status === "past"));
  const predictionHistory = $derived(pastCeremonies.filter((c) => c.prediction_count > 0));
  // Only the single most recent open ceremony is predictable (ceremonies are
  // returned newest-first), not a list of every non-past one.
  const upcomingCeremony = $derived(
    getCeremonies().find((c) => c.status === "nominated")
      ?? getCeremonies().find((c) => c.status === "upcoming") ?? null,
  );

  onMount(() => {
    loadCeremonies();
  });

  async function switchAward(t: AwardType) {
    await setAwardType(t);
  }

  async function doRefresh(full: boolean, retryFailed = false) {
    try {
      const s = await refreshAwards(full, retryFailed);
      if (s.errors.length > 0) {
        showError(`Synced with ${s.errors.length} issue(s); some ceremonies were skipped.`);
      } else {
        showSuccess(`Awards updated — ${s.ceremonies} ceremonies, ${s.winners} winners.`);
      }
    } catch {
      showError("Failed to sync awards from Wikipedia.");
    }
  }

  function open(c: CeremonySummary) {
    selectCeremony(c.id);
  }

  function togglePick(categoryId: number, nomineeId: number) {
    const ceremony = getSelectedCeremony();
    const category = ceremony?.categories.find((c) => c.id === categoryId);
    if (subTab !== "predict" || ceremony?.status !== "nominated"
      || category?.nominees.some((n) => n.is_winner === true)) return;
    if (getPrediction(categoryId) === nomineeId) {
      clearPrediction(categoryId);
    } else {
      setPrediction(categoryId, nomineeId);
    }
  }

  const pickCount = $derived(
    detail ? detail.categories.filter((c) => getPrediction(c.id) != null).length : 0,
  );

  function exportPicks() {
    if (detail) exportPredictionsAsImage(detail, getPredictionsMap());
  }

</script>

<div class="focus-page focus-awards-page">
  <header class="focus-page-heading">
    <div class="focus-page-bar">
      <h1>Awards</h1>
      {#if detail}
        <button onclick={clearSelectedCeremony} class="flex items-center gap-1 text-sm text-text-muted hover:text-text">
          <ChevronLeft class="w-4 h-4" /> Back to {subTab === "predict" ? "Predict" : "History"}
        </button>
        <h2 class="text-base font-semibold">{detail.name}</h2>
        <span class="text-xs text-text-muted">
          {subTab === "predict" && detail.status === "nominated"
            ? "Nominations open — choose a nominee"
            : subTab === "predict" ? "Your prediction results" : "Winners & nominees"}{#if detail.ceremony_date}
            · {formatDate(detail.ceremony_date)}{/if}
        </span>
      {:else}
        <div class="focus-segment" role="group" aria-label="Awards view">
          {#each [{ v: "history", l: "History" }, { v: "predict", l: "Predict" }] as t}
            <button onclick={() => (subTab = t.v as SubTab)} aria-pressed={subTab === t.v} class="px-3 py-1.5 text-sm font-medium">
              {t.l}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </header>
  <!-- Award toggle + refresh -->
  <div class="focus-toolbar flex flex-wrap items-center justify-between gap-3">
    <div class="focus-segment" role="group" aria-label="Award type">
      {#each [{ v: "oscars", l: "Oscars" }, { v: "emmys", l: "Emmys" }] as opt}
        <button
          onclick={() => switchAward(opt.v as AwardType)}
          aria-pressed={getAwardType() === opt.v}
          class="px-4 py-1.5 text-sm rounded-md transition-colors {getAwardType() === opt.v
            ? 'bg-surface text-text'
            : 'text-text-muted hover:text-text'}"
        >
          {opt.l}
        </button>
      {/each}
    </div>
    <div class="flex flex-wrap items-center gap-3">
      {#if getLastSync()}
        <span class="text-xs text-text-muted">Last complete sync {relativeTime(getLastSync()!)}</span>
      {/if}
      {#if getAwardsSyncReport()}
        {@const report = getAwardsSyncReport()!}
        <span class="text-xs text-text-muted">Last attempt: {formatDateTime(report.attempted_at)}</span>
        {#if report.errors.length}
          <details class="focus-awards-sync text-xs text-red-400">
            <summary>{report.errors.length} sync issues</summary>
            <ul>{#each report.errors as error}<li>{error}</li>{/each}</ul>
          </details>
          {#if report.failed_ceremonies?.length}
            <button class="text-xs text-red-400 underline" disabled={isSyncing()} onclick={() => doRefresh(false, true)}>Retry failed ceremonies</button>
          {/if}
        {/if}
      {/if}
      <div class="relative flex">
        <button
          onclick={() => doRefresh(false)}
          disabled={isSyncing()}
          class="flex items-center gap-1.5 pl-3 pr-2 py-1.5 text-sm rounded-l-lg bg-surface hover:bg-surface-hover text-text disabled:opacity-50"
          title="Refresh the last 5 years from Wikipedia"
        >
          <RefreshCw class="w-4 h-4 {isSyncing() ? 'animate-spin' : ''}" />
          Refresh
        </button>
        <button
          onclick={() => (refreshMenuOpen = !refreshMenuOpen)}
          disabled={isSyncing()}
          aria-label="More refresh options"
          class="px-1.5 py-1.5 rounded-r-lg bg-surface hover:bg-surface-hover text-text border-l border-border disabled:opacity-50"
        >
          <ChevronDown class="w-4 h-4" />
        </button>
        {#if refreshMenuOpen}
          <button
            type="button"
            class="fixed inset-0 z-0 cursor-default"
            tabindex="-1"
            aria-label="Close menu"
            onclick={() => (refreshMenuOpen = false)}
          ></button>
          <div
            class="absolute right-0 top-full mt-1 w-48 bg-surface border border-border rounded-lg shadow-lg z-10 overflow-hidden"
          >
            <button
              onclick={() => {
                refreshMenuOpen = false;
                doRefresh(true);
              }}
              class="w-full text-left px-3 py-2 text-sm hover:bg-surface-hover"
            >
              Full refresh (20 years)
            </button>
          </div>
        {/if}
      </div>
      {#if detail && subTab === "history" && pickCount > 0}
        <button onclick={() => (subTab = "predict")} class="focus-button" title="Open your saved picks and their results for this ceremony">
          <Check class="w-4 h-4" /> View your predictions
        </button>
      {/if}
      {#if detail && subTab === "predict" && pickCount > 0}
        <button onclick={exportPicks} class="focus-button" title="Save your picks as a shareable image">
          <Download class="w-4 h-4" /> Export picks
        </button>
        {#if getScore()}
          <div class="focus-awards-score flex flex-wrap items-center gap-2 text-sm">
            <strong class="text-accent">{getScore()!.correct}/{getScore()!.total}</strong>
            <span class="text-text-muted">correct picks</span>
            {#if pickCount > getScore()!.total}
              <span class="text-text-muted">· {pickCount - getScore()!.total} pending</span>
            {/if}
          </div>
        {/if}
      {/if}
    </div>
  </div>

  {#if detail}
    {@const showPicks = subTab === "predict"}
    {@const isOpen = showPicks && detail.status === "nominated"}
    <div class="space-y-4">
      {#each detail.categories as cat (cat.id)}
        {@const resultKnown = cat.nominees.some((n) => n.is_winner === true)}
        {@const canPick = isOpen && !resultKnown}
        <section class="focus-panel focus-awards-category bg-surface rounded-xl p-5 border border-border">
          <h3 class="font-medium mb-2">{cat.name}</h3>
          {#if detail.status === "past" && !resultKnown}
            <p class="text-xs text-text-muted mb-2">Result pending — a winner has not been confirmed in the synced data.</p>
          {/if}
          <ul class="space-y-1">
            {#each cat.nominees as nom (nom.id)}
              {@const picked = showPicks && getPrediction(cat.id) === nom.id}
              {@const outcome = picked ? predictionOutcome(cat, nom.id) : null}
              <li>
                <button
                  type="button"
                  disabled={!canPick}
                  onclick={() => togglePick(cat.id, nom.id)}
                  class="focus-nominee w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-left transition-colors
                    {nom.is_winner ? 'bg-available/10' : ''}
                    {picked && outcome === null ? 'ring-1 ring-accent bg-accent/10' : ''}
                    {picked && outcome === 'miss' ? 'bg-red-500/10' : ''}
                    {canPick ? 'hover:bg-surface-hover cursor-pointer' : 'cursor-default'}"
                >
                  {#if nom.is_winner}
                    <Trophy class="w-4 h-4 text-available flex-shrink-0" />
                  {:else if picked}
                    <Check class="w-4 h-4 text-accent flex-shrink-0" />
                  {:else}
                    <span class="w-4 flex-shrink-0"></span>
                  {/if}
                  <span
                    class={nom.is_winner
                      ? "font-medium text-text"
                      : picked
                        ? "text-text"
                        : "text-text-muted"}
                  >
                    {nom.title}
                  </span>
                  {#if picked}
                    <span
                      class="ml-auto text-xs flex-shrink-0 {outcome === 'win' ? 'text-available'
                        : outcome === 'miss' ? 'text-red-400' : 'text-accent'}"
                    >
                      your pick{outcome === 'win' ? " ✓" : outcome === 'miss' ? " ✗" : " · pending"}
                    </span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {:else}
    {#if isLoading()}
      <div class="text-center text-text-muted py-12">Loading…</div>
    {:else if getCeremonies().length === 0}
      <div class="text-center text-text-muted py-12">
        <Award class="w-10 h-10 mx-auto mb-3 opacity-40" />
        <p>No awards data yet.</p>
        <button onclick={() => doRefresh(true)} class="mt-3 text-accent hover:underline">
          Pull from Wikipedia
        </button>
      </div>
    {:else if subTab === "history"}
      <div class="focus-ceremony-grid grid grid-cols-2 sm:grid-cols-3 gap-4">
        {#each pastCeremonies as c (c.id)}
          <button
            onclick={() => open(c)}
            class="focus-panel focus-ceremony-card text-left p-5 rounded-xl bg-surface hover:bg-surface-hover border border-border transition-colors"
          >
            <div class="font-medium">{c.year}</div>
            <div class="text-xs text-text-muted">{c.name}</div>
          </button>
        {/each}
      </div>
    {:else}
      <h2 class="text-lg font-semibold mb-3">Upcoming predictions</h2>
      {#if !upcomingCeremony}
        <div class="text-center text-text-muted py-12">
          No upcoming ceremony right now.
        </div>
      {:else if upcomingCeremony.status === "upcoming"}
        <!-- Nominations not out yet — informational, not pickable. -->
        <div class="focus-panel w-full p-5 rounded-xl bg-surface border border-border">
          <div class="font-medium text-lg">{upcomingCeremony.name}</div>
          {#if upcomingCeremony.nominations_date}
            <div class="text-sm text-text-muted">
              Nominations expected {formatDate(upcomingCeremony.nominations_date)}
            </div>
          {/if}
          {#if upcomingCeremony.ceremony_date}
            <div class="text-sm text-text-muted">
              Ceremony {formatDate(upcomingCeremony.ceremony_date)}
            </div>
          {/if}
          <div class="text-xs text-text-muted mt-2">
            Picks open once nominations are announced.
          </div>
        </div>
      {:else}
        <button
          onclick={() => open(upcomingCeremony)}
          class="focus-panel focus-ceremony-card w-full text-left p-5 rounded-xl bg-surface hover:bg-surface-hover border border-border transition-colors"
        >
          <div class="font-medium text-lg">{upcomingCeremony.name}</div>
          {#if upcomingCeremony.ceremony_date}
            <div class="text-sm text-text-muted">
              Ceremony {formatDate(upcomingCeremony.ceremony_date)}
            </div>
          {/if}
          <div class="text-sm text-accent mt-1">Make your picks →</div>
        </button>
      {/if}
      <section class="mt-8">
        <h2 class="text-lg font-semibold mb-1">Prediction history</h2>
        <p class="text-sm text-text-muted mb-4">Your saved picks and results from past ceremonies.</p>
        {#if predictionHistory.length === 0}
          <p class="text-sm text-text-muted py-4">Completed ceremonies you made picks for will appear here.</p>
        {:else}
          <div class="focus-ceremony-grid grid grid-cols-2 sm:grid-cols-3 gap-4">
            {#each predictionHistory as c (c.id)}
              <button
                onclick={() => open(c)}
                class="focus-panel focus-ceremony-card text-left p-5 rounded-xl bg-surface hover:bg-surface-hover border border-border transition-colors"
              >
                <div class="font-medium">{c.year}</div>
                <div class="text-xs text-text-muted">{c.name}</div>
                <div class="text-sm text-accent mt-2">{c.prediction_count} saved {c.prediction_count === 1 ? "pick" : "picks"} →</div>
              </button>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  {/if}
</div>

<style>
  .focus-awards-page { max-width: 76rem; }
  .focus-awards-sync { position: relative; }
  .focus-awards-sync summary { cursor: pointer; }
  .focus-awards-sync ul { position: absolute; right: 0; top: 100%; z-index: 10; width: min(28rem, 80vw); max-height: 60vh; overflow: auto; overflow-wrap: anywhere; margin-top: .5rem; padding: .8rem 1rem; border: 1px solid var(--color-border); border-radius: .6rem; background: var(--color-surface); box-shadow: 0 10px 30px #0004; }
  .focus-awards-score { padding: .35rem .65rem; background: color-mix(in srgb, var(--color-accent) 8%, var(--color-surface)); border: 1px solid color-mix(in srgb, var(--color-accent) 20%, var(--color-border)); border-radius: .5rem; }
  .focus-awards-category h3 { font-size: 1rem; line-height: 1.5; margin-bottom: 0.875rem; }
  .focus-nominee { font-size: 0.875rem; }
  .focus-nominee:disabled { opacity: 1; }
  .focus-ceremony-card { min-height: 6rem; }
  .focus-ceremony-card > div:first-child { margin-bottom: 0.5rem; }
  @media (max-width: 1050px) { .focus-ceremony-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } .focus-nominee { flex-wrap: wrap; } }
</style>
