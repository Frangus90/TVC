<script lang="ts">
  import type { Tier, TierListShow, TierListMovie } from "../../stores/tiers.svelte";
  import { formatDate } from "../../utils/dateFormat";

  interface Props {
    mode: "shows" | "movies";
    tiers: Tier[];
    items: TierListShow[] | TierListMovie[];
    appVersion: string;
  }

  let { mode, tiers, items, appVersion }: Props = $props();

  const today = formatDate(new Date());
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
    <div class="export-eyebrow">TVC · YOUR LIBRARY</div>
    <h1>{heading} tier list</h1>
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
    background-color: #121318;
    color: #eff0f6;
    padding: 40px;
    font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;
    box-sizing: border-box;
  }

  .export-header {
    margin-bottom: 28px;
    padding-bottom: 8px;
  }

  .export-eyebrow {
    color: #a0a5b6;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 1.8px;
    margin-bottom: 8px;
  }

  .export-header h1 {
    font-size: 34px;
    font-weight: 600;
    margin: 0 0 8px 0;
    color: #eff0f6;
  }

  .subtitle {
    font-size: 14px;
    color: #a0a5b6;
  }

  .tier-row {
    display: flex;
    align-items: stretch;
    gap: 18px;
    padding: 18px;
    margin-bottom: 8px;
    border: 1px solid #2b2e38;
    border-radius: 12px;
    background: #191b22;
    min-height: 132px;
  }

  .tier-band {
    width: 4px;
    flex-shrink: 0;
    border-radius: 4px;
  }

  .tier-label {
    width: 170px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .tier-name {
    font-size: 20px;
    font-weight: 600;
    color: #eff0f6;
    line-height: 1.2;
    word-wrap: break-word;
  }

  .tier-pos {
    font-size: 13px;
    color: #a0a5b6;
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
    width: 88px;
    height: 132px;
    flex-shrink: 0;
    border-radius: 6px;
    overflow: hidden;
    background-color: #22252e;
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
    color: #a0a5b6;
    padding: 4px;
    box-sizing: border-box;
    border: 1px solid #2b2e38;
    overflow: hidden;
    word-break: break-word;
  }

  .export-footer {
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid #2b2e38;
    font-size: 12px;
    color: #a0a5b6;
    text-align: center;
  }
</style>
