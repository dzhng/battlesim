# 09 — Camera and keys

**Status:** done (2026-09-25). **Depends on:** 00. **Lane:** controls.

## Contract

A Total War camera (I5, I6):
- **Keys:** WASD and arrows pan, Q/E rotate, the wheel zooms, middle-drag orbits.
- **Zoom range:** from WARNO's strategic height down to Broken Arrow's ground level.
- **Pitch:** linked to zoom in the wheel gesture only.
- **Default framing:** Defilade's.

Commands move off those letters **first**: Backspace stops, Ctrl+right-click or R attack-moves, F toggles fire policy, G attacks ground, T deploys or packs, Escape disarms.

## API seam

- `web/src/battle/input` gains `CommandBindings` as the one command key table. Command keys move in an earlier commit than the camera keys.
- `renderer-core::CameraController` takes `CameraIntent` (held keys, wheel, drag) plus elapsed time and fixture `presentation.camera {zoom_min, zoom_max, pitch_curve[], pan_speed, rotate_speed}`, and produces `Camera3DParams`.
- `setCamera` stays raw and unrestricted, so scene-authored framings, including `geometry.mjs:108` below the minimum pitch, keep working.
- The stronger typing guard: inputs, textarea, select, contentEditable, modifiers, repeat. Keys are released on blur.
- `projectToCss` and `rayAt` must equal `camera3d` (landmine 17).

## What you can run or see

`/battle/village` with the new controls: a near, default and far tour.

## Verification

- Unit tests:
  - the pitch curve is monotonic and within its limits;
  - held keys combine;
  - release on blur;
  - the typing guard;
  - the binding table.
- Scenes: `readouts`, `movement`, `village-replay` and `foundation` migrate to the new keys, each driving real commands.
- Picking and `projectToCss` agreement tests stay green.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** framing (camera height, pitch, what is in frame); not colour or material.
- **Reference crop:**

- Default: `defilade/steam-4.jpg`, the whole frame.
- Strategic: `warno/gameplay-tutorial-29.jpg`, the whole frame minus the HUD.
- Ground: `brokenarrow/gameplay-trailer-19.jpg`, the lower half.

- **Out of scope:** Colour, flat shading, box units and the terrain material.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** The pitch curve and speeds, within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the default framing feels wrong at play, re-tune `presentation.camera` only.
