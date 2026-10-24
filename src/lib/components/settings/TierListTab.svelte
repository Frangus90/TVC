<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { GripVertical, Trash2, Plus, RotateCcw, ArrowUp, ArrowDown, Loader2 } from "lucide-svelte";
  import TiermakerImport from "./TiermakerImport.svelte";
  import { getTiers, getTierPreset, loadTiers, loadTierPreset, applyTierConfiguration, getTierDraft, saveTierDraft, clearTierDraft, type TierDefinition, type TierPreset } from "../../stores/tiers.svelte";
  import { openConfirmDialog } from "../../stores/confirmDialog.svelte";
  import { showSuccess } from "../../stores/toast.svelte";
  import { getGeneralPreferences, isGeneralLoaded, loadGeneralPreferences, updateGeneralPreferences, type TierListType } from "../../stores/general.svelte";

  let defaultListLoading = $state(true);
  let defaultListSaving = $state(false);
  let defaultListError = $state<string | null>(null);
  let defaultListSaved = $state(false);

  async function loadDefaultList() {
    defaultListLoading = true;
    defaultListError = null;
    try { await loadGeneralPreferences(); }
    catch (cause) { defaultListError = `Could not load the default tier list: ${String(cause)}`; }
    finally { defaultListLoading = false; }
  }

  async function saveDefaultList(select: HTMLSelectElement) {
    if (defaultListSaving) return;
    const value = select.value as TierListType;
    select.value = getGeneralPreferences().default_tier_list;
    defaultListSaving = true;
    defaultListSaved = false;
    defaultListError = null;
    try {
      await updateGeneralPreferences({ default_tier_list: value });
      defaultListSaved = true;
    } catch (cause) {
      defaultListError = `Could not save the default tier list: ${String(cause)}`;
    } finally {
      select.value = getGeneralPreferences().default_tier_list;
      defaultListSaving = false;
    }
  }

  interface DraftTier extends TierDefinition { key: number }
  let draft = $state<DraftTier[]>([]);
  let draftPreset = $state<TierPreset>("custom");
  let baseline = $state("");
  let loading = $state(true);
  let applying = $state(false);
  let error = $state<string | null>(null);
  let newName = $state("");
  let newColor = $state("#8b5cf6");
  let nextKey = 0;
  let draggedKey = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);
  let importOpen = $state(false);
  const definitions = $derived(draft.map(({ name, color }) => ({ name: name.trim(), color })));
  const dirty = $derived(JSON.stringify({ preset: draftPreset, tiers: definitions }) !== baseline);
  const valid = $derived(draft.length > 0 && draft.length <= 100 && draft.every(tier => tier.name.trim().length > 0 && [...tier.name.trim()].length <= 120));

  function snapshotCurrent() {
    draftPreset = ["10-star", "5-star"].includes(getTierPreset()) ? getTierPreset() as TierPreset : "custom";
    draft = getTiers().map(tier => ({ name: tier.name, color: tier.color, key: ++nextKey }));
    baseline = JSON.stringify({ preset: draftPreset, tiers: draft.map(({ name, color }) => ({ name: name.trim(), color })) });
    newName = "";
    error = null;
    clearTierDraft();
  }

  onMount(() => {
    let active = true;
    void loadDefaultList();
    const savedDraft = getTierDraft();
    Promise.all([loadTiers(), loadTierPreset()]).then(() => {
      if (active) {
        snapshotCurrent();
        if (savedDraft) {
          draftPreset = savedDraft.preset;
          draft = savedDraft.tiers.map(tier => ({ ...tier, key: ++nextKey }));
        }
        loading = false;
      }
    });
    return () => { active = false; };
  });

  // Import can change the saved tiers. Refresh a clean editor; retain a dirty draft.
  $effect(() => {
    getTiers(); getTierPreset();
    untrack(() => { if (!loading && !applying && !dirty) snapshotCurrent(); });
  });

  $effect(() => {
    if (!loading && !applying) {
      if (dirty) saveTierDraft(draftPreset, definitions);
      else clearTierDraft();
    }
  });

  function setPreset(preset: TierPreset) {
    draftPreset = preset;
    if (preset === "custom") return;
    const names = preset === "10-star"
      ? ["Masterpiece", "Excellent", "Great", "Good", "Solid", "Average", "Below Average", "Poor", "Bad", "Terrible"]
      : ["Excellent", "Great", "Good", "Average", "Poor"];
    draft = names.map(name => ({ name, color: "", key: ++nextKey }));
    error = null;
  }

  function editTier(key: number, values: Partial<TierDefinition>) {
    draft = draft.map(tier => tier.key === key ? { ...tier, ...values } : tier);
    draftPreset = "custom";
  }

  function addTier() {
    if (!newName.trim() || draft.length >= 100) return;
    draft = [...draft, { name: newName.trim(), color: newColor, key: ++nextKey }];
    draftPreset = "custom";
    newName = "";
  }

  function deleteTier(key: number) {
    draft = draft.filter(tier => tier.key !== key);
    draftPreset = "custom";
  }

  function moveTier(from: number, to: number) {
    if (to < 0 || to >= draft.length || from === to) return;
    const next = [...draft];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    draft = next;
    draftPreset = "custom";
  }

  function dropTier(event: DragEvent, index: number) {
    event.preventDefault();
    const from = draft.findIndex(tier => tier.key === draggedKey);
    if (from >= 0) moveTier(from, index);
    draggedKey = null;
    dragOverIndex = null;
  }

  async function applyDraft() {
    if (!dirty || !valid || applying) return;
    applying = true;
    error = null;
    const preset = draftPreset;
    const tiers = definitions.map(tier => ({ ...tier }));
    try {
      const confirmed = await openConfirmDialog({
        title: "Apply new tier layout?",
        message: "Applying replaces your current tiers and clears all tier assignments and ranking order for BOTH movies and TV shows, including archived titles. You will need to rank them again. Your saved titles, watched history and schedules are kept. This cannot be undone.",
        type: "warning", confirmLabel: "Apply and clear rankings", cancelLabel: "Keep current tiers",
      });
      if (!confirmed) return;
      const result = await applyTierConfiguration(preset, tiers);
      snapshotCurrent();
      showSuccess(`Tier layout applied. ${result.cleared_movies} movies and ${result.cleared_shows} shows are ready to rank again.`);
    } catch (cause) {
      error = `Could not apply your tier layout: ${cause instanceof Error ? cause.message : String(cause)}`;
    } finally { applying = false; }
  }
