<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/components/layout/Sidebar.svelte";
  import Header from "./lib/components/layout/Header.svelte";
  import MonthView from "./lib/components/calendar/MonthView.svelte";
  import WeekView from "./lib/components/calendar/WeekView.svelte";
  import AgendaView from "./lib/components/calendar/AgendaView.svelte";
  import SearchModal from "./lib/components/SearchModal.svelte";
  import EpisodePicker from "./lib/components/EpisodePicker.svelte";
  import DayDetail from "./lib/components/DayDetail.svelte";
  import { getViewMode } from "./lib/stores/calendar.svelte";
  import { checkForUpdates } from "./lib/stores/updates.svelte";

  // Check for updates on app start (silent check)
  onMount(() => {
    // Delay slightly to let app fully load
    setTimeout(() => {
      checkForUpdates(true).catch(console.error);
    }, 2000);
  });
</script>

<div class="flex h-screen bg-background text-text">
  <Sidebar />

  <main class="flex-1 flex flex-col overflow-hidden">
    <Header />

    <div class="flex-1 overflow-auto p-6">
      {#if getViewMode() === "month"}
        <MonthView />
      {:else if getViewMode() === "week"}
        <WeekView />
      {:else}
        <AgendaView />
      {/if}
    </div>
  </main>
</div>

<SearchModal />
<EpisodePicker />
<DayDetail />
