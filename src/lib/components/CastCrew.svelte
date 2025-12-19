<script lang="ts">
  import { User, ChevronDown, ChevronUp, RefreshCw } from "lucide-svelte";

  interface CastMember {
    id: number;
    person_id: number | null;
    name: string;
    character_name: string | null;
    image_url: string | null;
    order_index: number;
  }

  interface CrewMember {
    id: number;
    person_id: number | null;
    name: string;
    job: string | null;
    department: string | null;
    image_url: string | null;
  }

  interface Props {
    cast: CastMember[];
    crew?: CrewMember[];
    loading?: boolean;
    onFetch?: () => void;
  }

  let { cast, crew = [], loading = false, onFetch }: Props = $props();

  let showCrew = $state(false);

  // Group crew by job for display
  function groupCrewByJob(crewList: CrewMember[]) {
    const grouped: Record<string, CrewMember[]> = {};
    for (const member of crewList) {
      const job = member.job || "Other";
      if (!grouped[job]) {
        grouped[job] = [];
      }
      grouped[job].push(member);
    }
    return Object.entries(grouped);
  }
</script>

<div class="space-y-4">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <h3 class="font-semibold text-text">Cast & Crew</h3>
    {#if onFetch}
      <button
        type="button"
        onclick={onFetch}
        disabled={loading}
        class="px-2 py-1 text-xs bg-surface-hover hover:bg-surface-hover/80 rounded transition-colors flex items-center gap-1.5 disabled:opacity-50"
      >
        <RefreshCw class="w-3 h-3 {loading ? 'animate-spin' : ''}" />
        {cast.length === 0 ? "Load Cast" : "Refresh"}
      </button>
    {/if}
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-8">
      <RefreshCw class="w-6 h-6 text-accent animate-spin" />
    </div>
  {:else if cast.length === 0}
    <div class="text-center py-6 text-text-muted text-sm">
      <User class="w-8 h-8 mx-auto mb-2 opacity-50" />
      <p>No cast information available.</p>
      {#if onFetch}
        <p class="mt-1">Click "Load Cast" to fetch from the database.</p>
      {/if}
    </div>
  {:else}
    <!-- Cast - Horizontal Scroll -->
    <div class="overflow-x-auto pb-2">
      <div class="flex gap-3 min-w-max">
        {#each cast as member}
          <div class="w-24 flex-shrink-0 text-center">
            {#if member.image_url}
              <img
                src={member.image_url}
                alt={member.name}
                class="w-20 h-20 mx-auto rounded-full object-cover bg-surface-hover"
                loading="lazy"
                decoding="async"
              />
            {:else}
              <div class="w-20 h-20 mx-auto rounded-full bg-surface-hover flex items-center justify-center">
                <User class="w-8 h-8 text-text-muted" />
              </div>
            {/if}
            <p class="text-xs font-medium text-text mt-2 line-clamp-2">{member.name}</p>
            {#if member.character_name}
              <p class="text-xs text-text-muted line-clamp-2">{member.character_name}</p>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <!-- Crew Section (expandable) -->
    {#if crew.length > 0}
      <button
        type="button"
        onclick={() => (showCrew = !showCrew)}
        class="w-full flex items-center justify-between px-3 py-2 bg-surface-hover rounded-lg hover:bg-surface-hover/80 transition-colors"
      >
        <span class="text-sm font-medium text-text">Crew ({crew.length})</span>
        {#if showCrew}
          <ChevronUp class="w-4 h-4 text-text-muted" />
        {:else}
          <ChevronDown class="w-4 h-4 text-text-muted" />
        {/if}
      </button>

      {#if showCrew}
        <div class="grid gap-4 pl-2">
          {#each groupCrewByJob(crew) as [job, members]}
            <div>
              <h4 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">{job}</h4>
              <div class="flex flex-wrap gap-2">
                {#each members as member}
                  <div class="flex items-center gap-2 px-2 py-1 bg-background rounded">
                    {#if member.image_url}
                      <img
                        src={member.image_url}
                        alt={member.name}
                        class="w-6 h-6 rounded-full object-cover"
                        loading="lazy"
                      />
                    {:else}
                      <div class="w-6 h-6 rounded-full bg-surface-hover flex items-center justify-center">
                        <User class="w-3 h-3 text-text-muted" />
                      </div>
                    {/if}
                    <span class="text-sm text-text">{member.name}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
