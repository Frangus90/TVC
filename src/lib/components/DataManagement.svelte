<script lang="ts">
  import { onMount } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { X } from "lucide-svelte";
  import SyncTab from "./data/SyncTab.svelte";
  import OverviewTab from "./data/OverviewTab.svelte";
  import BackupsTab from "./data/BackupsTab.svelte";
  import MaintenanceTab from "./data/MaintenanceTab.svelte";
  import ArchiveManager from "./ArchiveManager.svelte";
  import { isModalOpen, getActiveTab, isDataWriting, setActiveTab, closeDataManagement, type DataTab } from "../stores/dataManagement.svelte";
  import { isBackupRestoring, isBackupRestartRequired } from "../stores/backups.svelte";
  import { modalFocus } from "../utils/modalFocus";
  let { embedded = false }: { embedded?: boolean } = $props();
  const tabs: { id: DataTab; label: string }[] = [{ id: "overview", label: "Overview" }, { id: "sync", label: "Sync" }, { id: "backups", label: "Backups" }, { id: "archive", label: "Archive" }, { id: "maintenance", label: "Maintenance" }];
  const active = $derived(getActiveTab());
  const restoreBlocked = $derived(isBackupRestoring() || isBackupRestartRequired());
  onMount(() => { if (embedded) setActiveTab(getActiveTab()); });
  function focusPage(node: HTMLElement) { if (!embedded) return modalFocus(node, closeDataManagement); }
</script>

{#if embedded || isModalOpen()}
  {#if !embedded}<button type="button" transition:fade={{ duration: 150 }} class="fixed inset-0 bg-black/60 z-50" onclick={closeDataManagement} aria-label="Close Data management"></button>{/if}
  <div use:focusPage transition:scale={{ duration: embedded ? 0 : 200, start: 0.95, opacity: 0 }} class={embedded ? "focus-page focus-data-page" : "fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[1000px] max-w-[95vw] max-h-[85vh] flex flex-col"}>
    <header class={embedded ? "focus-page-heading focus-page-bar" : "flex flex-wrap items-center justify-between gap-3 p-4 border-b border-border"}>
      {#if embedded}<h1>Data management</h1>{:else}<h2 class="text-lg font-semibold">Data management</h2>{/if}
      <nav class="focus-tabs" aria-label="Data management sections">
        {#each tabs as tab}<button type="button" aria-pressed={active === tab.id} class:active={active === tab.id} disabled={isDataWriting() || restoreBlocked} onclick={() => setActiveTab(tab.id)}>{tab.label}</button>{/each}
      </nav>
      {#if !embedded}<button type="button" aria-label="Close" onclick={closeDataManagement}><X size={20} /></button>{/if}
    </header>
    <div class={embedded ? "focus-data-content" : "flex-1 overflow-auto p-4"}>
      {#if active === "overview"}<OverviewTab />
      {:else if active === "sync"}<SyncTab />
      {:else if active === "backups"}<BackupsTab />
      {:else if active === "archive"}<ArchiveManager />
      {:else}<MaintenanceTab />{/if}
    </div>
  </div>
{/if}

<style>
  .focus-page-bar .focus-tabs { margin-bottom: 0; }
  .focus-data-page { max-width: 1280px; }
  .focus-data-content { min-width: 0; }
</style>
