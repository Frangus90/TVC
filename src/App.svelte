<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
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

  // Check for updates on app start
  onMount(() => {
    console.log("[TVC] App mounted, will check for updates in 2s...");
    setTimeout(() => {
      console.log("[TVC] Checking for updates...");
      checkForUpdates(false).catch((err) => {
        console.error("[TVC] Update check failed:", err);
      });
    }, 2000);
  });

  // CTRL+Shift+I to open dev tools (like browser)
  async function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.shiftKey && event.key === "I") {
      event.preventDefault();
      console.log("[TVC] Opening dev tools...");
      try {
        const webview = getCurrentWebview();
        await invoke("plugin:webview|internal_toggle_devtools", {
          label: webview.label,
        });
      } catch (err) {
        console.error("[TVC] Failed to open dev tools:", err);
      }
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

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
