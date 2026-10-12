# Graphics settings and auto-detect

## Next Agent Prompt

Slice 01 is complete. Pick up slice 02 next. Read this README and `slices/02-renderer-rebuild.md`, then add the one-owner effective profile flow and atomic paused renderer rebuild. Do not wire the settings UI or benchmark startup flow until slice 03/04. Preserve the slice 01 store contract and update this section before ending your pass.

Status: slice 01 complete; slice 02 next (2026-10-11).

## Goal

Give players one shared Settings surface in the Main Menu and in-game Pause menu. It exposes exactly four choices: Auto-detect, Low, Medium, and High. A concrete choice applies immediately while the battle is paused by rebuilding the renderer. Auto-detect runs on first launch and can be rerun from Settings, selecting the highest tier that reaches the agreed tactical target.

## Fixed contract

- Auto-detect target: average FPS >= 45 and 1% low FPS >= 30 on the canonical graphics workload.
- Auto-detect selects the highest passing tier. If no tier passes, the run is unusable and startup fails; there is no silent fallback.
- Missing WebGPU, adapter/device admission failure, device loss during required startup, or unusable auto-detect timing is a fatal startup error with an actionable message and Reload action.
- Settings persistence is a hard cutover under a new key. Missing, malformed, or unknown data starts Auto-detect. No migration or compatibility shim.
- Bloom, cast lights, authored sun/sky, and the overall game mood stay enabled and unchanged across tiers.
- Grass reductions are conservative: fields must retain continuous coverage and ownership rules; no bald-field or isolated-bush result is acceptable.
- A graphics choice never changes simulation rules, fog authority, map identity, replay contents, or deterministic battle digests.
- Render scale is deferred. It is not part of this plan until CSS size, GPU backing size, camera projection, picking, overlays, post targets, and capture size have one explicit owner and a passing probe.

## Measured starting point

The Windows Chrome / AMD RDNA 3 baseline used the existing reel:

- High/current: market-town 47.6 FPS, GPU mean 17.66 ms; paris-corner 75.6 FPS, GPU mean 13.17 ms.
- Medium candidate: market-town 52.0 FPS, GPU mean 17.31 ms; paris-corner 92.2 FPS, GPU mean 10.92 ms.
- Low candidate: market-town 51.4 FPS, GPU mean 15.61 ms; paris-corner 104.7 FPS, GPU mean 9.57 ms.

The evidence is in `throwaway/perf/2026-10-11/` and the spike record in `throwaway/graphics-settings-spike.md`. These candidates are starting data, not permission to invent different bundles without recording the reason and rerunning the relevant visual/performance gates.

## Slice graph

1. [01-settings-contract.md](slices/01-settings-contract.md) — persistent mode/effective result data, profile table, resolver, and invariants.
2. [02-renderer-rebuild.md](slices/02-renderer-rebuild.md) — one-owner quality inputs and atomic paused renderer rebuild.
3. [03-shared-settings-ui.md](slices/03-shared-settings-ui.md) — shared Main Menu and Pause Settings surfaces with immediate apply state.
4. [04-auto-detect-startup.md](slices/04-auto-detect-startup.md) — bounded canonical benchmark, first-launch/rerun flow, and fatal failure path.
5. [05-closeout.md](slices/05-closeout.md) — visual/performance confirmation, full checks, and closeout evidence.

## Ownership and firewalls

- `packages/battle-audio/src/settings.ts` is the persistence pattern; graphics gets its own owner and key, not a sound-store extension.
- `MainMenu.tsx` and `PauseMenu.tsx` compose one shared graphics control; neither owns a second quality state machine.
- `LabViewport.tsx` owns renderer generation, canvas/device wiring, and rebuild transaction. `BattleView` and `MenuBackdrop` pass the effective profile into that owner.
- Renderer profile adaptation belongs at existing model/building/scenery/shadow/grass owners. Do not mutate authored fixture JSON or simulation data at runtime.
- Auto-detect reuses the existing graphics reel/recording contracts. Do not create a second benchmark workload.
- `AppResourceBoundary`/loading error handling must distinguish a required startup resource failure from a recoverable developer/menu resource; the player path cannot silently continue with a blank backdrop.

## Review map

Every visual slice must run `screenshot-critique` as the last visual check. Where a before/after or approved baseline exists, also run `compare-screenshots`; use `preview-shots` to show the captures. The settings UI is judged as holo-tactical game UI, not as a SaaS dashboard. Performance slices must report adapter, viewport, profile identity, frame summary, GPU timing availability, and validity reason.

## Global checklist

- [x] 01 settings contract and profile tests (`bunx vitest run tests/graphicsSettings.test.ts`; `bunx tsc --noEmit -p tsconfig.json`)
- [ ] 02 atomic renderer rebuild and grass quality seam
- [ ] 03 shared settings UI and pause semantics
- [ ] 04 auto-detect benchmark, persistence, startup fatal states
- [ ] 05 visual/performance closeout and full suite

## Choices ledger

- 45 FPS average / 30 FPS 1% low target: user decision.
- Immediate apply while paused: user decision.
- Four preset choices only: user decision.
- Auto-detect on first launch plus manual rerun: user decision.
- Missing WebGPU and unusable benchmark are fatal: user decision.
- Shared settings layout and equal player/troubleshooting audience: user decisions.
- Hard cutover, no migration: user decision.
- Bloom/cast-light/mood preservation and conservative grass: user constraints.
- Render scale deferred pending a proper size contract: spike finding.
