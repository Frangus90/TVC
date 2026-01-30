<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import {
    X,
    Server,
    Plus,
    Edit2,
    Trash2,
    RefreshCw,
    Check,
    AlertTriangle,
    Download,
    ChevronRight,
    CheckSquare,
    Square,
  } from "lucide-svelte";
  import {
    isModalOpen,
    getActiveTab,
    isLoading,
    getError,
    getSuccessMessage,
    getServers,
    getEditingServer,
    isTestingConnection,
    getTestResult,
    getSelectedServer,
    getLibraryItems,
    getSelectedItems,
    isImporting,
    getImportResult,
    closeArrSettings,
    setActiveTab,
    setEditingServer,
    clearTestResult,
    testConnection,
    addServer,
    updateServer,
    deleteServer,
    setSelectedServer,
    toggleItemSelection,
    selectAllItems,
    deselectAllItems,
    importSelected,
    getSelectedItemsCount,
    type ArrServer,
    type ArrServerRequest,
  } from "../stores/arr.svelte";
  import { openConfirmDialog } from "../stores/confirmDialog.svelte";
  import { formatDateTime } from "../utils/dateFormat";

  // Form state for add/edit
  let formName = $state("");
  let formType = $state<"sonarr" | "radarr">("sonarr");
  let formBaseUrl = $state("");
  let formApiKey = $state("");
  let showApiKey = $state(false);

  // Reset form when editing server changes
  $effect(() => {
    const editing = getEditingServer();
    if (editing) {
      formName = editing.name;
      formType = editing.type as "sonarr" | "radarr";
      formBaseUrl = editing.base_url;
      formApiKey = editing.api_key;
    } else {
      formName = "";
      formType = "sonarr";
      formBaseUrl = "";
      formApiKey = "";
    }
    showApiKey = false;
    clearTestResult();
  });

  function handleTestConnection() {
    testConnection(formBaseUrl, formApiKey, formType);
  }

  // Check if we're editing an existing server (id > 0) vs adding a new one (id === 0)
  function isEditingExistingServer(): boolean {
    const editing = getEditingServer();
    return editing !== null && editing.id > 0;
  }

  async function handleSaveServer() {
    const request: ArrServerRequest = {
      name: formName,
      type: formType,
      base_url: formBaseUrl,
      api_key: formApiKey,
    };

    const editing = getEditingServer();
    if (editing && editing.id > 0) {
      // Update existing server
      await updateServer(editing.id, request);
    } else {
      // Add new server
      await addServer(request);
    }
  }

  async function handleDeleteServer(server: ArrServer) {
    const confirmed = await openConfirmDialog({
      title: "Delete Server",
      message: `Are you sure you want to delete "${server.name}"?`,
      type: "danger",
      confirmLabel: "Delete",
      cancelLabel: "Cancel",
    });

    if (confirmed) {
      await deleteServer(server.id);
    }
  }

  function handleStartImport(server: ArrServer) {
    setActiveTab("import");
    setSelectedServer(server);
  }
</script>

