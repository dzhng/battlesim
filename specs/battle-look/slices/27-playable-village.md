# 27 — Playable village

**Status:** planned. **Depends on:** every earlier slice, the movement lane 29–39, and 40 sound (user 2026-09-26). **Lane:** battle.

## Lanes (orchestrator, 2026-09-27)

- **27a balance:** the village rebalance through the fixture (and blue's scripts where a unit has no role, e.g. the jeep), against `village_report`'s ten seeds. The target is encounter.md's supported capture ≥ 7/10, with the unsupported push failing. Every deferred provisional number named "slice 27's rebalance" in `choices.md` (jeep, forest densities, destroyable-prop hp, cover tiers) is in scope as reversible tuning.
- **27b playable and look:** Play village from the menu, repeated reset and side-switch cleanup, local production assets, every lab re-shot, the whole-battle critique and compare-screenshots against the references below, and the visual items `choices.md` defers to 27: the unit-in-woods cue, the crater lattice, the village water, the scar dense field and low-angle view, and the route ribbon speckle. The final benchmark and critique run on the tree with 27a merged.

## Contract

Friends can play a full village battle on this Mac in the new look, at no less than **30 FPS average at the default camera** (1920×1080), measured by the benchmark. It has both armies, all five kinds, and every mechanic: directional sight, ricochets, craters, fog and the new camera.

## API seam

No new seams. Integration only:
- repeated reset and side-switch cleanup;
- local production asset loading;
- every lab re-shot, since the renderer is shared;
- the final frame-cost table.

## What you can run or see

The main menu → Play village; and the full benchmark.

## Verification

- The ten-seed `village_report`.
- Replay parity.
- `bun run check` and `bun run verify`.
- The full benchmark result recorded, with the 30 FPS floor at the default camera phase.
- A human play session with the user or friends.
- `choices.md` closed.
- Carried in from slice 35's critique: the route ribbon reads speckled (see `orderOverlay.ts` `ribbon`). Find the cause (depth against grass or terrain, lift, or drape step) and fix it, or record why not.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Visual variable

- **Variable:** the composed look.
- **Reference crop:**

- Strategic camera: `warno/gameplay-tutorial-29.jpg`, the whole frame minus the HUD.
- Default camera: `defilade/steam-4.jpg`, the whole frame (framing and scars; summer colours by design).

- **Out of scope:** The HUD, and winter colours.

### Visual acceptance (in this order)
1. Capture the named frames through the slice's scene, at fixed seed, tick, camera, 1920×1080 and DPR 1. Evidence goes in `throwaway/evidence/<fixture-id>/`.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the candidate crop against the reference crop above. Record the telemetry and a less-wrong verdict for **this variable only**. It is not a pixel match.
3. As the **last check before acceptance**, run an **unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)**. Give it only the frames and a neutral task. Include the question: *"Could any dark region be mistaken for sun shadow, or any shadow for fog?"* Act on its findings, or record why not.
4. Open the frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) as a **non-blocking** checkpoint. Allow about 5 minutes and keep working meanwhile. If there is no answer, decide on the evidence, record the decision in the verdict, and close Preview.


## Decision budget

- **Delegated:** None beyond reversible tuning.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

Any "doesn't read" becomes a named reslice, not a patch.