</script>

<div class="space-y-4">
  <div class="border-b border-border pb-4 space-y-2">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <label for="default-tier-list" class="font-semibold">Default tier list</label>
        <p id="default-tier-list-help" class="text-sm text-text-muted">Choose which list opens first. Saves automatically and keeps your rankings.</p>
      </div>
      <select id="default-tier-list" value={getGeneralPreferences().default_tier_list} aria-describedby="default-tier-list-help"
        disabled={defaultListLoading || defaultListSaving || !isGeneralLoaded()} onchange={(event) => saveDefaultList(event.currentTarget)}
        class="bg-background border border-border rounded-lg px-3 py-2 text-sm focus:border-accent focus:outline-none disabled:opacity-50">
        <option value="shows">Shows</option>
        <option value="movies">Movies</option>
      </select>
    </div>
    {#if defaultListError}
      <p role="alert" class="text-sm text-error">{defaultListError}</p>
      {#if !isGeneralLoaded()}<button type="button" onclick={loadDefaultList} disabled={defaultListLoading} class="focus-button">Retry</button>{/if}
    {:else if defaultListLoading || defaultListSaving || defaultListSaved}
      <p role="status" class="text-xs text-text-muted">{defaultListLoading ? 'Loading…' : defaultListSaving ? 'Saving…' : 'Default tier list saved.'}</p>
    {/if}
  </div>

  <div class="focus-panel p-4 space-y-3">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h2 class="font-semibold">Tier layout draft</h2>
        <p class="text-sm text-text-muted">Changes here stay in this draft until you apply them.</p>
      </div>
      <span class="text-xs {dirty ? 'text-warning' : 'text-text-muted'}" aria-live="polite">{loading ? "Loading…" : dirty ? "Unapplied changes" : "Matches your current layout"}</span>
    </div>
    <div class="flex flex-wrap items-center gap-2" role="group" aria-label="Draft tier preset">
      <span class="text-sm text-text-muted mr-1">Start from</span>
      {#each [{ id: "10-star", label: "10 Stars" }, { id: "5-star", label: "5 Stars" }, { id: "custom", label: "Custom" }] as preset}
        <button type="button" disabled={loading || applying} onclick={() => setPreset(preset.id as TierPreset)} aria-pressed={draftPreset === preset.id}
          class="px-3 py-1.5 text-sm rounded-lg border transition-colors disabled:opacity-50 {draftPreset === preset.id ? 'border-accent bg-accent/10 text-accent' : 'border-border hover:bg-surface-hover'}">{preset.label}</button>
      {/each}
    </div>
  </div>

  {#if error}<p role="alert" class="text-sm text-error">{error}</p>{/if}
  {#if loading}
    <p class="text-sm text-text-muted">Loading your current tiers…</p>
  {:else}
    <div class="flex justify-between items-center gap-3 text-sm"><span class="font-medium">Draft tiers</span><span class="text-text-muted">Top is best · {draft.length} tiers</span></div>
    <ul class="border border-border rounded-lg overflow-hidden" aria-label="Draft tier order">
      {#each draft as tier, index (tier.key)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <li class="flex flex-wrap items-center gap-2 p-3 border-b border-border last:border-b-0 {dragOverIndex === index ? 'bg-accent/10' : 'bg-surface'}"
          ondragover={(event) => { event.preventDefault(); if (!applying) dragOverIndex = index; }} ondragleave={() => dragOverIndex = null} ondrop={(event) => { if (!applying) dropTier(event, index); }}>
          <button type="button" disabled={applying} draggable={!applying} aria-label={`Drag ${tier.name || 'unnamed tier'} to reorder`} title="Drag to reorder"
            ondragstart={(event) => { draggedKey = tier.key; if (event.dataTransfer) { event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("text/plain", String(tier.key)); } }}
            ondragend={() => { draggedKey = null; dragOverIndex = null; }} class="p-1 text-text-muted cursor-grab"><GripVertical size={16} /></button>
          <span class="w-5 text-xs text-text-muted">{index + 1}</span>
          <input type="text" value={tier.name} disabled={applying} maxlength="120" aria-label={`Tier ${index + 1} name`}
            oninput={(event) => editTier(tier.key, { name: event.currentTarget.value })} class="flex-1 min-w-[8rem] bg-background border border-border rounded-lg px-3 py-1.5 text-sm" />
          <input type="color" value={tier.color || '#8b5cf6'} disabled={applying} aria-label={`Tier ${index + 1} colour`}
            oninput={(event) => editTier(tier.key, { color: event.currentTarget.value })} class="w-8 h-8 rounded border border-border bg-transparent" />
          {#if tier.color}<button type="button" disabled={applying} onclick={() => editTier(tier.key, { color: "" })} class="text-xs text-text-muted hover:text-text">Clear colour</button>{/if}
          <button type="button" disabled={applying || index === 0} onclick={() => moveTier(index, index - 1)} aria-label={`Move ${tier.name || 'tier'} up`} class="p-1.5 rounded hover:bg-surface-hover disabled:opacity-30"><ArrowUp size={16} /></button>
          <button type="button" disabled={applying || index === draft.length - 1} onclick={() => moveTier(index, index + 1)} aria-label={`Move ${tier.name || 'tier'} down`} class="p-1.5 rounded hover:bg-surface-hover disabled:opacity-30"><ArrowDown size={16} /></button>
          <button type="button" disabled={applying} onclick={() => deleteTier(tier.key)} aria-label={`Delete ${tier.name || 'tier'} from draft`} class="p-1.5 rounded text-text-muted hover:text-error hover:bg-surface-hover"><Trash2 size={16} /></button>
        </li>
      {:else}<li class="p-4 text-sm text-text-muted">Add at least one tier before applying.</li>{/each}
    </ul>
    <div class="flex flex-wrap items-center gap-2">
      <input type="text" bind:value={newName} maxlength="120" disabled={applying || draft.length >= 100} placeholder="New tier name" aria-label="New tier name" onkeydown={(event) => { if (event.key === 'Enter') addTier(); }} class="flex-1 min-w-[10rem] bg-background border border-border rounded-lg px-3 py-2 text-sm" />
      <input type="color" bind:value={newColor} disabled={applying} aria-label="New tier colour" class="w-8 h-8 rounded border border-border bg-transparent" />
      <button type="button" disabled={applying || !newName.trim() || draft.length >= 100} onclick={addTier} class="focus-button"><Plus size={16} />Add to draft</button>
    </div>
    <div class="focus-panel p-4 space-y-3">
      <p class="text-sm text-text-muted">Applying clears the current rankings for both movies and TV shows. Your titles and watched history stay saved. Your draft stays while you switch pages. Closing TVC discards unapplied changes.</p>
      <div class="flex flex-wrap items-center gap-2">
        <button type="button" onclick={applyDraft} disabled={!dirty || !valid || applying} class="focus-button focus-primary">{#if applying}<Loader2 size={16} class="animate-spin" />{/if}Apply layout</button>
        <button type="button" onclick={snapshotCurrent} disabled={!dirty || applying} class="focus-button">Discard draft</button>
        <button type="button" onclick={() => setPreset('10-star')} disabled={applying} class="focus-button"><RotateCcw size={15} />Reset draft to 10 Stars</button>
      </div>
    </div>
  {/if}

  <div class="pt-4 border-t border-border">
    <button type="button" class="focus-button" aria-expanded={importOpen} disabled={applying} onclick={() => importOpen = !importOpen}>Import from Tiermaker</button>
    {#if importOpen}
      <div class="mt-4">
        {#if dirty}<p class="text-sm text-warning mb-3">This import uses your saved tiers, independently of the unapplied draft above.</p>{/if}
        <TiermakerImport />
      </div>
    {/if}
  </div>
</div>
