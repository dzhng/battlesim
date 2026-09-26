# 29 — Sim performance: explain and fix the slice-07 regression

**Status:** done (the cause was an inlining change in the friendly-fire check, fixed with digests unchanged; `decisions.md`, slice 29). **Depends on:** 07, 08. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

After slice 07 the endurance battle runs about 40% more instructions per tick (p50 13.0 → 18.8 ms), with the ground layer's own work at about 1%. Slice 08 confirmed it on an identical battle with craters off: the same call counts, and byte-identical machine code for the sweep, sensing and triangle lookup (`decisions.md`, slices 07 and 08). Per-soldier movement lands on top of this, so find the cause first (landmine L10 in [`movement-unknowns-map.html`](../movement-unknowns-map.html)).

## API seam

- Bisect the slice-07 commit on the identical-battle setup (craters off), with instruction counts (`/usr/bin/time -l` or Instruments), not wall time.
- Suspects to rule in or out: digest refresh per tile; `MovementContext` or `DamageContext` growth changing inlining; `GroundLayer` borrow or allocation patterns; cache effects of the new layer's memory; a hash map on a hot path.
- Fix it, or explain it with evidence and record why it stays.

## Verification

- The endurance report on the identical battle is back within 5% of pre-07 instructions, or the remaining gap is explained in `decisions.md` with a profile.
- Digests are unchanged by the fix.


## Decision budget

- **Delegated:** the method.
- **Not delegated:** any change to rules or digests.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
