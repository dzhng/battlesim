# 10 — Main menu and scripted benchmark

**Status:** planned. **Depends on:** 09. **Lane:** controls.

## Contract

A scripted performance benchmark: a real simulation and scripted camera movement, runnable from a new main menu (the user's request). It is the one measuring stick for `frame-cost.md` and slice 27's 30 FPS floor. It is modelled on `~/dev/game/web/src/battle/benchmark/`, ported by technique:
- a versioned scenario;
- camera phases;
- recording;
- a results screen with a frame chart.

## API seam

- **Main menu** at `/`: Play village, Watch replay, Benchmark, Labs. The lab index moves to `/labs`.
- **`BenchmarkScenario`**:
  - `id` and `version`;
  - village fixture and seed;
  - both sides scripted: blue runs the supported comparison script, red is the defender;
  - `start_tick`, warm-started to heavy contact by stepping the real simulation;
  - `duration_ms` (5 minutes, plus a 60 s short run for per-slice records);
  - `camera_script` version.
- **`BenchmarkCamera`**: keyframes through the phases `strategic`, `pan`, `zoom`, `ground`, `combined` and `return`, using `CameraController`. Angles stay unwrapped. Changing anchors bumps the version.
- **`BenchmarkRecording`**:
  - per frame: interval, CPU time and GPU passes where available;
  - per tick: step time;
  - publication bytes;
  - buffer and texture bytes;
  - memory.

  Results are a JSON report plus a results screen with a frame chart and percentiles per phase.
- Slice 00's `web/scenes/_frameCost.mjs` is deleted; the benchmark recorder is the one measurement owner.
- A scene `benchmark` runs the short run on the production build, and writes the `frame-cost.md` row and the evidence.

## What you can run or see

The main menu's Benchmark entry, the results screen, and `bun run --cwd web scene -- benchmark`.

## Verification

- Unit tests: camera keyframe interpolation and phases; scenario version pinning; recording percentiles.
- The scene asserts:
  - the run completes;
  - the report has every phase;
  - the simulation actually advanced;
  - the camera matched its keyframes.
- Two runs on a quiet machine agree within a stated tolerance, recorded.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** results-screen readability; not the battle's look.
- **Reference crop:**

None. Judge by screenshot-critique only; there is no reference crop for a results screen.

- **Out of scope:** The battle's look.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** The exact keyframes (anchored on scouted contact positions) and the results-screen layout.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the user wants a different battle, for example the endurance 100-a-side battle as a second preset, add a scenario entry. The code doesn't change.
