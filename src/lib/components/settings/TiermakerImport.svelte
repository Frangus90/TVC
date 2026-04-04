<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Search, Upload, X, Check, AlertTriangle, Loader2 } from "lucide-svelte";
  import {
    getTiers,
    addMovieTierOnly,
    addShowTierOnly,
    addManualMovie,
    addManualShow,
    loadTierListMovies,
    loadTierListShows,
    createTier,
    reorderTiers,
    loadTiers,
  } from "../../stores/tiers.svelte";

  type ContentType = "movies" | "shows";

  type ImportStep = "paste" | "review" | "importing" | "done";

  interface ParsedItem {
    name: string;
    tierLabel: string;
  }

  interface MatchedItem {
    parsed: ParsedItem;
    matchedId: number | null;
    matchedTitle: string | null;
    matchedPoster: string | null;
    matchedYear: string | null;
    tierId: number | null;
    status: "matched" | "unmatched" | "manual" | "skipped";
    searchResults: SearchResult[];
    showSearch: boolean;
  }

  interface SearchResult {
    id: number;
    title: string;
    poster_url: string | null;
    year: string | null;
  }

  let step = $state<ImportStep>("paste");
  let contentType = $state<ContentType>("movies");
  let htmlInput = $state("");
  let parsedItems = $state<ParsedItem[]>([]);
  let matchedItems = $state<MatchedItem[]>([]);
  let searchProgress = $state(0);
  let searchTotal = $state(0);
  let isSearching = $state(false);
  let importProgress = $state(0);
  let importTotal = $state(0);
  let importErrors = $state<string[]>([]);
  let manualSearchQuery = $state("");
  let manualSearchResults = $state<SearchResult[]>([]);
  let manualSearchLoading = $state(false);
  let activeManualIndex = $state<number | null>(null);
  let autoCreateTiers = $state(true);
  let createdTierCount = $state(0);

  const tiers = $derived(getTiers());

  // Stats for review
  const matchedCount = $derived(matchedItems.filter(i => i.status === "matched" || i.status === "manual").length);
  const unmatchedCount = $derived(matchedItems.filter(i => i.status === "unmatched").length);
  const skippedCount = $derived(matchedItems.filter(i => i.status === "skipped").length);

  // =============================================
  // Step 1: Parse HTML
  // =============================================

  function cleanNameFromUrl(url: string): string {
    // Extract filename from Tiermaker image URL
    // Two formats:
    //   Clean: "inception.png", "a-clockwork-orange.png"
    //   Prefixed: "zzzzz-1733232122dune.png" (zzzzz-<10-digit-timestamp><name>)
    const parts = url.split("/");
    let filename = parts[parts.length - 1] ?? "";

    // Remove file extension
    filename = filename.replace(/\.\w+$/, "");

    // Strip "zzzzz-" prefix + 10-digit timestamp glued to the name
    // e.g. "zzzzz-1733232122dune" → "dune"
    // e.g. "zzzzz-1735307121the-godfather-part-iii" → "the-godfather-part-iii"
    filename = filename.replace(/^zzzzz-\d{10}/, "");

    // Replace hyphens/underscores with spaces
    filename = filename.replace(/[-_]+/g, " ");

    // Clean up common artifacts
    filename = filename
      .replace(/\s+/g, " ")
      .trim();

    // Title case
    filename = filename
      .split(" ")
      .map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
      .join(" ");

    return filename;
  }

  function parseHtml() {
    if (!htmlInput.trim()) return;

    const parser = new DOMParser();
    const doc = parser.parseFromString(htmlInput, "text/html");
    const items: ParsedItem[] = [];

    // Try Tiermaker structure: .tier-row > .label + .character
    const tierRows = doc.querySelectorAll(".tier-row");

    if (tierRows.length > 0) {
      tierRows.forEach(row => {
        const labelEl = row.querySelector(".tier-label, .label span, .label");
        const tierLabel = labelEl?.textContent?.trim() ?? "Unknown";

        const characters = row.querySelectorAll(".character, .tier-item");
        characters.forEach(char => {
          const img = char.querySelector("img");
          if (img) {
            const src = img.getAttribute("src") ?? img.getAttribute("data-src") ?? "";
            const alt = img.getAttribute("alt") ?? "";
            const name = alt || cleanNameFromUrl(src);
            if (name) {
              items.push({ name, tierLabel });
            }
          }
        });
      });
    }

    // Fallback: just find all images in tier-like containers
    if (items.length === 0) {
      const allImages = doc.querySelectorAll("img");
      allImages.forEach(img => {
        const src = img.getAttribute("src") ?? "";
        const alt = img.getAttribute("alt") ?? "";
        if (src.includes("tiermaker") || src.includes("chart")) {
          const name = alt || cleanNameFromUrl(src);
          if (name && name.length > 1) {
            // Try to find parent tier label
            const row = img.closest(".tier-row, [class*='tier'], tr");
            const labelEl = row?.querySelector(".tier-label, .label span, .label, td:first-child");
            const tierLabel = labelEl?.textContent?.trim() ?? "Unknown";
            items.push({ name, tierLabel });
          }
        }
      });
    }

    if (items.length === 0) {
      // Last resort: grab any images with meaningful names
      const allImages = doc.querySelectorAll("img[src]");
      allImages.forEach(img => {
        const src = img.getAttribute("src") ?? "";
        const alt = img.getAttribute("alt") ?? "";
        const name = alt || cleanNameFromUrl(src);
        if (name && name.length > 2) {
          items.push({ name, tierLabel: "Unknown" });
        }
      });
    }

    parsedItems = items;

    if (items.length > 0) {
      startBatchSearch();
    }
  }

  // =============================================
  // Step 2: Batch search
  // =============================================

  async function searchSingle(name: string): Promise<SearchResult[]> {
    if (contentType === "movies") {
      const results = await invoke<Array<{
        id: number;
        title: string;
        poster_url: string | null;
        release_date: string | null;
        vote_average: number | null;
      }>>("search_movies", { query: name });

      return results.slice(0, 5).map(r => ({
        id: r.id,
        title: r.title,
        poster_url: r.poster_url,
        year: r.release_date?.substring(0, 4) ?? null,
      }));
    } else {
      const results = await invoke<Array<{
        tvdb_id: string | null;
        name: string;
        image_url: string | null;
        year: string | null;
      }>>("search_shows", { query: name });

      return results.slice(0, 5).map(r => ({
        id: parseInt(r.tvdb_id ?? "0", 10),
        title: r.name,
        poster_url: r.image_url,
        year: r.year,
      }));
    }
  }

  // Default colors for auto-created tiers (top=best → bottom=worst)
  const defaultTierColors = [
    "#ef4444", "#f97316", "#eab308", "#22c55e", "#06b6d4",
    "#3b82f6", "#8b5cf6", "#ec4899", "#f43f5e", "#14b8a6",
  ];

  function findBestTierMatch(tierLabel: string, tierMap: Map<string, number>): number | null {
    if (!tierLabel || tierLabel === "Unknown") return null;

    // Check the import-created tier map first (exact match on original label)
    const mapped = tierMap.get(tierLabel);
    if (mapped !== undefined) return mapped;

    const label = tierLabel.toLowerCase().trim();

    // Exact match against existing tiers
    const exact = tiers.find(t => t.name.toLowerCase() === label);
    if (exact) return exact.id;

    // Partial match
    const partial = tiers.find(t =>
      t.name.toLowerCase().includes(label) || label.includes(t.name.toLowerCase())
    );
    if (partial) return partial.id;

    return null;
  }

  async function createTiersFromLabels(labels: string[]): Promise<Map<string, number>> {
    const tierMap = new Map<string, number>();
    createdTierCount = 0;

    // Reuse existing tiers by name, only create missing ones
    // This prevents wiping out movie tiers when importing a show list (or vice versa)
    for (let i = 0; i < labels.length; i++) {
      const label = labels[i];
      const lowerLabel = label.toLowerCase().trim();

      // Try exact match first, then prefix match for labels like "S - Masterpiece" → "S"
      const existing = tiers.find(t => t.name.toLowerCase() === lowerLabel)
        || tiers.find(t => {
          const prefix = lowerLabel.split(/\s*-\s*/)[0];
          return prefix && t.name.toLowerCase() === prefix;
        });
      if (existing) {
        tierMap.set(label, existing.id);
        continue;
      }

      const color = defaultTierColors[i % defaultTierColors.length];
      const newTier = await createTier(label, color);
      if (newTier) {
        tierMap.set(label, newTier.id);
        createdTierCount++;
      }
    }

    // Reorder: import labels first (top=best), then any pre-existing tiers not in the import
    if (createdTierCount > 0) {
      await loadTiers();
      const importTierIds = labels
        .map(l => tierMap.get(l))
        .filter((id): id is number => id !== undefined);
      const importIdSet = new Set(importTierIds);
      const otherTierIds = getTiers()
        .filter(t => !importIdSet.has(t.id))
        .map(t => t.id);
      await reorderTiers([...importTierIds, ...otherTierIds]);
    }

    return tierMap;
  }

  async function startBatchSearch() {
    isSearching = true;
    searchTotal = parsedItems.length;
    searchProgress = 0;

    // Extract unique tier labels in order of appearance
    const seenLabels = new Set<string>();
    const orderedLabels: string[] = [];
    for (const item of parsedItems) {
      if (item.tierLabel && item.tierLabel !== "Unknown" && !seenLabels.has(item.tierLabel)) {
        seenLabels.add(item.tierLabel);
        orderedLabels.push(item.tierLabel);
      }
    }

    // Auto-create tiers if enabled
    let tierMap = new Map<string, number>();
    if (autoCreateTiers && orderedLabels.length > 0) {
      tierMap = await createTiersFromLabels(orderedLabels);
    }

    const results: MatchedItem[] = [];

    for (const item of parsedItems) {
      try {
        const searchResults = await searchSingle(item.name);
        const best = searchResults[0];
        const tierId = findBestTierMatch(item.tierLabel, tierMap);

        if (best) {
          // Check if title is a reasonable match
          const similarity = calculateSimilarity(item.name.toLowerCase(), best.title.toLowerCase());

          results.push({
            parsed: item,
            matchedId: similarity > 0.3 ? best.id : null,
            matchedTitle: similarity > 0.3 ? best.title : null,
            matchedPoster: similarity > 0.3 ? best.poster_url : null,
            matchedYear: similarity > 0.3 ? best.year : null,
            tierId,
            status: similarity > 0.3 ? "matched" : "unmatched",
            searchResults,
            showSearch: false,
          });
        } else {
          results.push({
            parsed: item,
            matchedId: null,
            matchedTitle: null,
            matchedPoster: null,
            matchedYear: null,
            tierId,
            status: "unmatched",
            searchResults: [],
            showSearch: false,
          });
        }
      } catch {
        results.push({
          parsed: item,
          matchedId: null,
          matchedTitle: null,
          matchedPoster: null,
          matchedYear: null,
          tierId: findBestTierMatch(item.tierLabel, tierMap),
          status: "unmatched",
          searchResults: [],
          showSearch: false,
        });
      }

      searchProgress++;
    }

    matchedItems = results;
    isSearching = false;
    step = "review";
  }

  function calculateSimilarity(a: string, b: string): number {
    // Simple word overlap similarity
    const wordsA = new Set(a.split(/\s+/).filter(w => w.length > 1));
    const wordsB = new Set(b.split(/\s+/).filter(w => w.length > 1));
    if (wordsA.size === 0 || wordsB.size === 0) return 0;

    let overlap = 0;
    for (const word of wordsA) {
      if (wordsB.has(word)) overlap++;
    }
    return overlap / Math.max(wordsA.size, wordsB.size);
  }

  // =============================================
  // Manual matching
  // =============================================

  function toggleManualSearch(index: number) {
    if (activeManualIndex === index) {
      activeManualIndex = null;
      matchedItems[index].showSearch = false;
      manualSearchQuery = "";
      manualSearchResults = [];
    } else {
      // Close previous
      if (activeManualIndex !== null && activeManualIndex < matchedItems.length) {
        matchedItems[activeManualIndex].showSearch = false;
      }
      activeManualIndex = index;
      matchedItems[index].showSearch = true;
      manualSearchQuery = matchedItems[index].parsed.name;
      manualSearchResults = matchedItems[index].searchResults;
    }
  }

  async function handleManualSearch() {
    if (!manualSearchQuery.trim()) return;
    manualSearchLoading = true;
    try {
      manualSearchResults = await searchSingle(manualSearchQuery.trim());
    } catch {
      manualSearchResults = [];
    }
    manualSearchLoading = false;
  }

  function selectManualMatch(index: number, result: SearchResult) {
    matchedItems[index] = {
      ...matchedItems[index],
      matchedId: result.id,
      matchedTitle: result.title,
      matchedPoster: result.poster_url,
      matchedYear: result.year,
      status: "manual",
      showSearch: false,
    };
    activeManualIndex = null;
    manualSearchQuery = "";
    manualSearchResults = [];
  }

  function addAsManualEntry(index: number) {
    matchedItems[index] = {
      ...matchedItems[index],
      matchedId: null,
      matchedTitle: matchedItems[index].parsed.name,
      status: "manual",
      showSearch: false,
    };
    activeManualIndex = null;
    manualSearchQuery = "";
    manualSearchResults = [];
  }

  function skipItem(index: number) {
    matchedItems[index] = {
      ...matchedItems[index],
      status: "skipped",
      showSearch: false,
    };
    if (activeManualIndex === index) {
      activeManualIndex = null;
    }
  }

  function updateItemTier(index: number, tierId: number | null) {
    matchedItems[index] = { ...matchedItems[index], tierId };
  }

  // =============================================
  // Step 3: Import
  // =============================================

  async function startImport() {
    const toImport = matchedItems.filter(i => i.status === "matched" || i.status === "manual");
    step = "importing";
    importTotal = toImport.length;
    importProgress = 0;
    importErrors = [];

    for (const item of toImport) {
      try {
        if (item.matchedId && item.matchedId > 0) {
          // API-matched item
          if (contentType === "movies") {
            await addMovieTierOnly(item.matchedId, item.tierId);
          } else {
            await addShowTierOnly(item.matchedId, item.tierId);
          }
        } else {
          // Manual entry (no API ID)
          const title = item.matchedTitle ?? item.parsed.name;
          if (contentType === "movies") {
            await addManualMovie(title, item.matchedPoster, item.tierId);
          } else {
            await addManualShow(title, item.matchedPoster, item.tierId);
          }
        }
      } catch (e) {
        importErrors.push(`${item.parsed.name}: ${e}`);
      }
      importProgress++;
    }

    // Refresh tier lists
    if (contentType === "movies") {
      await loadTierListMovies();
    } else {
      await loadTierListShows();
    }

    step = "done";
  }

  // =============================================
  // Reset
  // =============================================

  function reset() {
    step = "paste";
    htmlInput = "";
    parsedItems = [];
    matchedItems = [];
    searchProgress = 0;
    searchTotal = 0;
    importProgress = 0;
    importTotal = 0;
    importErrors = [];
    manualSearchQuery = "";
    manualSearchResults = [];
    activeManualIndex = null;
    createdTierCount = 0;
  }
