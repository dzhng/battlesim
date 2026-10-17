# 01 · A helicopter shot down over a bridge rests on the deck

**Owns:** L11. **Depends on:** nothing. **Commit:** on its own, before any transport commit.

## Contract
A downed helicopter's fall ends on the walkable surface beneath it, bridge decks included, and its wreck rests there. Slice 09's bail-out places passengers around the wreck, so the wreck must not lie in the riverbed under a bridge.

## Seam
- `Crash::fall` (`crates/sim/src/crash.rs:86-133`). Its ground today is `height_at` (`:128`), which ignores bridge decks (`world/mod.rs:530-535`). Use `surface_at`, filtered to decks at or below the airframe's previous height, so a helicopter passing *under* a deck is not caught by it.
- The wreck in `Battle::land` (`battle.rs:2689`): it is added with `base_z: None`, and `crash::resting_place` (`crash.rs:172`) searches the ground. The wreck must sit on the deck when the impact was on the deck.

## What a human can see
Nothing new on screen; the proof is a test. Optionally, an existing `air-*` lab with a bridge.

## Verification
- One test, `a_helicopter_shot_down_over_a_bridge_rests_on_its_deck`, in `crates/sim/tests/air_crash.rs`. It builds the bridge inline, the way `crates/sim/tests/rivers.rs` does. It asserts the impact height and the wreck's base both equal the deck's `surface_at`.
- Rung 1: `cargo test -p sim --test sim air_crash::`.

## Skip rule (user, 2026-10-16)
If seating the wreck on a deck needs new props-on-bridges support, skip this slice entirely and record it in [choices.md](../choices.md). Never fix only the impact height: the burst would then happen on the deck while the wreck lies in the riverbed. Slice 09 then treats a crash over water like any other place with no room to land.

## Delegated
How the test builds its bridge; the tolerance of the height comparison.

## Must stay green
The other `air_crash` tests, bridge and river tests, and the menu reel (it has no aircraft). Digests of battles without a crash over a bridge are unchanged.
