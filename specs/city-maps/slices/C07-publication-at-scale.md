# C07: publication at scale

**Depends on:** C05 (conditional). **Kind:** slice.

## Question
Do publication bytes fit at city scale and at G0's fog cell?

## Contract it unlocks
An observation layout change:
- the fog field is published only on ticks a side's sweep ran, or as a change mask (the sweep runs every 6th tick, `battle.rs:51`);
- known props are published as deltas (added, moved, removed) with a full snapshot on subscribe.

The decoder cuts over in the same commit. **Required before any fog cell finer than 8 m ships on a playable map** (S-fogpub).

## API seam
`sim::publication`, `battle.rs:1850-1890`, `web/src/battle/sim/observation.ts`.

## What the human can run or see
`city_report` publication split before and after.

## Verification
- Replay parity.
- Decode round-trip test.
- Reconstructed state equals the full-state oracle, including reset, replacement, side switch and missed-generation recovery.
- Fog scene ≥95% agreement.
- City p95 ≤19.8 KB/tick.

## Delegated to the implementer
Mask or interval; delta encoding. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Digests; no hidden-destruction leaks.