</script>

<div class="space-y-4">
  <div>
    <h4 class="text-sm font-semibold text-text mb-1">Import from Tiermaker</h4>
    <p class="text-xs text-text-muted">
      Paste HTML from a Tiermaker list (right-click → Copy element in DevTools) to import items.
    </p>
  </div>

  {#if step === "paste"}
    <!-- Content type toggle -->
    <div class="flex gap-2">
      <button
        type="button"
        onclick={() => contentType = "movies"}
        class="px-3 py-1.5 text-xs rounded-lg border transition-colors
          {contentType === 'movies'
            ? 'border-accent bg-accent/10 text-accent font-medium'
            : 'border-border text-text-muted hover:text-text hover:bg-surface-hover'}"
      >
        Movies (TMDB)
      </button>
      <button
        type="button"
        onclick={() => contentType = "shows"}
        class="px-3 py-1.5 text-xs rounded-lg border transition-colors
          {contentType === 'shows'
            ? 'border-accent bg-accent/10 text-accent font-medium'
            : 'border-border text-text-muted hover:text-text hover:bg-surface-hover'}"
      >
        TV Shows (TVDB)
      </button>
    </div>

    <!-- Auto-create tiers option -->
    <label class="flex items-center gap-2 text-xs text-text">
      <input type="checkbox" bind:checked={autoCreateTiers} class="rounded border-border" />
      Auto-create tiers from Tiermaker labels
      <span class="text-text-muted">(replaces existing tier layout)</span>
    </label>

    <!-- HTML paste area -->
    <textarea
      bind:value={htmlInput}
      placeholder="Paste Tiermaker HTML here..."
      class="w-full h-32 bg-background border border-border rounded-lg px-3 py-2 text-xs text-text font-mono outline-none focus:ring-2 focus:ring-accent resize-y"
    ></textarea>

    <div class="flex items-center gap-2">
      <button
        type="button"
        onclick={parseHtml}
        disabled={!htmlInput.trim() || isSearching}
        class="flex items-center gap-2 px-4 py-2 text-sm bg-accent text-white rounded-lg hover:bg-accent/90 disabled:opacity-50 transition-colors"
      >
        {#if isSearching}
          <Loader2 class="w-4 h-4 animate-spin" />
          Searching... ({searchProgress}/{searchTotal})
        {:else}
          <Upload class="w-4 h-4" />
          Parse & Search
        {/if}
      </button>

      {#if parsedItems.length > 0 && isSearching}
        <span class="text-xs text-text-muted">
          Found {parsedItems.length} items, searching {contentType === "movies" ? "TMDB" : "TVDB"}...
        </span>
      {/if}
    </div>

  {:else if step === "review"}
    <!-- Review header -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <span class="flex items-center gap-1 text-xs text-green-400">
          <Check class="w-3.5 h-3.5" /> {matchedCount} matched
        </span>
        <span class="flex items-center gap-1 text-xs text-yellow-400">
          <AlertTriangle class="w-3.5 h-3.5" /> {unmatchedCount} unmatched
        </span>
        {#if skippedCount > 0}
          <span class="text-xs text-text-muted">{skippedCount} skipped</span>
        {/if}
      </div>
      <button
        type="button"
        onclick={reset}
        class="text-xs text-text-muted hover:text-text"
      >
        Start Over
      </button>
    </div>

    <!-- Items list -->
    <div class="border border-border rounded-lg overflow-hidden max-h-[400px] overflow-y-auto">
      {#each matchedItems as item, index (index)}
        <div class="flex items-start gap-3 px-3 py-2 border-b border-border last:border-b-0 text-xs
          {item.status === 'skipped' ? 'opacity-40' : ''}
          {item.status === 'unmatched' ? 'bg-yellow-500/5' : ''}">

          <!-- Poster thumbnail -->
          <div class="w-8 h-12 rounded overflow-hidden bg-border flex-shrink-0">
            {#if item.matchedPoster}
              <img src={item.matchedPoster} alt="" class="w-full h-full object-cover" />
            {/if}
          </div>

          <!-- Info -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              {#if item.status === "matched" || item.status === "manual"}
                <Check class="w-3 h-3 text-green-400 flex-shrink-0" />
              {:else if item.status === "unmatched"}
                <AlertTriangle class="w-3 h-3 text-yellow-400 flex-shrink-0" />
              {:else}
                <X class="w-3 h-3 text-text-muted flex-shrink-0" />
              {/if}

              <span class="text-text truncate font-medium">
                {item.matchedTitle ?? item.parsed.name}
              </span>

              {#if item.matchedYear}
                <span class="text-text-muted">({item.matchedYear})</span>
              {/if}
            </div>

            {#if item.matchedTitle && item.matchedTitle !== item.parsed.name}
              <p class="text-text-muted truncate mt-0.5">from: "{item.parsed.name}"</p>
            {/if}

            <div class="flex items-center gap-2 mt-1">
              <!-- Tier selector -->
              <select
                value={item.tierId ?? ""}
                onchange={(e) => updateItemTier(index, (e.target as HTMLSelectElement).value ? Number((e.target as HTMLSelectElement).value) : null)}
                class="bg-background border border-border rounded px-1.5 py-0.5 text-xs text-text outline-none"
              >
                <option value="">No tier</option>
                {#each tiers as tier (tier.id)}
                  <option value={tier.id}>{tier.name}</option>
                {/each}
              </select>

              <span class="text-text-muted">← {item.parsed.tierLabel}</span>
            </div>

            <!-- Manual search panel -->
            {#if item.showSearch}
              <div class="mt-2 p-2 bg-background rounded-lg border border-border space-y-2">
                <div class="flex gap-1">
                  <input
                    type="text"
                    bind:value={manualSearchQuery}
                    onkeydown={(e) => { if (e.key === "Enter") handleManualSearch(); }}
                    placeholder="Search..."
                    class="flex-1 bg-surface border border-border rounded px-2 py-1 text-xs text-text outline-none focus:ring-1 focus:ring-accent"
                  />
                  <button
                    type="button"
                    onclick={handleManualSearch}
                    disabled={manualSearchLoading}
                    class="px-2 py-1 text-xs bg-accent text-white rounded hover:bg-accent/90 disabled:opacity-50"
                  >
                    {#if manualSearchLoading}
                      <Loader2 class="w-3 h-3 animate-spin" />
                    {:else}
                      <Search class="w-3 h-3" />
                    {/if}
                  </button>
                </div>

                {#if manualSearchResults.length > 0}
                  <div class="space-y-1 max-h-[150px] overflow-y-auto">
                    {#each manualSearchResults as result (result.id)}
                      <button
                        type="button"
                        onclick={() => selectManualMatch(index, result)}
                        class="w-full flex items-center gap-2 px-2 py-1 rounded hover:bg-surface-hover text-left"
                      >
                        <div class="w-5 h-7 rounded overflow-hidden bg-border flex-shrink-0">
                          {#if result.poster_url}
                            <img src={result.poster_url} alt="" class="w-full h-full object-cover" />
                          {/if}
                        </div>
                        <span class="text-text truncate">{result.title}</span>
                        {#if result.year}
                          <span class="text-text-muted flex-shrink-0">({result.year})</span>
                        {/if}
                      </button>
                    {/each}
                  </div>
                {/if}

                <button
                  type="button"
                  onclick={() => addAsManualEntry(index)}
                  class="w-full text-left px-2 py-1 text-xs text-text-muted hover:text-text hover:bg-surface-hover rounded"
                >
                  Add as manual entry (no API data)
                </button>
              </div>
            {/if}
          </div>

          <!-- Actions -->
          <div class="flex items-center gap-1 flex-shrink-0">
            {#if item.status !== "skipped"}
              <button
                type="button"
                onclick={() => toggleManualSearch(index)}
                class="p-1 rounded hover:bg-surface-hover text-text-muted hover:text-accent transition-colors"
                title="Search manually"
              >
                <Search class="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                onclick={() => skipItem(index)}
                class="p-1 rounded hover:bg-surface-hover text-text-muted hover:text-red-400 transition-colors"
                title="Skip"
              >
                <X class="w-3.5 h-3.5" />
              </button>
            {:else}
              <button
                type="button"
                onclick={() => { matchedItems[index] = { ...matchedItems[index], status: "unmatched", showSearch: false }; }}
                class="p-1 text-xs text-text-muted hover:text-text"
                title="Undo skip"
              >
                Undo
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <!-- Import button -->
    <div class="flex items-center gap-3">
      <button
        type="button"
        onclick={startImport}
        disabled={matchedCount === 0}
        class="flex items-center gap-2 px-4 py-2 text-sm bg-accent text-white rounded-lg hover:bg-accent/90 disabled:opacity-50 transition-colors"
      >
        <Upload class="w-4 h-4" />
        Import {matchedCount} {contentType === "movies" ? "Movies" : "Shows"}
      </button>
      <span class="text-xs text-text-muted">
        {unmatchedCount} unmatched items will be skipped
      </span>
    </div>

  {:else if step === "importing"}
    <div class="flex flex-col items-center gap-3 py-8">
      <Loader2 class="w-8 h-8 text-accent animate-spin" />
      <p class="text-sm text-text">
        Importing... {importProgress}/{importTotal}
      </p>
      <div class="w-full max-w-xs bg-border rounded-full h-2">
        <div
          class="bg-accent rounded-full h-2 transition-all"
          style="width: {importTotal > 0 ? (importProgress / importTotal) * 100 : 0}%"
        ></div>
      </div>
    </div>

  {:else if step === "done"}
    <div class="flex flex-col items-center gap-3 py-8">
      <Check class="w-8 h-8 text-green-400" />
      <p class="text-sm text-text">
        Imported {importTotal - importErrors.length} {contentType === "movies" ? "movies" : "shows"} successfully!
        {#if createdTierCount > 0}
          <span class="text-text-muted">({createdTierCount} new tier{createdTierCount !== 1 ? 's' : ''} created)</span>
        {/if}
      </p>
      {#if importErrors.length > 0}
        <div class="w-full max-w-md">
          <p class="text-xs text-red-400 mb-1">{importErrors.length} errors:</p>
          <div class="max-h-[100px] overflow-y-auto text-xs text-text-muted bg-background rounded p-2 border border-border">
            {#each importErrors as error}
              <p>{error}</p>
            {/each}
          </div>
        </div>
      {/if}
      <button
        type="button"
        onclick={reset}
        class="px-4 py-2 text-sm text-text-muted hover:text-text hover:bg-surface-hover rounded-lg transition-colors"
      >
        Import More
      </button>
    </div>
  {/if}
</div>
