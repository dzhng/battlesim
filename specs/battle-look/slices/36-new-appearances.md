# 36 — Models: the jeep, sandbags, fence, trench and anti-tank wall

**Status:** done. **Depends on:** 34, 21b. **Lane:** asset. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Workbench appearances for the new kinds, textured to the 21b standard. **Quality bar (user, 2026-09-26): match the current models; no iterating on the "toy-like" critique.** A critique is used only to catch outright errors, with at most one fix round. A focused model pass comes later. The jeep is articulated: wheels, the turret HMG, and a jeep wreck. Sandbags, fences, trenches and the anti-tank wall (dragon's teeth) are scenery with footprints; the trench is a ground cut.

## API seam

- Blender scripts in `packages/scene-assets/blender/`, catalog entries, and `SCENERY_KINDS` rows. The reuse manifest gets `project-owned` entries.

## Verification

- The validator is clean and sheets are produced. One unprimed critique round checks for outright errors (floating or intersecting parts, scale, missing pieces), and those are fixed; the look is not iterated.


## Decision budget

- **Delegated:** the art details.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
