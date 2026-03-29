<script lang="ts">
  import { onMount } from "svelte";
  import {
    Play,
    Square,
    Copy,
    Check,
    AlertTriangle,
    RefreshCw,
  } from "lucide-svelte";
  import {
    isLoading,
    getError,
    getConfig,
    getServerStatus,
    getScrobbleLog,
    toggleServer,
    updateConfig,
    loadScrobbleLog,
    getWebhookUrl,
    initPlexTab,
  } from "../../stores/plex.svelte";
  import { formatDateTime } from "../../utils/dateFormat";

  onMount(() => {
    initPlexTab();
  });

  let copied = $state(false);
  let portInput = $state(9876);

  $effect(() => {
    const config = getConfig();
    portInput = config.port;
  });

  async function handleToggle() {
    const status = getServerStatus();
    await toggleServer(!status.running);
  }

  async function handlePortChange() {
    const config = getConfig();
    if (portInput !== config.port && portInput >= 1024 && portInput <= 65535) {
      await updateConfig({ ...config, port: portInput });
    }
  }

  function copyWebhookUrl() {
    navigator.clipboard.writeText(getWebhookUrl());
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  const loading = $derived(isLoading());
  const error = $derived(getError());
  const serverStatus = $derived(getServerStatus());
  const scrobbleLog = $derived(getScrobbleLog());
</script>

<div class="space-y-6">
  <!-- Server Status -->
  <div class="bg-background rounded-lg p-4 border border-border">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div
          class="w-3 h-3 rounded-full {serverStatus.running ? 'bg-available animate-pulse' : 'bg-text-muted'}"
        ></div>
        <div>
          <p class="font-medium">
            Webhook Server {serverStatus.running ? "Running" : "Stopped"}
          </p>
          {#if serverStatus.running && serverStatus.port}
            <p class="text-sm text-text-muted">Listening on port {serverStatus.port}</p>
          {/if}
        </div>
      </div>
      <button
        type="button"
        onclick={handleToggle}
        disabled={loading}
        class="px-4 py-2 text-sm rounded-lg transition-colors flex items-center gap-2 {serverStatus.running
          ? 'bg-red-500/20 text-red-400 hover:bg-red-500/30'
          : 'bg-available/20 text-available hover:bg-available/30'} disabled:opacity-50"
      >
        {#if loading}
          <RefreshCw class="w-4 h-4 animate-spin" />
        {:else if serverStatus.running}
          <Square class="w-4 h-4" />
          Stop
        {:else}
          <Play class="w-4 h-4" />
          Start
        {/if}
      </button>
    </div>
  </div>

  <!-- Configuration -->
  <div class="space-y-4">
    <h3 class="font-medium">Configuration</h3>

    <!-- Port -->
    <div>
      <label for="plex-port" class="block text-sm text-text-muted mb-1">Webhook Port</label>
      <div class="flex gap-2">
        <input
          id="plex-port"
          type="number"
          min="1024"
          max="65535"
          bind:value={portInput}
          onblur={handlePortChange}
          disabled={serverStatus.running}
          class="w-32 px-3 py-2 rounded border border-border bg-surface text-text outline-none focus:ring-2 focus:ring-accent disabled:opacity-50 disabled:cursor-not-allowed"
        />
        {#if serverStatus.running}
          <p class="text-sm text-text-muted self-center">Stop server to change port</p>
        {/if}
      </div>
    </div>

    <!-- Webhook URL -->
    <div>
      <p class="block text-sm text-text-muted mb-1">Webhook URL</p>
      <div class="flex gap-2">
        <code class="flex-1 px-3 py-2 bg-background rounded border border-border text-sm font-mono truncate">
          {getWebhookUrl()}
        </code>
        <button
          type="button"
          onclick={copyWebhookUrl}
          class="px-3 py-2 bg-surface-hover hover:bg-surface-hover/80 rounded transition-colors flex items-center gap-2 text-sm"
        >
          {#if copied}
            <Check class="w-4 h-4 text-available" />
            Copied!
          {:else}
            <Copy class="w-4 h-4" />
            Copy
          {/if}
        </button>
      </div>
    </div>

    <!-- Setup Instructions -->
    <div class="bg-accent/10 rounded-lg p-4 border border-accent/30">
      <h4 class="font-medium text-accent mb-2">Setup Instructions</h4>
      <ol class="text-sm text-text-muted space-y-1 list-decimal list-inside">
        <li>Start the webhook server above</li>
        <li>Open Plex Web and go to Settings</li>
        <li>Navigate to Account &rarr; Webhooks</li>
        <li>Click "Add Webhook" and paste the URL above</li>
        <li>Watch content to 90%+ completion to trigger scrobbles</li>
      </ol>
      <p class="text-xs text-text-muted mt-3 flex items-center gap-1">
        <AlertTriangle class="w-3 h-3" />
        Requires Plex Pass subscription
      </p>
    </div>
  </div>

  <!-- Recent Scrobbles -->
  <div>
    <div class="flex items-center justify-between mb-3">
      <h3 class="font-medium">Recent Scrobbles</h3>
      <button
        type="button"
        onclick={loadScrobbleLog}
        class="text-xs text-text-muted hover:text-text flex items-center gap-1"
      >
        <RefreshCw class="w-3 h-3" />
        Refresh
      </button>
    </div>

    {#if scrobbleLog.length === 0}
      <div class="text-center py-8 text-text-muted">
        <p>No scrobbles yet</p>
        <p class="text-sm mt-1">Watch something in Plex to see it here</p>
      </div>
    {:else}
      <div class="space-y-2 max-h-64 overflow-y-auto">
        {#each scrobbleLog as entry}
          <div class="bg-background rounded-lg p-3 border border-border">
            <div class="flex items-start justify-between gap-2">
              <div class="flex-1 min-w-0">
                <p class="font-medium truncate">
                  {#if entry.media_type === "episode" && entry.show_name}
                    {entry.show_name}
                    <span class="text-text-muted font-normal">
                      S{String(entry.season_number).padStart(2, "0")}E{String(entry.episode_number).padStart(2, "0")}
                    </span>
                  {:else}
                    {entry.raw_title}
                    {#if entry.year}
                      <span class="text-text-muted font-normal">({entry.year})</span>
                    {/if}
                  {/if}
                </p>
                <p class="text-xs text-text-muted">{formatDateTime(entry.scrobbled_at)}</p>
              </div>
              <div class="flex-shrink-0">
                {#if entry.matched_entity_id}
                  <span class="px-2 py-0.5 bg-available/20 text-available text-xs font-medium rounded">
                    Matched
                  </span>
                {:else}
                  <span class="px-2 py-0.5 bg-yellow-500/20 text-yellow-400 text-xs font-medium rounded">
                    Not Found
                  </span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Error -->
  {#if error}
    <div class="p-3 rounded-lg bg-red-500/10 text-red-400 text-sm text-center">
      {error}
    </div>
  {/if}
</div>
