# Transport: choices ledger

The calls the implementing agent made where the plan was silent, one entry per call. Each entry says:
- the slice it came from;
- the choice, and the alternative it beat;
- why;
- what it reaches;
- how to reverse it.

Decisions the user made live in [map.md](map.md); change them there.


## Slice 05
- **Recon prices stay the same after dropping a scout.**
  - Alternative: cut the price in proportion to the squad's size.
  - Why: the slice's default; nothing asked for retuning prices.
  - Reach: recon cards cost the same for three soldiers.
  - Reverse: edit the cost on the four rows in `fixtures/units/roster/*.json`.

## Slice 02
- **`Unit::garrisoned()` stays the name of the building-only query**, rather than being renamed.
  - Why: it still means "inside a building".
  - Reverse: rename it if carriers make the name ambiguous.
- **Building seat data lives in `Place::Building(Garrison)`, while `Hold` owns the host and phase.**
  - Alternative: flatten the seat fields into `Hold`.
  - Why: keeps building specifics in `garrison`.
- **`Hold` has a temporary `Deref`/`DerefMut` to `Garrison`** so building code reads unchanged. It panics on a non-building host.
  - This is short-lived scaffolding: slice 06 removes it when `Host::Carrier` arrives, and building code then reaches its `Place` explicitly.
- **The digest proof was replay-based.** The garrison and buildings modules ran before and after (54 and 32 tests), the 1800 per-tick replay digests matched, and `cargo test -p sim` passed. A probe listing every battle's final digest wasn't practical through the private test helpers.

## Slice 01
- **A falling helicopter stops on a bridge deck only if the deck was below the airframe on the previous tick.** One flying under the deck falls to the riverbed.
  - Alternative: always stop on the deck.
  - Why: an airframe below a deck must not be caught by it.
- **Bridge slabs don't count as obstacles while the airframe falls,** so it can land on them. They also don't block the wreck's resting place on a deck. Other immovable props stay obstacles.
  - Reverse: `crash.rs`, `Crash::fall` and `resting_place`.

## Slice 03
- **The own row carries the host as `hostKind` plus the host id and geometry columns.** One layout serves building and carrier hosts, so slice 06 adds no new columns.
- **The refusal reason table is fixed and published,** so slices 06 and 07 add reasons without another layout change.
- **Hold progress stays a 0–1 fraction,** keeping the existing UI behaviour.
- **The captures were re-recorded from one new browser run** (`922f3416c`).
  - The market-town window has had no script since it was first made, so a scratch example (not committed) rebuilt it from a native replay. It was checked byte for byte against the run's layout and samples.
  - That window's content also moved, because commit `ee1651c09` had retimed the menu battle after the previous capture. The new window matches the current game.
  - The paris-corner poses moved because this run drew fewer frames (2461 against 4473); they are sampled at frame times.

## Slice 04
- **Bail-out rows are keyed by an enum of physical host kinds** (`BailOutHost`: building, ground, air) in `rules.bail_out`. The caller picks the row through `bail_out::row`; `hold::Host` only supplies the event's random key.
- **A survivor's spot must be reachable from where he stood,** with the destroyed host's own parts ignored. This keeps the old rule: no appearing behind a neighbouring wall.
  - The first pass dropped the rule. The orchestrator's review restored it, and a courtyard test now pins it.
- **Soldier spacing for forced exits stays 1 m** (`arrangement::ESCAPE_SPACING_M`).
- **Partial deaths go through the existing corpse and unit accounting;** the observation is unchanged.
