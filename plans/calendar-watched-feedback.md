# Calendar watched feedback

## Request and scope

Make watched episodes and movies easy to recognise in the Focus calendar. Keep the current compact layout, readable titles, custom title colours and independent detail/watch/scheduling actions.

## Plan

1. Research Month, Week, Agenda and the day-details dialog. Current grey checks and 5–8% grey backgrounds are too subtle; Month has no visible Watched label, and Agenda hides its status on smaller widths.
2. Add one reusable check/text badge, stronger calendar-specific watched background/border colours and filled watch controls. Keep watched badges visible when rows wrap, posters are hidden or text is enlarged. Preserve custom left stripes and all handlers.
3. Validate Svelte/build/regression checks and visually exercise mark watched/unwatched in isolated memory fixtures at 1920×1080 and 3840×2160, including compact spacing, colourblind colours and larger text.

## Safeguards

- Stay on `codex/focus-redesign`; preserve main and the original pre-Focus backup.
- Use only mock/disposable data. Never access release `tvc.db` or its sidecars.
- No database, migration, version or watch-history logic changes.
- Use plain-language changelog notes.

## Completed — 13.10.2026

- Added a shared **✓ Watched** badge to Month, Week, Agenda and day details, with a stronger background, visible border and filled watch control. Titles and episode codes remain readable.
- Watched movie cards now say **Completed watching** rather than **Planned watching**. Unwatched movies retain the planned label.
- Kept custom episode colour stripes, title-opening actions, watch/unwatch actions and scheduling/context menus. Watch controls expose their current state and action to assistive technology.
- Badges wrap in narrow Month/Week cards and stay visible in the narrow Agenda layout. Colourblind-friendly appearance uses a distinct neutral-blue watched colour.
- Updated the unreleased changelog in plain language.

## Validation

- `npm run check`: 0 Svelte errors or warnings, 75 frontend tests passed, 119 Rust tests passed (1 live Wikipedia test ignored), production frontend build passed.
- Independent read-only review confirmed the existing calendar handlers were preserved; its narrow Week wrapping concern was fixed before the final checks.
- Visually tested the compiled app with isolated in-memory fixtures at **1920×1080** and **3840×2160**, including 150% text with compact spacing, hidden posters and colourblind colours. Month, Week and Agenda retained their watched badges within cards without horizontal overflow.
- Also checked 75% text and a 1024×768 window with 150% text. Watched badges stayed visible in Week and Agenda.
- Marking and unmarking episodes and movies updated the badge, background and control state immediately. Verified title details open independently and day-details counts update after watch/unwatch.
- No installed database was accessed. Native database/file APIs in the preview were replaced with in-memory fixtures, and even the compiled preview was explicitly configured for the dev environment.
- Assumes the existing `watched` field correctly represents watch state; this change improves its presentation and does not change watch-history storage. Hidden events still use the existing **+ more** control to open day details.
- Main and the pre-Focus recovery tag/bundle remain unchanged.

### Visual evidence

Local screenshots (ignored preview artifacts):

- `focus-redesign-screenshots/calendar-watched-month-1080p.jpg`
- `focus-redesign-screenshots/calendar-watched-month-4k.jpg`
- `focus-redesign-screenshots/calendar-watched-week-1080p.jpg`
- `focus-redesign-screenshots/calendar-watched-day-1080p.jpg`
- `focus-redesign-screenshots/calendar-watched-agenda-large-text.jpg`
