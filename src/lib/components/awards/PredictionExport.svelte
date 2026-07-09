<script lang="ts">
  // Off-screen render target for the shareable predictions PNG. Inline styles are
  // used so the screenshot is self-contained and doesn't depend on app CSS.
  interface Pick {
    category: string;
    pick: string;
    result: "win" | "miss" | null;
  }
  let {
    ceremonyName,
    picks,
    appVersion,
    date,
  }: {
    ceremonyName: string;
    picks: Pick[];
    appVersion: string;
    date: string;
  } = $props();
</script>

<div
  style="width:640px; background:#0f0f0f; color:#e5e5e5; padding:28px;
         font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;"
>
  <div style="display:flex; align-items:baseline; justify-content:space-between;">
    <div style="font-size:22px; font-weight:700;">{ceremonyName}</div>
    <div style="font-size:13px; color:#a3a3a3;">My picks</div>
  </div>
  <div style="height:1px; background:#262626; margin:12px 0 16px;"></div>

  <div style="display:flex; flex-direction:column; gap:12px;">
    {#each picks as p}
      <div style="display:flex; align-items:flex-start; gap:10px;">
        <div style="flex:1; min-width:0;">
          <div style="font-size:11px; color:#a3a3a3; text-transform:uppercase; letter-spacing:0.04em;">
            {p.category}
          </div>
          <div style="font-size:15px; font-weight:600; line-height:1.3;">{p.pick}</div>
        </div>
        {#if p.result === "win"}
          <div style="color:#22c55e; font-size:16px; font-weight:700; line-height:1.3;">✓</div>
        {:else if p.result === "miss"}
          <div style="color:#ef4444; font-size:16px; font-weight:700; line-height:1.3;">✗</div>
        {/if}
      </div>
    {/each}
  </div>

  <div style="margin-top:22px; font-size:11px; color:#6b7280;">
    TVC v{appVersion} · {date}
  </div>
</div>
