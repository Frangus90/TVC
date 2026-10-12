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
  style="width:640px; background:#121318; color:#eff0f6; padding:32px;
         font-family:'Segoe UI Variable','Segoe UI',system-ui,sans-serif;"
>
  <div style="font-size:10px; font-weight:600; color:#a0a5b6; letter-spacing:1.5px; margin-bottom:10px;">TVC · AWARDS</div>
  <div style="display:flex; align-items:baseline; justify-content:space-between;">
    <div style="font-size:24px; font-weight:600; line-height:1.25;">{ceremonyName}</div>
    <div style="font-size:12px; color:#a0a5b6; flex-shrink:0; margin-left:20px;">My picks</div>
  </div>
  <div style="height:1px; background:#2b2e38; margin:20px 0;"></div>

  <div style="display:flex; flex-direction:column; gap:12px;">
    {#each picks as p}
      <div style="display:flex; align-items:flex-start; gap:14px; padding:14px; background:#191b22; border:1px solid #2b2e38; border-radius:10px;">
        <div style="flex:1; min-width:0;">
          <div style="font-size:10px; color:#a0a5b6; text-transform:uppercase; letter-spacing:0.08em; margin-bottom:6px;">
            {p.category}
          </div>
          <div style="font-size:15px; font-weight:600; line-height:1.4;">{p.pick}</div>
        </div>
        {#if p.result === "win"}
          <div style="color:#8ed4b0; font-size:16px; font-weight:600; line-height:1.4;">✓</div>
        {:else if p.result === "miss"}
          <div style="color:#ed9fae; font-size:16px; font-weight:600; line-height:1.4;">✗</div>
        {:else}
          <div style="color:#e4c17a; font-size:11px; line-height:1.4;">Pending</div>
        {/if}
      </div>
    {/each}
  </div>

  <div style="margin-top:24px; font-size:11px; color:#a0a5b6;">
    TVC v{appVersion} · {date}
  </div>
</div>
