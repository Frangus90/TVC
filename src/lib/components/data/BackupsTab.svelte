<script lang="ts">
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { readTextFile } from "@tauri-apps/plugin-fs";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { Download, Upload, FolderOpen, RefreshCw, AlertTriangle, Check, CircleX } from "lucide-svelte";
  import { getBackupStatus, getBackupError, isBackupLoading, isBackupSaving, isBackupRestartRequired, loadBackupStatus, updateBackupPreferences, createBackupFile, createLocalBackup, restoreBackup, type BackupFrequency } from "../../stores/backups.svelte";
  import { describeBackup, parseBackup } from "../../utils/backup";
  import { formatDateTime } from "../../utils/dateFormat";
  import { openConfirmDialog } from "../../stores/confirmDialog.svelte";
  import { waitForPendingPreferenceSaves } from "../../utils/preferenceSaves";

  let working = $state(false);
  let includeSettings = $state(false);
  let result = $state<string | null>(null);
  let warning = $state(false);
  let actionError = $state<string | null>(null);
  const restartRequired = $derived(isBackupRestartRequired());
  const status = $derived(getBackupStatus());
  const busy = $derived(working || isBackupSaving() || !!status?.running || restartRequired);

  onMount(() => {
    void loadBackupStatus();
    const timer = setInterval(() => { if (!restartRequired) void loadBackupStatus(); }, 5000);
    return () => clearInterval(timer);
  });

  async function configure(updates: Parameters<typeof updateBackupPreferences>[0]) {
    actionError = null;
    try { await updateBackupPreferences(updates); }
    catch (error) { actionError = `Could not save automatic backup settings: ${String(error)}`; }
  }
  async function selectFolder() {
    working = true; actionError = null;
    try {
      const folder = await open({ directory: true, multiple: false, title: "Choose an automatic backup folder" });
      if (typeof folder === "string") await configure({ folder });
    } catch (error) { actionError = `Could not choose a backup folder: ${String(error)}`; }
    finally { working = false; }
  }
  async function create(managed = false) {
    if (busy) return;
    working = true; actionError = null; result = null;
    try {
      const path = managed ? null : await save({
        title: "Create a TVC backup", defaultPath: `tvc-backup-${new Date().toISOString().replace(/[:.]/g, "-")}.json`,
        filters: [{ name: "TVC JSON backup", extensions: ["json"] }],
      });
      if (!managed && !path) return;
      const saved = managed ? await createLocalBackup() : await createBackupFile(path!, includeSettings);
      result = `Backup created ${formatDateTime(saved.created_at)}: ${saved.path}${saved.warning ? `\n${saved.warning}` : ""}`;
      warning = !!saved.warning;
    } catch (error) { actionError = `Backup was not created: ${String(error)}`; }
    finally { working = false; }
  }
  async function restore() {
    if (busy) return;
    working = true; actionError = null; result = null;
    try {
      const path = await open({ title: "Restore a TVC backup", filters: [{ name: "TVC JSON backup", extensions: ["json"] }], multiple: false });
      if (typeof path !== "string") return;
      const data = parseBackup(await readTextFile(path));
      const confirmed = await openConfirmDialog({ title: "Restore this backup?", message: describeBackup(data), type: "danger", confirmLabel: "Replace from backup", cancelLabel: "Keep current data" });
      if (!confirmed) return;
      const restored = await restoreBackup(data, waitForPendingPreferenceSaves);
      warning = !!(restored.quarantined || restored.episodes_orphaned || restored.warnings.length);
      result = `Restored ${restored.shows_imported} shows, ${restored.episodes_imported} episodes, ${restored.movies_imported} movies and ${restored.predictions_imported} predictions.${restored.settings_restored ? " Portable settings restored too." : " Your settings were kept."} Restart TVC to finish loading the restored data.${warning ? ` ${restored.quarantined} shows need matching; ${restored.episodes_orphaned} legacy episodes could not be matched. ${restored.warnings.join(" ")}` : ""}`;
    } catch (error) { actionError = `Restore failed: ${String(error)}`; }
    finally { working = false; }
  }
  async function restart() {
    try { await relaunch(); }
    catch (error) { actionError = `Could not restart TVC: ${String(error)}. Close and reopen it to finish restoring.`; }
  }
</script>

