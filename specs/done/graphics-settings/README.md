# Graphics settings

## Purpose

The game gives players one graphics choice surface in the Main Menu and the
paused in-game menu. The surface has exactly four choices: Auto-detect, Low,
Medium, and High. A concrete choice takes effect immediately by replacing the
viewport renderer generation; simulation state, map identity, fog authority,
and replay digests remain independent of presentation quality.

Auto-detect runs against the live Main Menu backdrop reel. It tests Low,
Medium, and High in that order, stopping at the first that misses, and
persists the highest tier whose **average FPS is at least 45**. The 1% low is
retained in the diagnostic result so jagged pacing is visible in reports, but
it is not a pass/fail gate. **Detection never turns a player away.** A machine
that reaches the target on no tier plays on Low. A measurement that breaks
(no WebGPU, no reel mounted, a timeout, unusable timing) falls back to the
best tier proven so far, else Low, for that page only, so the next launch
measures again.

## Why these profiles

The spike showed that the useful cost controls are model/building/tree LOD,
shadow map size/reach, and vegetation dressing. Low and Medium reduce those
costs while keeping fields visually continuous. Bloom, cast-light effects, the
authored sun/sky, and the mood grade stay enabled at every tier. Low also
draws at a reduced internal render scale to bound fill cost.

The detector measures renderer-ready frames from the existing menu reel instead
of inventing a synthetic scene. Candidate profiles are transient: a broken or
cancelled run cannot persist a half-tested tier. Persistence is a hard cutover
under `battle.graphics`; malformed or unknown records return to Auto-detect.

## Code pointers and invariants

- `apps/battle-lab/src/graphicsSettings.ts` owns the mode schema, profile table,
  persistence, and average-FPS resolver.
- `apps/battle-lab/src/graphicsDetection.ts` owns the shared runner boundary,
  cancellation, timeout, frame statistics, and selection lifecycle.
- `apps/battle-lab/src/MenuBackdrop.tsx` registers the canonical reel runner;
  `GraphicsDetectionHost.tsx` starts first-launch detection; the Main Menu owns
  its progress card and is the only surface that offers a rerun, because only
  it has the reel mounted.
- `apps/battle-lab/src/LabViewport.tsx` owns atomic renderer generations and
  quality application; `GraphicsSettingsControls.tsx` is shared by both menus.
- `web/scenes/_lab.mjs` seeds a settled graphics record for battle/menu scenes,
  so unrelated browser contracts do not accidentally exercise first-launch
  detection. Native fixture helpers are built into Cargo `target/`; `throwaway/`
  contains evidence only.
- `web/tests/graphicsSettings.test.ts`, `graphicsDetection.test.ts`,
  `graphicsSettingsControls.test.tsx`, `pauseMenu.test.tsx`, and
  `navigation.test.tsx` pin the settings and detector contracts.

The player-facing invariants are: four choices only; immediate apply while
paused; mood-critical effects stay enabled; auto-detect chooses the highest
passing tier by average FPS, else Low; no persistence after cancellation or a
broken measurement; and detection is never fatal.

## Visual and performance provenance

The profile values came from the graphics spike and production reel evidence in
`throwaway/perf/2026-10-11/` and `throwaway/perf/graphics-settings-closeout/`.
The accepted UI captures are `throwaway/evidence/settings-main.png`,
`menu-settings-crop.png`, and `settings-pause.png`; they document the shared
Main Menu and Pause modal surfaces. The durable visual standard remains the
existing holo-tactical menu and HUD baselines under `web/scenes/baselines/`.

## Rejected approaches

A synthetic RAF probe was rejected because it did not represent the renderer
work the player sees. Aggressive grass removal was rejected
because it turns fields into isolated bushes and loses the game's mood.
