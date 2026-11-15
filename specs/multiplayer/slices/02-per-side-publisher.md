# 02 Per-side publisher

**Stage:** Foundation. **Needs:** nothing. **Carries:** decision 57, landmine 4.

## Contract

The battle keeps one publication baseline per side. Publishing Blue and Red
in the same tick gives each side a delta of its own, and resyncing one side
leaves the other untouched.

Today the handle holds a single publisher (`crates/game-wasm/src/lib.rs:650`)
and a change of side empties its baseline, so serving two sides would send a
full snapshot every tick.

## Seam

- **`crates/game-wasm/src/lib.rs`, `BattleHandle`:** one
  `sim::publication::Publisher` per side, made on first use.
  `publish(side)` stays; `publication_ptr(side)` and
  `resync_observation(side)` take the side. `Publisher` itself is unchanged.
- **`web/src/battle/sim/protocol.ts`:** `init.side` becomes
  `init.sides: SideName[]`; the `publication` reply gains `side`; the lab's
  `{ type: "side" }` request becomes `{ type: "sides"; sides: SideName[] }`. A
  side new to the list opens with a full snapshot.
- **`web/src/battle/sim/authority.ts`:** publishes every listed side each
  tick. It remains the only owner of ticking.

This is a hard cutover of the binding and the protocol. Every caller moves in
this slice: the page passes one side.

## Run or see

The authority lab route (`apps/battle-lab/src/routes/authority.tsx`) still
switches the side it watches.

## Verification

Extend `web/tests/authority.test.ts`:

- With two sides listed, each side's records after its first are deltas.
- Resyncing Blue makes Blue's next record a snapshot and leaves Red's a delta.
- A side added mid-battle opens with a snapshot.

## Must stay green

Every existing digest and replay: the publisher sits outside the digest. The
publication tests in `crates/sim`.

## Delegated

How publication buffers are pooled per side; internal names.

## Feedback that would change this

None expected from the user: nothing here is visible.
