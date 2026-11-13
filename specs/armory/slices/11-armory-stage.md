# 11 · The Armory stage

Map rows: Q1′, Q6, D1, L5, L8, L9, L10, L11. Depends on 07, 08, 09 and 10.

## Contract

`/armory` shows the selected unit on its spot in the yard, with idle life and
idle sound. Failures stay on the stage, and detection is never aborted.

**The camera is fixed at the spot's pose in this slice.** Slice 12 adds orbit
and zoom.

## Seam

`apps/battle-lab/src/armory/ArmoryStage.tsx` composes:
- `ShowcaseStage` (04);
- `ArmoryAppearances` (09);
- the yard (10);
- `Showground.squad` (02);
- `appAudio.showLoops(stage.show(...))` (07).

**Art per selection (L6 against slice 04's API).**
- Slice 04's `ShowcaseStage` takes `appearances` once, at `create`. That would
  force the Armory to install every unit's art up front, which L6 rules out.
- Each `show` already rebuilds the stage's scene, so `show` takes the
  selection's art too: `show(unit, seed, art: ArmoryArt)`, with
  `appearances` and `runtime` from `ArmoryAppearances.show`. `create` no
  longer takes `appearances`.
- Update the showcase lab to match. There is one owner, and no second
  constructor path.
- Pass `art.runtime` to fielding, never `gameRuntime`.
- **Show is async.** A superseded `show` resolves `[]`, and an `AbortError`
  from `ArmoryAppearances.show` is a normal switch, not a stage error.

**Mounting and loading (L8).**
- It mounts only when `useMenuShell().detected` is true.
- It reports `useLabLoading("renderer", ready)` inside the shell's
  `LoadingTasks`.
- If first-launch detection hasn't finished, `/armory` holds the backdrop and
  the cover until it does.

**Failures (L5).**
- It reports `ready = true` so the cover lifts, then draws the error on the
  stage inside a local `ArmoryStageBoundary`.
- It never calls `appResources.refuse`, and never reaches
  `AppResourceBoundary`.

**Other cards.**
- **Planned cards (L10):** never ask the library for anything; the sheet carries
  them.
- **Faction (L11):** comes from the open tab through `Fielding`.

**The backdrop.**
- It stays mounted in the shell, so it never restarts (slice 01, test 1).
- Measure frame time with both scenes live:
  - if it holds the frame budget, change nothing;
  - otherwise the shell gains `holdBackdrop`: `ScenePlayer` stops presenting,
    but keeps its clock and worker.
- Record the measurement in `decisions.md`.

**Stage sound.** Leaving the Armory calls `showLoops([])`. The menu bed keeps
playing.

## The human runs or sees

`/armory` with real 3D:
- a tank idling;
- a helicopter, its rotors turning on the ground;
- a jet parked on its gear;
- a squad in its scatter;
- engine sound under the menu bed.

Pull the network or pick a broken bundle, and the error stays on the stage.

## Verification

Write these first, in `web/tests/armoryStage.test.tsx`:
- `a stage failure stays on the Armory`:
  - the menu and panels stay usable;
  - `appResources.error()` is null;
  - the error text is on the stage.
- `opening the Armory mid-detection waits and saves the measured tier`.
- `a planned card never asks for appearances`.
- `leaving the Armory stops its loops and keeps the bed`.

`web/scenes/armory.mjs`:
- One unit per group, picked by group.
- The show voices are greater than 0 on `/armory`, and equal 0 after Back.
- Menu → Armory → Back: the backdrop doesn't restart, and the cover doesn't
  show again.
- **Measure once and record** (loose tripwires, not targets):
  - switch time (S7);
  - the load line (S5), logged to `throwaway/armory/11/`.

**Visual gate.**
- **The one variable:** the unit is seated in its spot. Wheels, tracks, skids
  and gear are on the ground, with no float and no sink.
- **Crop:** the unit's bounds plus 20 %, at the spot camera, with both panels
  masked.
- `compare-screenshots` against slice 10's composition shots and the helicopter
  reference.
- `screenshot-critique` last.
- Then the non-blocking `preview-shots` checkpoint.
- Re-record the `armory` baseline in `ui.mjs` once, with the stage live.

## Delegated to the implementer

- The error copy.
- Whether `holdBackdrop` is needed. The measurement decides.

## Must stay green

- Slice 01's six tests.
- `uiCoverage`.
- `_appJourney.mjs`: one device, no leftover buffers, one backdrop worker. Its
  `releasedBefore` check extends to an Armory round trip.
- `graphicsDetection.test.ts`.
- `appAudio.test.ts`.

## Feedback that would change it

- **"Too quiet or too loud":** the audio fixture, as a named decision.
- **"Wrong spot":** `yard.json`.

## Reslice if too big

Split it into three, each with its own tests:
1. the silent stage (library, yard, unit);
2. loading, detection and errors;
3. sound.