<div class="space-y-4">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <p class="text-sm text-text-muted">Keep a local copy of your library, tiers, predictions and history.</p>
    <div class="flex flex-wrap gap-2">
      <button type="button" class="focus-button" disabled={busy} onclick={() => create()}><Download size={16} />Create backup</button>
      <button type="button" class="focus-button" disabled={busy} onclick={restore}><Upload size={16} />Restore backup</button>
    </div>
  </div>
  {#if actionError || getBackupError()}
    <div role="alert" class="p-3 rounded-lg border border-error/30 bg-error/10 text-error text-sm break-words">
      <p class="flex items-start gap-2"><CircleX size={16} class="shrink-0 mt-0.5" /><span>{actionError ?? getBackupError()}</span></p>
      {#if getBackupError()}<button type="button" class="underline ml-2" disabled={isBackupLoading()} onclick={() => loadBackupStatus()}>Reload status</button>{/if}
    </div>
  {/if}
  {#if result}
    <div role="status" class="p-3 rounded-lg border text-sm whitespace-pre-line break-words {warning ? 'border-warning/30 bg-warning/10 text-warning' : 'border-success/30 bg-success/10 text-success'}">
      <p class="flex items-start gap-2">{#if warning}<AlertTriangle size={16} class="shrink-0 mt-0.5" />{:else}<Check size={16} class="shrink-0 mt-0.5" />{/if}<span>{result}</span></p>
      {#if restartRequired}<button type="button" class="focus-button mt-3" onclick={restart}><RefreshCw size={16} />Restart now</button>{/if}
    </div>
  {/if}
  {#if restartRequired && !result}
    <div role="status" class="focus-panel p-4 text-sm"><p>Your backup was restored. Restart TVC to load the restored data and settings.</p><button type="button" class="focus-button mt-3" onclick={restart}><RefreshCw size={16} />Restart now</button></div>
  {/if}
  <label class="flex items-start gap-2 text-sm text-text-muted">
    <input type="checkbox" bind:checked={includeSettings} disabled={busy} class="mt-1 accent-accent" />
    Include portable settings in the backup I create (useful when moving to another laptop).
  </label>
  <section class="focus-panel p-4 space-y-3" aria-label="Backup status">
    <div class="flex flex-wrap gap-x-6 gap-y-1 text-sm">
      <span>Last successful backup: <strong>{status?.last_success_at ? formatDateTime(status.last_success_at) : isBackupLoading() ? "Loading…" : "Never"}</strong></span>
      {#if status?.running || working}<span role="status" class="flex items-center gap-2 text-accent"><RefreshCw size={14} class="animate-spin" />Working…</span>{/if}
    </div>
    {#if status?.last_success_path}<p class="text-xs text-text-muted break-all">{status.last_success_path}</p>{/if}
    {#if status?.last_error}<p class="flex items-start gap-2 text-sm text-warning break-words"><AlertTriangle size={16} class="shrink-0 mt-0.5" />{status.last_error}</p>{/if}
  </section>
  {#if status}
    <section class="focus-panel p-4 space-y-4" aria-label="Automatic backups">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div><h2 class="text-base font-semibold">Automatic local backups</h2><p class="text-xs text-text-muted">Runs while TVC is open or in the tray; overdue backups run after reopening.</p></div>
        <button type="button" class="focus-button" disabled={busy || !status.preferences.folder || !!getBackupError()} onclick={() => create(true)}>Back up now</button>
      </div>
      <div class="flex flex-wrap items-center gap-x-6 gap-y-3">
        <label class="flex items-center gap-2 text-sm">Automatic backup
          <select class="bg-surface border border-border rounded-lg px-3 py-2" value={status.preferences.frequency} disabled={busy || !!getBackupError()}
            onchange={(event) => { const frequency = event.currentTarget.value as BackupFrequency; event.currentTarget.value = status!.preferences.frequency; void configure({ frequency }); }}>
            <option value="off">Off</option><option value="daily">Every day</option><option value="weekly">Every week</option><option value="monthly">Every month</option>
          </select>
        </label>
        <label class="flex items-center gap-2 text-sm">Keep
          <select class="bg-surface border border-border rounded-lg px-3 py-2" value={status.preferences.retention} disabled={busy || !!getBackupError()}
            onchange={(event) => { const retention = Number(event.currentTarget.value); event.currentTarget.value = String(status!.preferences.retention); void configure({ retention }); }}>
            {#each [1,3,5,10,20,50] as count}<option value={count}>{count} backup{count === 1 ? "" : "s"}</option>{/each}
          </select>
        </label>
        {#if isBackupSaving()}<span role="status" class="text-xs text-text-muted">Saving…</span>{/if}
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <button type="button" class="focus-button" disabled={busy || !!getBackupError()} onclick={selectFolder}><FolderOpen size={16} />Choose folder</button>
        <p class="text-sm text-text-muted break-all flex-1 min-w-0">{status.preferences.folder ?? "Choose a folder before enabling automatic backups."}</p>
      </div>
      <label class="flex items-center gap-2 text-sm text-text-muted"><input type="checkbox" checked={status.preferences.include_settings} disabled={busy || !!getBackupError()} class="accent-accent" onchange={(event) => { const include_settings = event.currentTarget.checked; event.currentTarget.checked = status!.preferences.include_settings; void configure({ include_settings }); }} />Include portable settings in automatic backups</label>
      {#if status.next_due_at}<p class="text-xs text-text-muted">Next backup: {formatDateTime(status.next_due_at)}. Every day is 24 hours, every week is 7 days, every month is one calendar month.</p>{/if}
      <p class="text-xs text-text-muted">Only TVC’s automatic backup files for this app profile are removed by retention, after a new backup succeeds. Manual backups and other files are kept. Failed attempts wait until the next interval; use Back up now to retry sooner.</p>
    </section>
  {/if}
  <details class="text-sm text-text-muted">
    <summary class="cursor-pointer py-1">What is included?</summary>
    <div class="space-y-2 pt-2">
      <p>All new backups include shows, episodes, movies, watched state, notes, ratings, tiers, awards, predictions, Plex title corrections, scrobble logs and change history.</p>
      <p>Portable settings add Appearance, General and calendar preferences, Notifications and quiet hours, your tier preset, and Racing series choices, colours and reminders.</p>
      <p>Credentials, Windows startup registration, Plex/Sonarr/Radarr connections, backup folders and schedules, TMDB sync schedules, custom Racing feed URLs and downloaded Racing events are excluded. Set up these connections on the new laptop.</p>
      <p>Restore replaces the data described in its confirmation. Create a backup of the current app first if you need to keep it. Older v1/v2/v3 backups are supported and keep your existing settings.</p>
    </div>
  </details>
</div>
