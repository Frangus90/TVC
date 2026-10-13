# Focus release preparation

## Scope

Prepare the Focus branch for the owner's **0.15.0** release, including the approved colourblind improvements, final build/interaction checks and a human-readable changelog. Do not publish a release, create a release tag, merge main, run the release script or manually change app versions.

## Safeguards

- Stay on `codex/focus-redesign`; preserve main and the pre-Focus tag/bundle.
- Never access the installed release database or sidecars. All runtime checks use verified dev targets or disposable/in-memory fixtures.
- Build production installers without running them. Native smoke tests must use dev identity/database.
- Preserve all tier assignments, custom colours, watch history, preferences and existing actions. No released migration edits.

## Plan

1. Implement measured status/accent contrast, consistent red–green support, readable custom-colour labels and visible status words/icons. Keep one existing Appearance option and add a compact preview.
2. Verify interactions at 1080p and 4K, plus colourblind and larger-text modes. Exercise backup/export, sync, restart/navigation guards and tier movement with disposable data.
3. Run full project checks and compile native release code. Build Windows NSIS/MSI installers locally without publication or version changes.
4. Review the final diff, update the 0.15.0 Unreleased changelog and record actual validation results and remaining external/manual limits. Commit and push only the Focus branch.

## Ownership

- Theme: shared palette/contrast/hover/focus, Appearance preview and contrast regression tests.
- Statuses: awards/export, notifications, Data/settings feedback and statistics.
- Labels: libraries, tiers, racing, title details and episode picker.
- Root: calendar/shared integration, native/build validation, UI checks, documentation and checkpoints.

## Progress

- Research and existing dev/release database selection verified. Starting from clean pushed branch `0099092`.
- Implementation and source checks complete. Release script has not been run.

## Implemented

- Retained the existing colourblind preference and added a compact Appearance preview. Positive/negative results use blue/orange in that mode; statuses also carry visible words and distinct symbols in both modes.
- Brighter status foregrounds pass contrast checks against the actual dark surfaces and tints up to 30%. Chosen accent fills remain unchanged; normal/hover button ink, accent labels and focus outlines use measured contrast.
- Tier and racing names use readable neutral text alongside their saved decorative colours. Selection checks, Appearance/General switches and watched details are clearer.
- Awards and exported picks explicitly say Correct, Incorrect and Pending. Notifications show Unread and clearer timestamps, with keyboard reading support.
- Data feedback, common messages and confirmation dialogs use the shared status colours while preserving existing actions and safety prompts.
- Month/Week keep readable minimum day widths and scroll horizontally in smaller windows. No horizontal scrollbar is needed at the tested monitor sizes.
- Updated the human-readable **0.15.0 Unreleased** changelog and linked the earlier research to this implementation record.

## Validation

- Final `npm run check`: **0 Svelte errors/warnings, 78 frontend tests passed, 119 Rust tests passed, 1 live Wikipedia test ignored, production frontend build passed**.
- New contrast regressions cover both actual CSS palettes, tinted panels, watched badges, every offered accent preset, dark custom colours, hover ink and focus. A review found and corrected the alternative-palette test selector so it cannot accidentally test only the standard colours.
- Production Rust configuration compiled with `cargo check --release --locked --offline`.
- Independent review found no action/assignment/storage regressions. Additional hover and General-switch contrast issues found during review were fixed and rechecked.
- Real compiled UI with in-memory IPC fixtures: all nine main pages checked at **1920×1080** and **3840×2160**, including 150% text, compact spacing and hidden posters. No measured document horizontal overflow.
- Month/Week/Agenda watched badges stay within their cards at **900×600**, **1200×800**, 1080p and 4K with 150% text. Calendar horizontal scrolling is local to Month/Week when needed.
- Confirmed colourblind preview changes, readable dark custom accent labels/focus, white checks on a dark selection fill and dark checks on the Focus fill. Greyscale UI retains explicit state words and symbols.
- Appearance preview also remained readable at the offered 75% text setting, then was restored to 100% for the final proof image.
- Exercised prediction results and PNG rendering through the intercepted file-write boundary, backup creation feedback, restore preview/cancellation/confirmation, persistent restart action, inert navigation and blocked Ctrl K until restart, successful sync retry and retained full-run report, watched/unwatched detail toggles, notification Space-to-read, and actual keyboard tier reordering.
- Preview console had no errors or warnings after these interactions. Preview APIs never opened an installed database, external service or native file dialog.

### Installer build

Local NSIS and MSI builds use the ignored `plans/release-validation/tauri.validation.json`, which changes only the product name to **TVC Release Validation** and disables updater-artifact signing. The builds use `--locked --offline --ci --no-sign`; the installers are never run. This keeps existing release installer filenames distinct and does not publish or require signing credentials.

The app manifests remain **0.14.8**, as required: the owner's release script will create the actual **0.15.0** versions, signatures, update manifest and release. Validation packages are test artifacts, not files to distribute.

Final NSIS and MSI builds **passed** after the last source and changelog changes. The resulting files were moved into the ignored `plans/release-validation/` folder. Existing release installer files were kept.

| Validation package | Bytes | SHA-256 |
| --- | ---: | --- |
| `TVC Release Validation_0.14.8_x64-setup.exe` | 4,296,827 | `182249A14F56F867519592030BD88016E6BB885D7AE6FC46357756E43E056C26` |
| `TVC Release Validation_0.14.8_x64_en-US.msi` | 5,931,008 | `283F1C1D0D1892FBDB6D6C6B309A45229B5EE6542ACAF77F6672A088D10F3A16` |

### Visual evidence

Ignored local screenshots:

- `focus-redesign-screenshots/release-colourblind-1080p.jpg`
- `focus-redesign-screenshots/release-colourblind-4k.jpg`
- `focus-redesign-screenshots/release-colourblind-greyscale.jpg`

## Practical limits and owner handoff

- Native desktop controls are unavailable in this session. Live tray/Windows notification delivery, startup registration, native file dialogs and actual process relaunch were not automated; their relevant code compiled and native command/file logic was tested with disposable data.
- Live TMDB, Plex, Sonarr/Radarr and Wikipedia were not contacted. Existing failure handling remains; connection-specific behaviour needs the owner's normal dev setup. Pointer tier dragging was reviewed; keyboard dragging/reordering was exercised.
- Greyscale and contrast were checked; a protan/deutan simulator was not available through the enabled browser controls. Final palette preference should be checked by the actual user with red–green difficulty.
- Palette calculations assume the documented dark surfaces/tints; future new backgrounds need their own contrast check. Colour remains supplemental to text/icons.
- The final candidate stays on the Focus branch. Main, pre-Focus recovery tag/bundle, version files and released migrations remain unchanged. No release database or sidecar was accessed.
- No release/tag/update manifest was created, and no installer was installed. Publishing remains the owner's action.
