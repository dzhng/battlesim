# 01 · Lane rule, served once; opening camera on it

**Depends on:** nothing. **Changes digests:** no.

## Contract

`sim::entries` owns lane numbering, slot-to-lane and the opening entry
(see [Contracts](../README.md#contracts)). Every Rust consumer calls it, and
TypeScript reads it through one Wasm function. The opening camera uses it, and
the TypeScript copy of the rule is deleted.

## Seam

In `crates/sim/src/entries.rs` (names are working names):

- `pub struct LaneNumber(u8)`, 1-based, a separate type from `EntryIndex`.
- `lane_number(sites, EntryIndex) -> LaneNumber`: rank by descending x within the entry's
  side, ties by list order.
- `homes(sites, sides: &[Side]) -> Vec<Option<EntryIndex>>`, one per player in
  list order: the only place slot meets lane.
- `opening(sites, players) -> EntryIndex`: the human's home entry; otherwise
  the centre of the human's side; Blue's centre when there is no human.
- `select`, `centre` and the list order do not change.

In `crates/game-wasm/src/skirmish.rs`: `skirmish_lanes(sites_json, players_json)`
returns, per entry in list order, `{ side, lane, holder }` (holder is a player
id or null) and the opening entry. One TypeScript reader sits next to the
prepare protocol types (`web/src/battle/prepare/`), used by the lobby (slice 05)
and the battle overlay (slice 06).

The opening camera: preparation takes `report.start` from the Rust opening
entry. Either `SkirmishAdmission` (`crates/sim/src/map_analysis.rs:151`)
carries the start, or `prepare.ts` calls the export; pick the one that keeps
`prepare.ts` free of lane logic. Delete `web/src/battle/prepare/prepare.ts:188-195`.

## What a human can see

Start a 3v3 battle seated as Blue 1, 2 and 3 in turn: the camera opens on the
top, middle and bottom road as the lobby picture draws them. As Blue 4 or 5, or
with no human, it opens on the centre.

## Verification

- Rust tests in `crates/sim/tests/entries.rs`:
  - on synthetic 1-, 2- and 3-road sites and on admitted maps of every size,
    each side's lanes are a permutation of 1..n and descend in x;
  - Blue lane k and Red lane k sit at the same x on generated maps;
  - slot k ≤ n holds lane k; slots above n and empty slots hold nothing;
  - the opening is the human's home entry, or the centre for slot 4–5 and for
    no human;
  - `select` and `centre` answer as before.
- A `crates/game-wasm` test that the export matches `sim::entries`.
- `web/tests/prepareBattle.test.ts` (real Wasm): `report.start` is the entry the
  export names as the human's. Remove the lane case from
  `web/tests/skirmishPreparation.test.ts:91-172`, which fakes admission and would
  only test itself.
- Digests and replays unchanged (`cargo test -p sim`).

## Docs in this slice

Rewrite [I17](../../done/team-skirmish/choices.md) to say the camera opens on
the human's lane by the lane rule.

## Delegated

Exact names; whether the entry-lane record type lives in `contract::skirmish`
or `sim::entries`; how the start reaches `report.start` (within the rule above).

## Must stay green

All digests and replays; the menu reel; jets entering by the centre; every
scene that reads `report.start` (`web/scenes/_appJourney.mjs`, `allies.mjs`,
`unit-roster.mjs`).

## What would change this slice

If any admitted map produces two entries of one side at the same x, the tie
rule needs a look before going on.
