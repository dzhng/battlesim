# 02 — Spike: sight-fog technique

**Status:** planned. **Depends on:** 00. **Lane:** spike (throwaway).

## Contract

Which per-pixel fog technique gives ARMAPHRACT-sharp building sight shadows at 100 units a side, at acceptable cost, and in agreement with the simulation's knowledge?

## API seam

A throwaway fog prototype over the current renderer, using a hand-written sight shape. It tries two candidates on the same inputs:
1. **Sight lights:**
   - per own unit, an angular horizon map: the maximum blocking slope per azimuth over terrain and known props;
   - updated only when the unit's pose or bearing changes;
   - culled per screen tile;
   - each fragment tests the few units covering its tile.
2. **Viewshed texture:** a per-side 1 m texture marched in compute over a 1 m occlusion height raster, then sampled per fragment.

## What you can run or see

`spikes/02.md`, plus frames of the village street and `/lab/endurance` at 100 a side, for both candidates.

## Verification

For each candidate, measure:
- GPU ms per update and per frame, at 1080p;
- edge sharpness at Broken Arrow ground framing against `armaphract/x-urban-fog-t9s.jpg` (centre sight wedge);
- disagreement with the simulation's `ground_visibility` at 8 m cell centres, excluding a one-cell boundary band;
- behaviour with foliage attenuation and canopy.

**Acceptance bar** (provisional, recorded in the verdict):
- building edges show at most one pixel of stair-stepping at ground framing;
- about 2 ms or less per frame amortized at 100 a side;
- disagreement at most 5% of cells outside the band.

Choose the first candidate that passes. If neither passes, the fallback order is: frustum-limited sight lights, then a 2 m viewshed, then ask the user.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Everything inside the spike.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

None; slice 14 consumes the verdict.
