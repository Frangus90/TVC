<script lang="ts">
  import { ChevronDown } from "lucide-svelte";
  import { getTiers, getTierById } from "../stores/tiers.svelte";

  interface Props {
    tierId: number | null;
    onTierChange: (tierId: number | null) => void;
  }

  let { tierId, onTierChange }: Props = $props();
  let open = $state(false);

  const tiers = $derived(getTiers());
  const currentTier = $derived(tierId ? getTierById(tierId) : null);

  function handleSelect(id: number | null) {
    // Click same tier = unrate
    const newId = id === tierId ? null : id;
    onTierChange(newId);
    open = false;
  }

  function handleClickOutside(e: MouseEvent) {
    if (!(e.target as HTMLElement).closest('.tier-picker')) {
      open = false;
    }
  }

  $effect(() => {
    if (open) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });
</script>

<div class="relative tier-picker">
  <button
    type="button"
    onclick={(e) => { e.stopPropagation(); open = !open; }}
    class="flex items-center gap-2 px-3 py-1.5 rounded-lg border border-border hover:bg-surface-hover transition-colors text-sm"
  >
    {#if currentTier}
      {#if currentTier.color}
        <span class="w-3 h-3 rounded-full" style="background-color: {currentTier.color};"></span>
      {/if}
      <span class="text-text">{currentTier.name}</span>
    {:else}
      <span class="text-text-muted">No tier</span>
    {/if}
    <ChevronDown class="w-3.5 h-3.5 text-text-muted" />
  </button>

  {#if open}
    <div class="absolute top-full left-0 mt-1 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 min-w-[160px]">
      {#each tiers as tier (tier.id)}
        <button
          type="button"
          onclick={() => handleSelect(tier.id)}
          class="w-full px-3 py-1.5 text-left text-xs hover:bg-surface-hover flex items-center gap-2
            {tier.id === tierId ? 'text-accent font-medium' : 'text-text'}"
        >
          {#if tier.color}
            <span class="w-2.5 h-2.5 rounded-full" style="background-color: {tier.color};"></span>
          {:else}
            <span class="w-2.5 h-2.5 rounded-full bg-border"></span>
          {/if}
          {tier.name}
        </button>
      {/each}
      {#if tierId !== null}
        <div class="border-t border-border my-1"></div>
        <button
          type="button"
          onclick={() => handleSelect(null)}
          class="w-full px-3 py-1.5 text-left text-xs text-red-400 hover:bg-surface-hover"
        >
          Remove from tier
        </button>
      {/if}
    </div>
  {/if}
</div>
