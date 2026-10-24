# Release 0.15.1 preparation

## Scope and safeguards

Prepare the completed 0.15.1 additions on `codex/focus-redesign` for the owner's release. Review the human-readable changelog and current changes, run the project checks, and build local Windows validation packages. Do not run `scripts/release.ps1`, publish, create a release tag, merge main, install/run a production binary, or change application version files.

Never access the installed release database or sidecars. All tests use mocked, in-memory or disposable databases. Production code may be compiled and packaged but must not run. Preserve the existing pre-Focus recovery tag/bundle and released migrations.

## Plan

1. Review all changes since the recorded 0.15.0 release, with independent reviews of watch controls and sync/preferences compatibility. Verify changelog coverage and release-script version detection.
2. Run `npm run check` for Svelte diagnostics, all frontend tests, offline locked Rust tests and the production frontend build. Confirm version/lock consistency and unchanged migrations.
3. Build NSIS and MSI validation packages with the existing validation config: separate **TVC Release Validation** product name, updater artifacts disabled, no signing, locked/offline Cargo. Do not install or launch them.
4. Record actual results and practical limits here, commit the final preparation on the Focus branch, and leave release publication to the owner.

## Release contents

- Accept and undo acceptable title sync failures while retaining sync history.
- Choose Shows or Movies as the default tier list without clearing rankings.
- Whole-title watched/unwatched controls in both libraries and detail windows, including tier-only titles.
- Consistent poster metadata, tier and action rows, including long tier names.

## Validation

- Independent review found no blockers in accepted sync failures or the saved default tier list. Older stored preferences, reports and version-4 portable backups remain compatible. Targeted frontend coverage passed **26/26**.
- Watch-action review found and fixed one new-summary cache gap: after episode sync adds a new unwatched episode, tracked/archived/tier summaries now reload alongside the calendar. A previously fully watched show now offers the correct next action in the library. A regression reproduced the wrong old count before the fix and passed after it; the watch suite passed **12/12**.
- Final `npm run check`: **0 Svelte errors, 0 warnings; 101 frontend tests passed; 130 Rust tests passed; 1 live Wikipedia test ignored; production frontend build passed**. Tests use isolated memory/disposable data. Log: `release-0-15-1-check.log` (ignored).
- Release script PowerShell syntax parsed successfully without executing it. Its changelog detection finds exactly one Unreleased entry: **0.15.1**.
- Package, package-lock root/metadata, Cargo manifest/lock and Tauri config versions all remain **0.15.0**, matching the released application. The owner's release script will set **0.15.1**, synchronize the locks and stamp the changelog date. No released migration or version file changed.
- Human-readable changelog covers all four additions, the eye toggle in both directions, the refresh-status fix, and the saved-episode limitation for shows.

### Windows packaging

Optimized native release compilation and both NSIS/MSI bundles **passed**. Command:

```powershell
node node_modules/@tauri-apps/cli/tauri.js build --config plans/release-validation/tauri.validation.json --bundles nsis msi --ci --no-sign -- --locked --offline
```

The existing ignored validation config changes only the product name to **TVC Release Validation** and disables updater artifacts. These packages retain the current manifest version and are test artifacts, not installers to distribute. No production binary or installer is executed.

Both candidate packages were moved from the build output into the ignored `plans/release-validation/0.15.1-candidate/` folder after verifying all resolved paths remain inside the workspace. Existing release installer files were preserved. Build log: `release-0-15-1-bundles.log` (ignored).

| Local validation package | Bytes | SHA-256 |
| --- | ---: | --- |
| `TVC Release Validation_0.15.0_x64-setup.exe` | 4,312,380 | `87D062B70EB677B602C1EB610EE9331E9F36536C2EDB7B927AA0C728F203A8ED` |
| `TVC Release Validation_0.15.0_x64_en-US.msi` | 5,951,488 | `132CE0FCAF11301F153EC19C7899C0A2FB5E1E7BD248EF32F32EC32A373329F5` |

## Practical limits and owner handoff

- Previous per-feature compiled-UI checks covered 1080p/4K, library/card variants, both modal buttons, preference persistence, sync acceptance/Undo and failure feedback with memory IPC fixtures. The later eye-toggle correction was exercised in the compiled poster/list UI. This preparation's additional status-refresh correction is covered by an isolated regression against the actual stores.
- Live TMDB/Arr/Plex services, signing credentials, Windows installation/update delivery and installed release data are outside these local checks. The normal release script performs the final version bump, signing and publication when the owner runs it.
- Whole-show watch actions affect every saved episode, including specials and future episodes. They do not fetch missing episodes automatically. A show with no saved episodes keeps its existing error/recovery guidance rather than reporting success.
- Keep the candidate on the Focus branch and preserve main and the pre-Focus recovery tag/bundle. Do not create a release/tag/update manifest, merge or publish as part of preparation.
- Preparation is complete. The owner can run `.\scripts\release.ps1` when ready; it detects **0.15.1** from the changelog. The local validation packages above are not the signed 0.15.1 distribution files.
