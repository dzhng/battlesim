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

## Found at integration (2026-11-14)

Codex's commit `67844ee46` builds the contract above, with two defects. Both
are fixed or restored in the uncommitted work, and the second is not finished.

1. **Switching the watched side a second time left the page blind.** Each
   publisher counted its own epochs, and a page's reader treats an epoch no
   later than the last one it saw as stale. Blue, Red, Blue: Blue's reopened
   stream arrived with an epoch the reader had already passed, and every
   record after it was dropped. The fix makes epochs rise across a battle's
   sides: `Publisher::resync_after(epoch)`, called by the handle with the
   latest epoch any of its publishers has reached. A new test switches four
   times and sees each side.
2. **The native parity assertion was deleted.** `web/tests/fogDelivery.test.ts`
   checks that the WebAssembly build publishes the same bytes as the native
   build for a recorded stream. That stream is one consumer on one publisher
   that moves from Blue to Red, so it fails at the first side change: a
   publisher per side has its own fog revision counter. The assertion is
   restored and is red. The honest repair is to give per-side publication one
   owner in `crates/sim` (a small type holding a publisher per side, with the
   rising epochs), have the handle and the native test
   (`crates/sim/tests/publication.rs`) both use it, and regenerate the
   fixtures in `fixtures/parity/publication/` from the native test. That is a
   named change to a parity fixture: the stream it recorded can no longer be
   produced by the product. It needs the native publication tests built and
   run, which the disk could not hold.

Also not run here: `web/tests/battleRules.test.ts` and
`web/tests/buildingKnowledge.test.ts`, which this slice edits. They read
runtime assets this checkout holds only as large-file pointers.

**For slice 06.** The authority takes the commanding side from the first
published side (`commandSide = sides[0]`), and one player from `init`. A page
publishes one side, so that holds. A server host publishes both and serves
many seats: there the `command` request must name the seat it is for, set by
the host from the port, never from the wire.
