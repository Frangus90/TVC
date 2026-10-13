<script lang="ts">
  import { getActiveIntegrationTab, setActiveIntegrationTab } from "../../stores/settings.svelte";
  import PlexTab from "./PlexTab.svelte";
  import ArrTab from "./ArrTab.svelte";
</script>

<div class="space-y-4">
  <div class="focus-segment w-fit" role="group" aria-label="Integration">
    <button type="button" aria-pressed={getActiveIntegrationTab() === 'plex'} onclick={() => setActiveIntegrationTab('plex')}>Plex</button>
    <button type="button" aria-pressed={getActiveIntegrationTab() === 'arr'} onclick={() => setActiveIntegrationTab('arr')}>Sonarr / Radarr</button>
  </div>
  {#if getActiveIntegrationTab() === 'plex'}<PlexTab />{:else}<ArrTab />{/if}
</div>
