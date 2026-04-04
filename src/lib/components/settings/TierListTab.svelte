<script lang="ts">
  import { GripVertical, Trash2, Plus, RotateCcw } from "lucide-svelte";
  import TiermakerImport from "./TiermakerImport.svelte";
  import {
    getTiers,
    getTierPreset,
    loadTiers,
    loadTierPreset,
    createTier,
    updateTierInfo,
    deleteTier,
    reorderTiers,
    applyPreset,
    type Tier,
  } from "../../stores/tiers.svelte";

  // Load data on mount
  $effect(() => {
    loadTiers();
    loadTierPreset();
  });

  const tiers = $derived(getTiers());
  const preset = $derived(getTierPreset());

  // Editing state
  let editingTierId = $state<number | null>(null);
  let editName = $state("");
  let editColor = $state("");

  // New tier state
  let showNewTierForm = $state(false);
  let newTierName = $state("");
  let newTierColor = $state("");

  // Drag reorder state
  let draggedTierId = $state<number | null>(null);
  let dragOverIndex = $state<number | null>(null);

  // Confirmation state
  let pendingPreset = $state<string | null>(null);
  let pendingDeleteId = $state<number | null>(null);
  let pendingReset = $state(false);

  // Color presets for quick selection
  const colorPresets = [
    "#ef4444", "#f97316", "#eab308", "#22c55e", "#06b6d4",
    "#3b82f6", "#8b5cf6", "#ec4899", "#f43f5e", "#14b8a6",
  ];

  // Start editing a tier
  function startEdit(tier: Tier) {
    editingTierId = tier.id;
    editName = tier.name;
    editColor = tier.color;
  }

  // Save tier edit
  async function saveEdit() {
    if (editingTierId === null || !editName.trim()) return;
    await updateTierInfo(editingTierId, editName.trim(), editColor);
    editingTierId = null;
  }

  // Cancel edit
  function cancelEdit() {
    editingTierId = null;
  }

  // Handle Enter key in edit
  function handleEditKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") saveEdit();
    else if (e.key === "Escape") cancelEdit();
  }

  // Create new tier
  async function handleCreateTier() {
    if (!newTierName.trim()) return;
    await createTier(newTierName.trim(), newTierColor);
    newTierName = "";
    newTierColor = "";
    showNewTierForm = false;
  }

  // Delete tier (with confirmation)
  function requestDelete(tierId: number) {
    pendingDeleteId = tierId;
  }

  async function confirmDelete() {
    if (pendingDeleteId === null) return;
    await deleteTier(pendingDeleteId);
    pendingDeleteId = null;
  }

  // Preset change (with confirmation if items are tiered)
  function requestPresetChange(newPreset: string) {
    if (newPreset === preset) return;
    pendingPreset = newPreset;
  }

  async function confirmPresetChange() {
    if (!pendingPreset) return;
    await applyPreset(pendingPreset);
    pendingPreset = null;
  }

  // Reset to default
  function requestReset() {
    pendingReset = true;
  }

  async function confirmReset() {
    await applyPreset("10-star");
    pendingReset = false;
  }

  // Drag reorder handlers
  function handleDragStart(e: DragEvent, tierId: number) {
    draggedTierId = tierId;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
    }
  }

  function handleDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    dragOverIndex = index;
  }

  function handleDragLeave() {
    dragOverIndex = null;
  }

  async function handleDrop(e: DragEvent, dropIndex: number) {
    e.preventDefault();
    dragOverIndex = null;

    if (draggedTierId === null) return;

    const currentTiers = [...tiers];
    const draggedIndex = currentTiers.findIndex(t => t.id === draggedTierId);
    if (draggedIndex === -1 || draggedIndex === dropIndex) {
      draggedTierId = null;
      return;
    }

    // Reorder: remove from old position, insert at new
    const [moved] = currentTiers.splice(draggedIndex, 1);
    currentTiers.splice(dropIndex, 0, moved);

    // Send new order to backend (array of tier IDs in display order, top = highest position)
    await reorderTiers(currentTiers.map(t => t.id));
    draggedTierId = null;
  }

  function handleDragEnd() {
    draggedTierId = null;
    dragOverIndex = null;
  }
</script>

