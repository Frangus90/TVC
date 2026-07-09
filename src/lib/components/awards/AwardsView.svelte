<script lang="ts">
  import { onMount } from "svelte";
  import { Award, Trophy, RefreshCw, ChevronLeft } from "lucide-svelte";
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
    type AwardType,
    type CeremonySummary,
  } from "../../stores/awards.svelte";
  import { showSuccess, showError } from "../../stores/toast.svelte";

  type SubTab = "predict" | "history";
  let subTab = $state<SubTab>("history");

  const detail = $derived(getSelectedCeremony());
  const pastCeremonies = $derived(getCeremonies().filter((c) => c.status === "past"));
  const openCeremonies = $derived(getCeremonies().filter((c) => c.status !== "past"));

  onMount(() => {
    loadCeremonies();
  });

  async function switchAward(t: AwardType) {
    await setAwardType(t);
  }

  async function doRefresh(full: boolean) {
    try {
      const s = await refreshAwards(full);
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
</script>

<div class="max-w-4xl mx-auto">
  <!-- Award toggle + refresh -->
  <div class="flex items-center justify-between mb-4">
    <div class="flex bg-background rounded-lg p-1">
      {#each [{ v: "oscars", l: "Oscars" }, { v: "emmys", l: "Emmys" }] as opt}
        <button
          onclick={() => switchAward(opt.v as AwardType)}
          class="px-4 py-1.5 text-sm rounded-md transition-colors {getAwardType() === opt.v
            ? 'bg-surface text-text'
            : 'text-text-muted hover:text-text'}"
        >
          {opt.l}
        </button>
      {/each}
    </div>
    <div class="flex items-center gap-3">
      <button
        onclick={() => doRefresh(true)}
        disabled={isSyncing()}
        class="text-xs text-text-muted hover:text-text disabled:opacity-50"
        title="Re-pull 20 years of history from Wikipedia"
      >
        Full refresh
      </button>
      <button
        onclick={() => doRefresh(false)}
        disabled={isSyncing()}
        class="flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-lg bg-surface hover:bg-surface-hover text-text disabled:opacity-50"
      >
        <RefreshCw class="w-4 h-4 {isSyncing() ? 'animate-spin' : ''}" />
        Refresh
      </button>
    </div>
  </div>

  {#if detail}
    <!-- Ceremony detail -->
    <button
      onclick={clearSelectedCeremony}
      class="flex items-center gap-1 text-sm text-text-muted hover:text-text mb-3"
    >
      <ChevronLeft class="w-4 h-4" /> Back
    </button>
    <h2 class="text-xl font-semibold">{detail.name}</h2>
    <p class="text-sm text-text-muted mb-4">
      {detail.status === "past"
        ? "Winners & nominees"
        : "Nominations — winners not yet announced"}
    </p>
    <div class="space-y-4">
      {#each detail.categories as cat (cat.id)}
        <div class="bg-surface rounded-lg p-3 border border-border">
          <h3 class="font-medium mb-2">{cat.name}</h3>
          <ul class="space-y-1">
            {#each cat.nominees as nom (nom.id)}
              <li
                class="flex items-start gap-2 px-2 py-1 rounded {nom.is_winner
                  ? 'bg-green-500/10'
                  : ''}"
              >
                {#if nom.is_winner}
                  <Trophy class="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
                {:else}
                  <span class="w-4 flex-shrink-0"></span>
                {/if}
                <span class={nom.is_winner ? "font-medium text-text" : "text-text-muted"}>
                  {nom.title}
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  {:else}
    <!-- Sub-tabs -->
    <div class="flex gap-4 border-b border-border mb-4">
      {#each [{ v: "history", l: "History" }, { v: "predict", l: "Predict" }] as t}
        <button
          onclick={() => (subTab = t.v as SubTab)}
          class="pb-2 text-sm font-medium border-b-2 -mb-px {subTab === t.v
            ? 'border-accent text-accent'
            : 'border-transparent text-text-muted hover:text-text'}"
        >
          {t.l}
        </button>
      {/each}
    </div>

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
      <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
        {#each pastCeremonies as c (c.id)}
          <button
            onclick={() => open(c)}
            class="text-left p-3 rounded-lg bg-surface hover:bg-surface-hover border border-border transition-colors"
          >
            <div class="font-medium">{c.year}</div>
            <div class="text-xs text-text-muted">{c.name}</div>
          </button>
        {/each}
      </div>
    {:else if openCeremonies.length === 0}
      <div class="text-center text-text-muted py-12">
        No upcoming ceremony with open nominations right now.
      </div>
    {:else}
      <div class="space-y-2">
        {#each openCeremonies as c (c.id)}
          <button
            onclick={() => open(c)}
            class="w-full text-left p-3 rounded-lg bg-surface hover:bg-surface-hover border border-border transition-colors"
          >
            <div class="font-medium">{c.name}</div>
            <div class="text-xs text-text-muted">Nominations open — make your picks</div>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>
