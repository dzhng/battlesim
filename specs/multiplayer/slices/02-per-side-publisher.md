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

- **`crates/sim/src/publication.rs`, `Publishers`:** the one owner of a
  battle's publications to its sides. It holds a `Publisher` per side and
  has `publish(battle, side)`, `record(side)` and `resync(side)`. A resync
  opens an epoch later than any side has reached, so one reader may leave a
  side's stream for another's. `Publisher` itself is unchanged.
- **`crates/game-wasm/src/lib.rs`, `BattleHandle`:** holds a `Publishers`.
  `publish(side)` stays; `publication_ptr(side)` and
  `resync_observation(side)` take the side.
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

## Found at integration

Codex's commit `67844ee46` built the binding and protocol above with a
publisher per side inside the WebAssembly handle. Two defects came with it,
both hidden by a changed test.

1. **Switching the watched side a second time left the page blind.** Each
   publisher counted its own epochs, and a page's reader treats an epoch no
   later than the last one it saw as stale. Blue, Red, Blue: Blue's reopened
   stream arrived with an epoch the reader had already passed. An epoch
   expectation had been repinned to match. Epochs now rise across a battle's
   sides, and tests on both builds switch back and forth.
2. **The native parity assertion was deleted.** The paired records in
   `fixtures/parity/publication/` are one reader's stream that moves between
   sides. With a publisher per side they could no longer be produced by the
   WebAssembly handle, and the native test still used a single publisher.

The repair gave per-side publication its one owner in `crates/sim`, used by
the handle and by the native test alike, and re-recorded the five paired
records from the native test. That is a named change to a parity fixture.
What changed in them: a row that changes side now says `resync`, and the
published bytes differ from the first side change onward. Every battle
digest and every delivered fog field in them is unchanged.

**For slice 06.** The authority takes the commanding side from the first
published side (`commandSide = sides[0]`), and one player from `init`. A page
publishes one side, so that holds. A server host publishes both and serves
many seats: there the `command` request must name the seat it is for, set by
the host from the port, never from the wire.
