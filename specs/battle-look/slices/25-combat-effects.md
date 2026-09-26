# 25 — Combat effects

**Status:** planned. **Depends on:** 05, 06, 13. **Lane:** battle.

## Contract

Short combat effects, derived only from published events and counters:
- tracers by round kind, with enemy tracers still clipped to seen ground;
- muzzle flashes on the shooting soldier or mount;
- impact puffs by hit class, placed on the normal;
- ricochet sparks;
- blast fireballs.

## API seam

- `battle-renderer/src/effects/` `EffectFrame` is the one owner. Its inputs are segments, blasts, shot deltas and ricochet normals.
- Effect tunables live in `presentation.effects`.
- CC0 flipbooks, per [research](../research.md), each recorded in the manifest.

## What you can run or see

`/battle/village` firefight, and `/lab/ballistics` with the oblique-AP preset.

## Verification

- Tests:
  - event deduplication across publications;
  - round kinds look distinct;
  - clipped enemy polylines;
  - no effect without a published cause.
- Record frame cost.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** the read of an impact or explosion at the moment of the burst.
- **Reference crop:**

- `warno/gameplay-tutorial-22.jpg`, the fireball on the road.
- `brokenarrow/steam-brokenarrow-1.jpg`, the fireball.

- **Out of scope:** Lingering smoke (slice 26), scars and fog.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** Effect curves and sizes.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If effects overpower readability, reduce `presentation.effects` intensities.