{#if isModalOpen()}
  {@const activeTab = getActiveTab()}
  {@const loading = isLoading()}
  {@const error = getError()}
  {@const successMessage = getSuccessMessage()}
  {@const servers = getServers()}
  {@const editingServer = getEditingServer()}
  {@const testingConnection = isTestingConnection()}
  {@const testResult = getTestResult()}
  {@const selectedServer = getSelectedServer()}
  {@const libraryItems = getLibraryItems()}
  {@const selectedItems = getSelectedItems()}
  {@const importing = isImporting()}
  {@const importResult = getImportResult()}

  <!-- Backdrop -->
  <button
    type="button"
    transition:fade={{ duration: 150 }}
    class="fixed inset-0 bg-black/60 z-50"
    onclick={closeArrSettings}
    aria-label="Close modal"
  ></button>

  <!-- Modal -->
  <div
    transition:scale={{ duration: 200, start: 0.95, opacity: 0 }}
    class="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 bg-surface rounded-xl border border-border shadow-2xl w-[1100px] max-w-[95vw] h-[90vh] max-h-[90vh] flex flex-col"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-border">
      <div class="flex items-center gap-3">
        <Server class="w-5 h-5 text-accent" />
        <h2 class="text-lg font-semibold">Sonarr / Radarr Integration</h2>
      </div>
      <button
        type="button"
        onclick={closeArrSettings}
        class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
        aria-label="Close"
      >
        <X class="w-5 h-5 text-text-muted" />
      </button>
    </div>

    <!-- Tabs -->
    <div class="flex border-b border-border">
      <button
        type="button"
        onclick={() => setActiveTab("servers")}
        class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'servers'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
      >
        Servers
      </button>
      <button
        type="button"
        onclick={() => setActiveTab("import")}
        class="px-4 py-3 text-sm font-medium transition-colors {activeTab === 'import'
          ? 'text-accent border-b-2 border-accent'
          : 'text-text-muted hover:text-text'}"
      >
        Import Library
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto p-4">
      {#if activeTab === "servers"}
        <!-- Servers Tab -->
        {#if editingServer !== null || servers.length === 0}
          <!-- Add/Edit Form -->
          <div class="bg-background rounded-lg p-4 border border-border">
            <h3 class="font-medium mb-4">{isEditingExistingServer() ? "Edit Server" : "Add Server"}</h3>

            <div class="space-y-4">
              <div>
                <label for="server-name" class="block text-sm font-medium text-text-muted mb-1">Name</label>
                <input
                  id="server-name"
                  type="text"
                  bind:value={formName}
                  placeholder="My Sonarr Server"
                  class="w-full px-3 py-2 rounded border border-border bg-surface text-text placeholder:text-text-muted outline-none focus:ring-2 focus:ring-accent"
                />
              </div>

              <div>
                <label for="server-type" class="block text-sm font-medium text-text-muted mb-1">Type</label>
                <select
                  id="server-type"
                  bind:value={formType}
                  class="w-full px-3 py-2 rounded border border-border bg-surface text-text outline-none focus:ring-2 focus:ring-accent"
                >
                  <option value="sonarr">Sonarr (TV Shows)</option>
                  <option value="radarr">Radarr (Movies)</option>
                </select>
              </div>

              <div>
                <label for="server-url" class="block text-sm font-medium text-text-muted mb-1">Base URL</label>
                <input
                  id="server-url"
                  type="text"
                  bind:value={formBaseUrl}
                  placeholder="http://localhost:8989"
                  class="w-full px-3 py-2 rounded border border-border bg-surface text-text placeholder:text-text-muted outline-none focus:ring-2 focus:ring-accent"
                />
              </div>

              <div>
                <label for="server-api-key" class="block text-sm font-medium text-text-muted mb-1">API Key</label>
                <div class="relative">
                  <input
                    id="server-api-key"
                    type={showApiKey ? "text" : "password"}
                    bind:value={formApiKey}
                    placeholder="Your API key"
                    class="w-full px-3 py-2 pr-20 rounded border border-border bg-surface text-text placeholder:text-text-muted outline-none focus:ring-2 focus:ring-accent"
                  />
                  <button
                    type="button"
                    onclick={() => (showApiKey = !showApiKey)}
                    class="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-text-muted hover:text-text"
                  >
                    {showApiKey ? "Hide" : "Show"}
                  </button>
                </div>
              </div>

              {#if testResult}
                <div
                  class="p-3 rounded-lg text-sm {testResult.success
                    ? 'bg-available/10 text-available'
                    : 'bg-red-500/10 text-red-400'}"
                >
                  <div class="flex items-center gap-2">
                    {#if testResult.success}
                      <Check class="w-4 h-4" />
                    {:else}
                      <AlertTriangle class="w-4 h-4" />
                    {/if}
                    {testResult.message}
                  </div>
                </div>
              {/if}

              <div class="flex gap-3">
                <button
                  type="button"
                  onclick={handleTestConnection}
                  disabled={testingConnection || !formBaseUrl || !formApiKey}
                  class="px-4 py-2 text-sm bg-surface-hover hover:bg-surface-hover/80 text-text rounded-lg transition-colors flex items-center gap-2 disabled:opacity-50"
                >
                  {#if testingConnection}
                    <RefreshCw class="w-4 h-4 animate-spin" />
                    Testing...
                  {:else}
                    Test Connection
                  {/if}
                </button>

                <button
                  type="button"
                  onclick={handleSaveServer}
                  disabled={loading || !formName || !formBaseUrl || !formApiKey}
                  class="px-4 py-2 text-sm bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors flex items-center gap-2 disabled:opacity-50"
                >
                  {#if loading}
                    <RefreshCw class="w-4 h-4 animate-spin" />
                  {/if}
                  {isEditingExistingServer() ? "Update" : "Add"} Server
                </button>

                {#if editingServer}
                  <button
                    type="button"
                    onclick={() => setEditingServer(null)}
                    class="px-4 py-2 text-sm text-text-muted hover:text-text transition-colors"
                  >
                    Cancel
                  </button>
                {/if}
              </div>
            </div>
          </div>
        {/if}

        {#if servers.length > 0 && editingServer === null}
          <!-- Server List -->
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <h3 class="font-medium">Configured Servers</h3>
              <button
                type="button"
                onclick={() => setEditingServer({ id: 0, name: "", type: "sonarr", base_url: "", api_key: "", is_active: true, auto_sync_enabled: false, sync_interval_hours: 24, last_synced: null, added_at: null })}
                class="px-3 py-1.5 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded-lg transition-colors flex items-center gap-2"
              >
                <Plus class="w-4 h-4" />
                Add Server
              </button>
            </div>

            {#each servers as server}
              <div class="bg-background rounded-lg p-4 border border-border">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <div
                      class="w-10 h-10 rounded-lg flex items-center justify-center {server.type === 'sonarr'
                        ? 'bg-blue-500/20 text-blue-400'
                        : 'bg-orange-500/20 text-orange-400'}"
                    >
                      <Server class="w-5 h-5" />
                    </div>
                    <div>
                      <p class="font-medium">{server.name}</p>
                      <p class="text-sm text-text-muted">{server.type === "sonarr" ? "Sonarr" : "Radarr"} - {server.base_url}</p>
                    </div>
                  </div>

                  <div class="flex items-center gap-2">
                    <button
                      type="button"
                      onclick={() => handleStartImport(server)}
                      class="px-3 py-1.5 text-sm bg-accent/20 hover:bg-accent/30 text-accent rounded-lg transition-colors flex items-center gap-2"
                    >
                      <Download class="w-4 h-4" />
                      Import
                    </button>
                    <button
                      type="button"
                      onclick={() => setEditingServer(server)}
                      class="p-2 rounded-lg hover:bg-surface-hover transition-colors"
                      aria-label="Edit server"
                    >
                      <Edit2 class="w-4 h-4 text-text-muted" />
                    </button>
                    <button
                      type="button"
                      onclick={() => handleDeleteServer(server)}
                      class="p-2 rounded-lg hover:bg-red-500/20 transition-colors"
                      aria-label="Delete server"
                    >
                      <Trash2 class="w-4 h-4 text-red-400" />
                    </button>
                  </div>
                </div>

                {#if server.last_synced}
                  <p class="text-xs text-text-muted mt-2">Last synced: {formatDateTime(server.last_synced)}</p>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if servers.length === 0 && !editingServer}
          <div class="text-center py-12 text-text-muted">
            <Server class="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>No servers configured</p>
            <p class="text-sm mt-1">Add a Sonarr or Radarr server to get started</p>
          </div>
        {/if}
      {:else if activeTab === "import"}
        <!-- Import Tab -->
        {#if !selectedServer}
          <!-- Server Selection -->
          <div class="space-y-3">
            <h3 class="font-medium">Select a server to import from</h3>

            {#if servers.length === 0}
              <div class="text-center py-12 text-text-muted">
                <Server class="w-12 h-12 mx-auto mb-3 opacity-50" />
                <p>No servers configured</p>
                <button
                  type="button"
                  onclick={() => setActiveTab("servers")}
                  class="text-sm text-accent hover:text-accent/80 underline mt-2"
                >
                  Add a server first
                </button>
              </div>
            {:else}
              {#each servers as server}
                <button
                  type="button"
                  onclick={() => setSelectedServer(server)}
                  class="w-full bg-background rounded-lg p-4 border border-border hover:border-accent/50 transition-colors text-left flex items-center justify-between"
                >
                  <div class="flex items-center gap-3">
                    <div
                      class="w-10 h-10 rounded-lg flex items-center justify-center {server.type === 'sonarr'
                        ? 'bg-blue-500/20 text-blue-400'
                        : 'bg-orange-500/20 text-orange-400'}"
                    >
                      <Server class="w-5 h-5" />
                    </div>
                    <div>
                      <p class="font-medium">{server.name}</p>
                      <p class="text-sm text-text-muted">{server.type === "sonarr" ? "Sonarr" : "Radarr"}</p>
                    </div>
                  </div>
                  <ChevronRight class="w-5 h-5 text-text-muted" />
                </button>
              {/each}
            {/if}
          </div>
        {:else}
          <!-- Library View -->
          <div class="flex flex-col h-full gap-4">
            <div class="flex items-center justify-between">
              <button
                type="button"
                onclick={() => setSelectedServer(null)}
                class="text-sm text-text-muted hover:text-text flex items-center gap-1"
              >
                <ChevronRight class="w-4 h-4 rotate-180" />
                Back to servers
              </button>
              <div class="flex items-center gap-2">
                <span class="text-sm text-text-muted">
                  {getSelectedItemsCount()} selected
                </span>
                {#if libraryItems.filter((i) => !i.already_tracked).length > 0}
                  <button
                    type="button"
                    onclick={selectAllItems}
                    class="text-xs text-accent hover:text-accent/80"
                  >
                    Select All
                  </button>
                  <span class="text-text-muted">|</span>
                  <button
                    type="button"
                    onclick={deselectAllItems}
                    class="text-xs text-text-muted hover:text-text"
                  >
                    Deselect All
                  </button>
                {/if}
              </div>
            </div>

            <div class="bg-background rounded-lg p-3 border border-border flex items-center gap-3">
              <div
                class="w-10 h-10 rounded-lg flex items-center justify-center {selectedServer.type === 'sonarr'
                  ? 'bg-blue-500/20 text-blue-400'
                  : 'bg-orange-500/20 text-orange-400'}"
              >
                <Server class="w-5 h-5" />
              </div>
              <div>
                <p class="font-medium">{selectedServer.name}</p>
                <p class="text-sm text-text-muted">
                  {libraryItems.length} {selectedServer.type === "sonarr" ? "shows" : "movies"} in library
                </p>
              </div>
            </div>

            {#if loading}
              <div class="flex items-center justify-center py-12">
                <RefreshCw class="w-6 h-6 text-accent animate-spin" />
              </div>
            {:else if libraryItems.length === 0}
              <div class="text-center py-12 text-text-muted">
                <p>No items found in library</p>
              </div>
            {:else}
              <!-- Library List -->
              <div class="flex-1 overflow-y-auto border border-border rounded-lg">
                {#each libraryItems as item}
                  {@const isSelected = selectedItems.has(item.arr_id)}
                  <button
                    type="button"
                    disabled={item.already_tracked}
                    onclick={() => toggleItemSelection(item.arr_id)}
                    class="w-full flex items-center gap-3 px-4 py-2.5 border-b border-border last:border-b-0 transition-colors text-left {item.already_tracked
                      ? 'opacity-50 cursor-not-allowed bg-surface'
                      : isSelected
                        ? 'bg-accent/10'
                        : 'hover:bg-surface-hover'}"
                  >
                    <!-- Checkbox -->
                    <div class="flex-shrink-0">
                      {#if item.already_tracked}
                        <Check class="w-5 h-5 text-available" />
                      {:else if isSelected}
                        <CheckSquare class="w-5 h-5 text-accent" />
                      {:else}
                        <Square class="w-5 h-5 text-text-muted" />
                      {/if}
                    </div>

                    <!-- Title and Year -->
                    <div class="flex-1 min-w-0">
                      <p class="font-medium truncate">{item.title}</p>
                    </div>

                    <!-- Year -->
                    {#if item.year}
                      <span class="flex-shrink-0 text-sm text-text-muted">{item.year}</span>
                    {/if}

                    <!-- Tracked Badge -->
                    {#if item.already_tracked}
                      <span class="flex-shrink-0 px-2 py-0.5 bg-available/20 text-available text-xs font-medium rounded">
                        Tracked
                      </span>
                    {/if}
                  </button>
                {/each}
              </div>

              <!-- Import Button -->
              {#if getSelectedItemsCount() > 0}
                <div class="flex justify-end">
                  <button
                    type="button"
                    onclick={importSelected}
                    disabled={importing}
                    class="px-4 py-2 text-sm bg-accent hover:bg-accent/90 text-white rounded-lg transition-colors flex items-center gap-2 disabled:opacity-50"
                  >
                    {#if importing}
                      <RefreshCw class="w-4 h-4 animate-spin" />
                      Importing...
                    {:else}
                      <Download class="w-4 h-4" />
                      Import {getSelectedItemsCount()} {selectedServer.type === "sonarr" ? "Shows" : "Movies"}
                    {/if}
                  </button>
                </div>
              {/if}

              {#if importResult}
                <div class="p-4 rounded-lg bg-available/10 border border-available/30">
                  <div class="flex items-center gap-2 text-available">
                    <Check class="w-5 h-5" />
                    <span class="font-medium">Import Complete</span>
                  </div>
                  <p class="text-sm text-text-muted mt-2">
                    Imported: {importResult.imported} | Skipped: {importResult.skipped} | Failed: {importResult.failed}
                  </p>
                  {#if importResult.errors.length > 0}
                    <div class="mt-2 text-sm text-red-400">
                      {#each importResult.errors.slice(0, 3) as err}
                        <p>{err}</p>
                      {/each}
                      {#if importResult.errors.length > 3}
                        <p>...and {importResult.errors.length - 3} more errors</p>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      {/if}
    </div>

    <!-- Footer message -->
    {#if error}
      <div class="p-3 border-t border-border bg-red-500/10 text-red-400 text-sm text-center">
        {error}
      </div>
    {:else if successMessage}
      <div class="p-3 border-t border-border bg-available/10 text-available text-sm text-center">
        {successMessage}
      </div>
    {/if}
  </div>
{/if}
