# 17 — Scars on the ground

**Status:** planned. **Depends on:** 08, 16. **Lane:** renderer.

## Contract

Battle scars from the authoritative ground layer, drawn only from the side's learned cells:
- crater relief cues and a cover rim;
- scorch;
- track wear;
- trampling.

Collision geometry never changes.

## API seam

- `GroundSurface` consumes `GroundView` patches.
- A GPU scar texture gets dirty-cell uploads.
- The terrain material samples it.
- Trampling also feeds grass bend (slice 18).

## What you can run or see

`/lab/ground` re-shot, and `/replay/village` after a bombardment.

## Verification

- Tests: exact dirty-cell uploads; persistence; side resync; hidden cells never drawn.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** crater form and scar readability.
- **Reference crop:**

- `defilade/x-shader-craters-f4.jpg`, the dark road strip and crater field: judge form, not the winter colour.
- `defilade/x-refined-explosions-f8.jpg`, the upper-left crater.

- **Out of scope:** Snow colours, smoke and fire, and grass.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Material contrast, and crater relief shading.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If scars read too strong or too weak, adjust presentation contrast only. Footprints follow the simulation.
