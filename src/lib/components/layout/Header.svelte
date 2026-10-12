<script lang="ts">
  import { Search, TvMinimal, Bell, Settings, Power, X, Tv, Film } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openConfirmDialog } from "../../stores/confirmDialog.svelte";
  import { getUnreadCount, isNotificationCenterOpen, toggleNotificationCenter } from "../../stores/notifications.svelte";
  import NotificationCenter from "../notifications/NotificationCenter.svelte";
  import { setSidebarTab } from "../../stores/sidebar.svelte";
  import { getTrackedShows, getArchivedShows } from "../../stores/shows.svelte";
  import { getTrackedMovies, getArchivedMovies, openMovieDetail } from "../../stores/movies.svelte";
  import { getTierListShows, getTierListMovies } from "../../stores/tiers.svelte";
  import { openShowDetail } from "../../stores/showDetail.svelte";
  import { showError } from "../../stores/toast.svelte";
  import { modalFocus } from "../../utils/modalFocus";
  import { getThemeSettings } from "../../stores/theme.svelte";

  let searchOpen = $state(false);
  let query = $state("");
  const results = $derived.by(() => {
    const archivedShows = new Set(getArchivedShows().map(show => show.id));
    const archivedMovies = new Set(getArchivedMovies().map(movie => movie.id));
    const shows = new Map([...getTierListShows(), ...getTrackedShows()].map(show => [show.id, { id: show.id, title: show.name, poster: show.poster_url, type: "shows" as const }]));
    const movies = new Map([...getTierListMovies(), ...getTrackedMovies()].map(movie => [movie.id, { id: movie.id, title: movie.title, poster: movie.poster_url, type: "movies" as const }]));
    const text = query.trim().toLowerCase();
    if (!text) return [];
    return [...shows.values(), ...movies.values()].filter(item => !(item.type === "shows" ? archivedShows : archivedMovies).has(item.id) && item.title.toLowerCase().includes(text)).sort((a, b) => a.title.localeCompare(b.title));
  });
  function openSearch() { query = ""; searchOpen = true; }
  function handleKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") { event.preventDefault(); openSearch(); }
  }
  async function exitApp() {
    if (!await openConfirmDialog({ title: "Exit TVC", message: "Close TVC completely? Automatic sync and reminders will stop until you reopen the app.", type: "warning", confirmLabel: "Exit", cancelLabel: "Cancel" })) return;
    try { await invoke("exit_app"); } catch (error) { showError(`Could not exit TVC: ${error}`); }
  }
  async function openResult(item: { id: number; type: "shows" | "movies" }) {
    searchOpen = false;
    if (item.type === "shows") await openShowDetail(item.id); else await openMovieDetail(item.id);
  }
</script>

<svelte:window onkeydown={handleKeydown} />
<header class="focus-topbar">
  <div class="focus-brand"><TvMinimal size={25} strokeWidth={1.6} /><strong>TVC</strong><span>Your watching, organised.</span></div>
  <div class="focus-top-actions">
    <button type="button" class="focus-search-trigger" onclick={openSearch} aria-label="Search library"><Search size={17} /><span>Search library</span><kbd>Ctrl K</kbd></button>
    <div class="relative">
      <button type="button" data-notification-bell class="focus-icon-button" onclick={toggleNotificationCenter} aria-label="Notifications" title="Notifications"><Bell size={18} />{#if getUnreadCount() > 0}<span class="focus-unread-dot"></span>{/if}</button>
      {#if isNotificationCenterOpen()}<NotificationCenter />{/if}
    </div>
    <button type="button" class="focus-icon-button" onclick={() => setSidebarTab('settings')} aria-label="Settings" title="Settings"><Settings size={18} /></button>
    <button type="button" class="focus-icon-button" onclick={exitApp} aria-label="Exit application" title="Exit application"><Power size={18} /></button>
  </div>
</header>
{#if searchOpen}
  <div class="focus-search-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) searchOpen = false; }}>
    <div class="focus-search-dialog focus-dialog" role="dialog" aria-modal="true" aria-labelledby="library-search-title" use:modalFocus={() => searchOpen = false}>
      <div class="flex justify-between items-center gap-4 mb-5"><h2 id="library-search-title" class="text-xl font-semibold">Search your library</h2><button type="button" class="focus-icon-button" onclick={() => searchOpen = false} aria-label="Close search"><X size={19} /></button></div>
      <label class="focus-search-field"><Search size={18} /><input type="search" data-modal-initial bind:value={query} placeholder="Find a saved show or movie" aria-label="Search saved titles" /></label>
      <div class="focus-search-results" aria-live="polite">
        {#if !query.trim()}<p class="text-text-muted py-6 text-sm">Search your tracked and tier-list titles.</p>
        {:else if !results.length}<p class="text-text-muted py-6 text-sm">No saved titles match “{query}”.</p>
        {:else}
          {#each results as item (`${item.type}-${item.id}`)}
            <button type="button" class="focus-search-result" onclick={() => openResult(item)}>
              {#if item.poster && !getThemeSettings().hidePosters}<img src={item.poster} alt="" loading="lazy" decoding="async" />{:else if item.type === 'shows'}<Tv size={20} />{:else}<Film size={20} />{/if}
              <span><strong>{item.title}</strong><small>{item.type === 'shows' ? 'TV show' : 'Movie'}</small></span>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}