<div class="space-y-6">
  <div>
    <h3 class="text-lg font-semibold text-text mb-1">Tier List</h3>
    <p class="text-sm text-text-muted">Customize your tier system, names, and colors.</p>
  </div>

  <!-- Preset selector -->
  <div class="space-y-2">
    <span class="block text-sm font-medium text-text">Preset</span>
    <div class="flex gap-2">
      {#each [
        { id: "10-star", label: "10 Stars" },
        { id: "5-star", label: "5 Stars" },
        { id: "custom", label: "Custom" },
      ] as presetOption (presetOption.id)}
        <button
          type="button"
          onclick={() => requestPresetChange(presetOption.id)}
          class="px-4 py-2 text-sm rounded-lg border transition-colors
            {preset === presetOption.id
              ? 'border-accent bg-accent/10 text-accent font-medium'
              : 'border-border text-text-muted hover:text-text hover:bg-surface-hover'}"
        >
          {presetOption.label}
        </button>
      {/each}
    </div>
    <p class="text-xs text-text-muted">
      {#if preset === "10-star"}
        10 tiers (Masterpiece to Terrible). Star ratings available in detail modals.
      {:else if preset === "5-star"}
        5 tiers (Excellent to Terrible). Full-star ratings in detail modals.
      {:else}
        Custom tiers. Use the tier picker dropdown in detail modals.
      {/if}
    </p>
  </div>

  <!-- Tiers list -->
  <div class="space-y-2">
    <div class="flex items-center justify-between">
      <span class="block text-sm font-medium text-text">Tiers</span>
      <span class="text-xs text-text-muted">{tiers.length} tier{tiers.length !== 1 ? 's' : ''}</span>
    </div>

    <div class="border border-border rounded-lg overflow-hidden">
      {#each tiers as tier, index (tier.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="flex items-center gap-3 px-3 py-2.5 border-b border-border last:border-b-0 transition-colors
            {dragOverIndex === index ? 'bg-accent/10' : 'bg-surface'}
            {draggedTierId === tier.id ? 'opacity-50' : ''}"
          draggable="true"
          ondragstart={(e) => handleDragStart(e, tier.id)}
          ondragover={(e) => handleDragOver(e, index)}
          ondragleave={handleDragLeave}
          ondrop={(e) => handleDrop(e, index)}
          ondragend={handleDragEnd}
        >
          <!-- Drag handle -->
          <div class="cursor-grab active:cursor-grabbing text-text-muted hover:text-text">
            <GripVertical class="w-4 h-4" />
          </div>

          {#if editingTierId === tier.id}
            <!-- Edit mode -->
            <div class="flex-1 flex items-center gap-2">
              <input
                type="text"
                bind:value={editName}
                onkeydown={handleEditKeyDown}
                class="flex-1 bg-background border border-border rounded px-2 py-1 text-sm text-text outline-none focus:ring-2 focus:ring-accent"
              />
              <input
                type="color"
                bind:value={editColor}
                class="w-8 h-8 rounded cursor-pointer border border-border bg-transparent"
              />
              <button
                type="button"
                onclick={saveEdit}
                class="px-2 py-1 text-xs bg-accent text-white rounded hover:bg-accent/90"
              >
                Save
              </button>
              <button
                type="button"
                onclick={cancelEdit}
                class="px-2 py-1 text-xs text-text-muted hover:text-text"
              >
                Cancel
              </button>
            </div>
          {:else}
            <!-- Display mode -->
            <!-- Color indicator -->
            {#if tier.color}
              <div class="w-4 h-4 rounded-full flex-shrink-0" style="background-color: {tier.color};"></div>
            {:else}
              <div class="w-4 h-4 rounded-full flex-shrink-0 bg-border"></div>
            {/if}

            <!-- Tier name (clickable to edit) -->
            <button
              type="button"
              onclick={() => startEdit(tier)}
              class="flex-1 text-left text-sm text-text hover:text-accent transition-colors"
              title="Click to edit"
            >
              {tier.name}
            </button>

            <!-- Position badge -->
            <span class="text-xs text-text-muted">#{tier.position}</span>

            <!-- Delete button -->
            <button
              type="button"
              onclick={() => requestDelete(tier.id)}
              class="p-1 rounded hover:bg-red-500/10 text-text-muted hover:text-red-400 transition-colors"
              title="Delete tier"
            >
              <Trash2 class="w-3.5 h-3.5" />
            </button>
          {/if}
        </div>
      {/each}

      {#if tiers.length === 0}
        <div class="px-3 py-6 text-center text-sm text-text-muted">
          No tiers. Select a preset or add tiers below.
        </div>
      {/if}
    </div>
  </div>

  <!-- Add tier -->
  {#if showNewTierForm}
    <div class="flex items-center gap-2 p-3 bg-surface rounded-lg border border-border">
      <input
        type="text"
        bind:value={newTierName}
        placeholder="Tier name..."
        onkeydown={(e) => { if (e.key === 'Enter') handleCreateTier(); if (e.key === 'Escape') { showNewTierForm = false; } }}
        class="flex-1 bg-background border border-border rounded px-2 py-1.5 text-sm text-text outline-none focus:ring-2 focus:ring-accent"
      />
      <div class="flex gap-1">
        {#each colorPresets as color}
          <button
            type="button"
            onclick={() => newTierColor = color}
            class="w-5 h-5 rounded-full transition-transform hover:scale-110
              {newTierColor === color ? 'ring-2 ring-accent ring-offset-1 ring-offset-surface' : ''}"
            style="background-color: {color};"
            title="Select color {color}"
          ></button>
        {/each}
        <input
          type="color"
          bind:value={newTierColor}
          class="w-5 h-5 rounded cursor-pointer border border-border bg-transparent"
          title="Custom color"
        />
      </div>
      <button
        type="button"
        onclick={handleCreateTier}
        disabled={!newTierName.trim()}
        class="px-3 py-1.5 text-xs bg-accent text-white rounded hover:bg-accent/90 disabled:opacity-50"
      >
        Add
      </button>
      <button
        type="button"
        onclick={() => { showNewTierForm = false; newTierName = ""; newTierColor = ""; }}
        class="px-2 py-1.5 text-xs text-text-muted hover:text-text"
      >
        Cancel
      </button>
    </div>
  {:else}
    <button
      type="button"
      onclick={() => { showNewTierForm = true; }}
      class="flex items-center gap-2 px-3 py-2 text-sm text-text-muted hover:text-text border border-dashed border-border rounded-lg hover:bg-surface-hover transition-colors w-full"
    >
      <Plus class="w-4 h-4" />
      Add Tier
    </button>
  {/if}

  <!-- Tiermaker Import -->
  <div class="pt-4 border-t border-border">
    <TiermakerImport />
  </div>

  <!-- Reset button -->
  <div class="pt-2 border-t border-border">
    <button
      type="button"
      onclick={requestReset}
      class="flex items-center gap-2 px-3 py-2 text-sm text-text-muted hover:text-text hover:bg-surface-hover rounded-lg transition-colors"
    >
      <RotateCcw class="w-4 h-4" />
      Reset to Default (10 Stars)
    </button>
  </div>

  <!-- Confirmation dialogs -->
  {#if pendingPreset}
    <div class="fixed inset-0 bg-black/60 z-[70] flex items-center justify-center" role="dialog">
      <div class="bg-surface rounded-xl border border-border shadow-2xl p-6 max-w-sm">
        <h4 class="text-base font-semibold text-text mb-2">Change Preset?</h4>
        <p class="text-sm text-text-muted mb-4">
          Switching presets will replace all current tiers and remove all tier assignments from shows and movies. This cannot be undone.
        </p>
        <div class="flex justify-end gap-2">
          <button
            type="button"
            onclick={() => pendingPreset = null}
            class="px-3 py-2 text-sm text-text-muted hover:bg-surface-hover rounded-lg"
          >
            Cancel
          </button>
          <button
            type="button"
            onclick={confirmPresetChange}
            class="px-3 py-2 text-sm bg-accent text-white rounded-lg hover:bg-accent/90"
          >
            Switch to {pendingPreset === "10-star" ? "10 Stars" : pendingPreset === "5-star" ? "5 Stars" : "Custom"}
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if pendingDeleteId !== null}
    {@const tierToDelete = tiers.find(t => t.id === pendingDeleteId)}
    <div class="fixed inset-0 bg-black/60 z-[70] flex items-center justify-center" role="dialog">
      <div class="bg-surface rounded-xl border border-border shadow-2xl p-6 max-w-sm">
        <h4 class="text-base font-semibold text-text mb-2">Delete Tier?</h4>
        <p class="text-sm text-text-muted mb-4">
          Delete "{tierToDelete?.name ?? 'this tier'}"? Shows and movies in this tier will become untiered.
        </p>
        <div class="flex justify-end gap-2">
          <button
            type="button"
            onclick={() => pendingDeleteId = null}
            class="px-3 py-2 text-sm text-text-muted hover:bg-surface-hover rounded-lg"
          >
            Cancel
          </button>
          <button
            type="button"
            onclick={confirmDelete}
            class="px-3 py-2 text-sm bg-red-500 text-white rounded-lg hover:bg-red-600"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if pendingReset}
    <div class="fixed inset-0 bg-black/60 z-[70] flex items-center justify-center" role="dialog">
      <div class="bg-surface rounded-xl border border-border shadow-2xl p-6 max-w-sm">
        <h4 class="text-base font-semibold text-text mb-2">Reset to Default?</h4>
        <p class="text-sm text-text-muted mb-4">
          This will replace all current tiers with the default 10-star preset and remove all tier assignments. This cannot be undone.
        </p>
        <div class="flex justify-end gap-2">
          <button
            type="button"
            onclick={() => pendingReset = false}
            class="px-3 py-2 text-sm text-text-muted hover:bg-surface-hover rounded-lg"
          >
            Cancel
          </button>
          <button
            type="button"
            onclick={confirmReset}
            class="px-3 py-2 text-sm bg-red-500 text-white rounded-lg hover:bg-red-600"
          >
            Reset
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
