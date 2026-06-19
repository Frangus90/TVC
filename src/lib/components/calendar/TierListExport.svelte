<script lang="ts">
  import type { Tier, TierListShow, TierListMovie } from "../../stores/tiers.svelte";

  interface Props {
    mode: "shows" | "movies";
    tiers: Tier[];
    items: TierListShow[] | TierListMovie[];
    appVersion: string;
  }

  let { mode, tiers, items, appVersion }: Props = $props();

  const today = new Date().toISOString().slice(0, 10);
  const heading = $derived(mode === "shows" ? "Shows" : "Movies");

  function itemTitle(item: TierListShow | TierListMovie): string {
    return (item as TierListMovie).title ?? (item as TierListShow).name ?? "";
  }

  function itemsForTier(tierId: number | null) {
    return items
      .filter((i) => i.tier_id === tierId)
      .sort(
        (a, b) =>
          (a.rank_order ?? 999999) - (b.rank_order ?? 999999) || a.id - b.id
      );
  }

  const untieredItems = $derived(itemsForTier(null));
</script>

<div class="export-root">
  <header class="export-header">
    <h1>TVC — {heading} Tier List</h1>
    <div class="subtitle">{today}</div>
  </header>

  {#each tiers as tier (tier.id)}
    {@const tierItems = itemsForTier(tier.id)}
    <div class="tier-row">
      <div class="tier-band" style="background-color: {tier.color};"></div>
      <div class="tier-label">
        <div class="tier-name">{tier.name}</div>
        <div class="tier-pos">#{tier.position}</div>
      </div>
      <div class="tier-items">
        {#each tierItems as item (item.id)}
          <div class="poster">
            {#if item.poster_url}
              <img
                src={item.poster_url}
                alt={itemTitle(item)}
                loading="eager"
                decoding="sync"
                fetchpriority="high"
              />
            {:else}
              <div class="poster-placeholder">{itemTitle(item)}</div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/each}

  <div class="tier-row">
    <div class="tier-band" style="background-color: #404040;"></div>
    <div class="tier-label">
      <div class="tier-name">Untiered</div>
      <div class="tier-pos">—</div>
    </div>
    <div class="tier-items">
      {#each untieredItems as item (item.id)}
        <div class="poster">
          {#if item.poster_url}
            <img
              src={item.poster_url}
              alt={itemTitle(item)}
              loading="eager"
              decoding="sync"
              fetchpriority="high"
            />
          {:else}
            <div class="poster-placeholder">{itemTitle(item)}</div>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  <footer class="export-footer">
    Generated {today} · TVC v{appVersion}
  </footer>
</div>

<style>
  .export-root {
    width: 1400px;
    background-color: #0f0f0f;
    color: #e5e5e5;
    padding: 32px;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    box-sizing: border-box;
  }

  .export-header {
    margin-bottom: 24px;
    border-bottom: 1px solid #2a2a2a;
    padding-bottom: 16px;
  }

  .export-header h1 {
    font-size: 28px;
    font-weight: 700;
    margin: 0 0 4px 0;
    color: #ffffff;
  }

  .subtitle {
    font-size: 14px;
    color: #a3a3a3;
  }

  .tier-row {
    display: flex;
    align-items: stretch;
    gap: 16px;
    padding: 16px 0;
    border-bottom: 1px solid #1f1f1f;
    min-height: 120px;
  }

  .tier-band {
    width: 8px;
    flex-shrink: 0;
    border-radius: 4px;
  }

  .tier-label {
    width: 200px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .tier-name {
    font-size: 22px;
    font-weight: 700;
    color: #ffffff;
    line-height: 1.2;
    word-wrap: break-word;
  }

  .tier-pos {
    font-size: 13px;
    color: #a3a3a3;
    margin-top: 2px;
  }

  .tier-items {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-content: flex-start;
  }

  .poster {
    width: 80px;
    height: 120px;
    flex-shrink: 0;
    border-radius: 4px;
    overflow: hidden;
    background-color: #1a1a1a;
  }

  .poster img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .poster-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    font-size: 10px;
    color: #a3a3a3;
    padding: 4px;
    box-sizing: border-box;
    border: 1px solid #2a2a2a;
    overflow: hidden;
    word-break: break-word;
  }

  .export-footer {
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid #2a2a2a;
    font-size: 12px;
    color: #737373;
    text-align: center;
  }
</style>
