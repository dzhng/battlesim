# 22b — Wrecks, the bridge, and soldier variety

**Status:** planned. **Depends on:** 21b, 32 (for the infantry item). **Lane:** asset.

## Contract

Slice 21b cleared the "toy-like" gate for the soldiers, tank, truck, wall and crate, and escalated three named items. This slice owns them, under the same pass/fail critique gate (user 2026-09-25):
1. **Wrecks need destruction modelling, not texture.** Deformed and torn geometry, and a tank wreck whose hull matches the live tank's wedge hull. The truck wreck too. Ten rounds of surface work didn't move the critic.
2. **The bridge fails on its surroundings.** Model piers below the 0.8 m prop box, and fix how the bridge meets the terrain's opaque water box (the water surface is slice 16's material; coordinate, don't fork).
3. **Village infantry read as lockstep clones.** The movement side is slices 31–32. The asset side is per-soldier **appearance variety**: a few head, kit and pose-phase variants per kind, chosen deterministically per soldier id, so a squad never reads as copies.

## API seam

- Blender scripts in `packages/scene-assets/blender/`, catalog entries, `project-owned` manifest entries, and the slice-21b texture recipes.
- Variety: `AppearanceCatalog.resolve(kind, side, soldierId)` picks a variant. Variants share one skeleton and clip set. Picking stays on the simulation's boxes.

## Verification

- An unprimed critique per item, at close and battle views, must not call it toy-like; the village ground camera must not read as clones.
- compare-screenshots against the references.
- Workbench sheets.
- Record frame cost and texture MiB (within 21b's 32 MiB budget, or record a raised budget with its reason).

## Decision budget

- **Delegated:** the art, and the variant count (at least 3 per infantry kind).
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Spike 03 parity rows. Every existing scene and test. `bun run check` and `bun run verify` at closeout.
