# Colourblind support research

Research date: **13.10.2026**. Reviewed Focus branch at `0f2f887`.

## Implementation follow-up — 13.10.2026

The proposed first three implementation stages are now implemented on the Focus branch as part of [release preparation](focus-release-preparation.md): shared measured colours, readable normal/hover accent text and focus, visible status words/icons, neutral tier/racing labels with saved decorative colours, clearer unread notifications and an Appearance preview. The existing colourblind boolean preference is retained; no diagnosis selector or whole-app filter was added. A greyscale control exists only in the ignored test harness.

The research below records the earlier findings. Final checks, installer builds, visual evidence and any remaining manual limits are recorded in the release-preparation document.

## Scope and conclusion

Improve support for **red–green colour vision deficiency** while keeping TVC simple for its two users. This is research and a proposed implementation order; no app behaviour has changed.

Keep one colourblind-support option, extend it across meaningful statuses, and make important states understandable through visible words and distinct shapes. The recent watched-calendar badge is a useful model. Use a small preview in Appearance so users can judge the actual result.

## Research findings

- Important information needs a visible alternative to colour, such as text or shape. A screen-reader label alone does not solve this for a sighted colourblind user. [W3C: Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html).
- Aim for at least **4.5:1** contrast for normal text and **3:1** for qualifying large text. Measure against the actual background, including tinted and hover states. [W3C: Contrast (Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).
- Essential control indicators and state symbols generally need **3:1** against adjacent colours. This does not mean every decorative card border must meet that ratio when text/icons already provide the necessary information. [W3C: Non-text Contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html).
- Okabe and Ito recommend combining colour with shape, direct labels and clear brightness differences. Their palette is a useful starting point for category colours, but colours still need testing on TVC's dark backgrounds. [Color Universal Design](https://jfly.uni-koeln.de/color/).
- Red–green deficiency includes multiple types and severities; test both protan and deutan simulations rather than assuming one exact subtype. [National Eye Institute: Types of Color Vision Deficiency](https://www.nei.nih.gov/eye-health-information/eye-conditions-and-diseases/color-blindness/types-color-vision-deficiency).
- Edge provides protanopia, deuteranopia and greyscale simulations. Microsoft describes them as approximations; testing with the actual user remains necessary. [Microsoft: Emulate vision deficiencies](https://learn.microsoft.com/en-us/microsoft-edge/devtools/accessibility/emulate-vision-deficiencies).

## Current app findings

### Confirmed scope gaps

The Appearance switch currently changes four status variables and the new calendar watched colours. It does not consistently affect warning/error colours, notifications, statistics, custom tiers or racing colours.

- `src/app.css:155`: Upcoming `#0066cc`, Available `#00aa00`, Watched `#888888`, Premiere `#ff8800` in the alternative palette. Available remains green.
- `src/app.css:65`: watched calendar badges use a neutral blue-grey alternative.
- `src/lib/components/settings/AppearanceTab.svelte:108`: help text describes episode statuses only.
- `src/lib/components/awards/AwardsView.svelte:246`: winners use Available green while losing picks use hardcoded red. Trophy/check/cross/pending text already provide non-colour distinctions.
- `src/lib/components/StatisticsDashboard.svelte:43`: completion bars use fixed red/orange/yellow/green classes. Counts and percentages already explain the information.
- `src/lib/components/notifications/NotificationPopup.svelte:20` and Data/settings components use their own warning/error colours.

### Contrast examples

These are WCAG formula calculations from declared sRGB colours and the stated backgrounds, not runtime pixel measurements or a complete accessibility certification.

| Example | Approximate contrast | Assessment |
| --- | ---: | --- |
| Colourblind Upcoming `#0066cc` on calendar card `#22252e` | 2.75:1 | Too low for the small Upcoming text. |
| Standard Upcoming `#3b82f6` on the same card | 4.16:1 | Also below the normal-text target. |
| White selection check on Focus accent `#bba4ff` | 2.13:1 | Too low for an essential check symbol. |
| Normal Watched `#6b7280` on surface `#191b22` | 3.56:1 | Small watched counts outside the new badge need brighter text. |
| Current colourblind watched badge: `#17202e` on `#c0cbdc` | 9.99:1 | Good text/check contrast; retain this treatment. |

The Upcoming example is used in `src/lib/components/calendar/MonthView.svelte:266`. The definite white-check example is the scoped `.library-checkbox.checked` rule in `src/lib/components/LibraryView.svelte:376`. Some other controls declare `text-white`, but the global `.bg-accent` rule can override it: inspect computed styles before treating those as confirmed failures.

### Other useful improvements

- **Tiers:** some tier names/badges use arbitrary saved colour as text (`TierView.svelte:408`, `LibraryView.svelte:369`). Keep names in readable neutral text and use saved colour as a stripe/dot. Preserve tier names, colours and assignments; changing accessibility appearance must never invoke the destructive tier Apply action.
- **Racing:** saved series colours also colour abbreviations and badges (`RaceCalendar.svelte:247`, `:313`). Keep F1/WEC/etc. readable independently of the saved stripe colour.
- **Notifications:** unread status relies on a small accent dot and faint background (`NotificationCenter.svelte:162`, `:171`). Add a compact Unread label or clear weight/shape difference. Review timestamps currently reduced to 60% opacity (`:186`).
- **Custom accents and hover:** valid hex alone does not guarantee readable accent text or focus outlines. An accent of `#121318` contrasts only 1.08:1 against the surface. The preset Purple primary-button text measures about 4.38:1, and its hover state about 3.15:1 with the retained foreground. Review `theme.svelte.ts:159–179` and `app.css:269–270`: choose foreground using actual contrast on both normal and hover colours, and ensure focus remains visible even with a dark saved accent.
- **Exports:** award exports already use check/cross/Pending; tier exports retain names and positions. Carry any new clearer labels/readable text into exports too.

## Recommended behaviour

### 1. Fix contrast first

Use a lighter, measured Upcoming colour in both palettes and brighter watched counts outside the new calendar badge. Use the accent-foreground mechanism for essential selection checks, improve its choice using actual foreground/background contrast, and check that choice again on hover. Measure text, symbols and tinted backgrounds separately.

### 2. Make statuses clear across the app

Use a small shared set of status colours rather than changing unrelated red/green classes globally. For red–green support, trial light blue for positive results and amber/orange for negative or attention states. Final shades and roles should be checked in context and with the user.

| State | Visible cue to retain or add |
| --- | --- |
| Watched | Check + Watched badge + filled watch control. |
| Available | Open-circle/play symbol + Available. |
| Upcoming | Clock + Upcoming. |
| Correct prediction | Check + Correct. |
| Incorrect prediction | Cross + Incorrect. |
| Pending prediction | Clock/ellipsis + Pending. |
| Success | Check + explicit result text. |
| Warning or failure | Distinct warning/error symbol + clear explanation. |

Use text/icons in the standard appearance too. Keep existing tier letters/names, racing abbreviations and numerical progress labels. Category colours and decorative custom stripes need not encode status.

### 3. Keep the settings simple

- Keep the existing saved on/off option; clarify that it covers status colours across TVC.
- Add a small sample showing watched, upcoming, correct, incorrect and attention states beside the option, using the actual components/styles.
- Keep custom accents and tier/racing colours intact while ensuring their text remains readable.
- Consider a separate stronger-contrast option only if user testing shows a need. A diagnosis selector, severity slider or whole-app colour filter would add complexity without resolving the current gaps.

## Proposed implementation and validation

1. Fix status/selection contrast and confirm rendered styles on dark, tinted and hover backgrounds.
2. Apply consistent status colours and words/icons to calendars, details, awards, Data and notifications. Improve custom-colour label readability and exports.
3. Add the Appearance preview and updated help text.
4. Test protanopia, deuteranopia and greyscale previews at 1920×1080 and 3840×2160, plus 75–150% text and compact spacing. Confirm statuses remain understandable without colour.
5. Ask the user to check a few practical tasks: distinguish watched/unwatched, find an upcoming episode, identify correct/incorrect/pending picks, read sync failures and tier names, recognise unread notifications.
6. Run the appropriate Svelte, frontend and Rust checks after implementation. Add contrast checks for shared palette/foreground logic where they prevent a real regression.

All future work stays on `codex/focus-redesign` and uses only `tvc_dev.db` or disposable/in-memory fixtures. No database was accessed during this research. No migrations, versions, changelog or application source were changed.
