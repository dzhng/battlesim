# C02: per map fog cell

**Depends on:** C01. **Kind:** slice.

## Question
Is fog resolution owned by the map everywhere?

## Contract it unlocks
`MapDefinition.fog_cell_m` becomes required, and `sensors.fog_cell_m` is **deleted** (`scenario.rs:311`, read at `battle.rs:464`). The foliage grid follows the map's cell (`world/mod.rs:108`, `forest.rs:70`). The village and labs carry 8 m.

## API seam
`contract::map`, `sim::visibility`, `sim::world::forest`, exports.

## What the human can run or see
A street-visibility probe on S1's generated urban plot at G0's cell.

## Verification
- Village digests unchanged.
- Native and wasm replay.
- Foliage agreement.
- Record the intentional fixture/config identity change separately from outcomes.

## Delegated to the implementer
Internal plumbing only. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village at exactly 8 m for this schema-only cutover. Full-extent delivery at every resolution must pass G0/C07 before playable release; 8 m does not exempt a map. C56 names later geometry/identity changes.

## Feedback that would change this slice
Unacceptable street visibility or snapshot delivery at the chosen cell size reopens the measured fog choice at G0.
