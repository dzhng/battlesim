# 02 · One hold: garrison becomes the building case

**Owns:** groundwork for D6 and the "one hold" invariant. **Depends on:** nothing. **Outcome change:** none.

## Contract
A unit inside something has one owner, `sim::hold`. This slice is a refactor: only the internal module boundary changes, and every battle's digest stays the same. Slices 06–09 add the carrier host to it instead of writing a second state machine.

## Seam
- New `crates/sim/src/hold.rs`:
  - `Unit::hold: Option<Hold>` replaces `Unit::garrison` (`units.rs:279`).
  - `Hold { host: Host, phase: Phase, place: Place }`. In this slice `Host` has only `Building(PropId)`; slice 06 adds `Carrier(UnitId)`.
  - `Phase::{Entering(u32), Inside, Exiting(u32)}` moves here from `garrison.rs:45`, along with the stationary timer and the cancellation by any other order (the `want()` dispatch, `garrison.rs:460`, inside `advance`, `:474-617`).
  - `Host::timing(&Rules) -> Timing { enter_exit_s, reach_m }`. Buildings read `rules.garrison`. This is how a 1 s carrier timer later becomes data, not a second timer.
- `garrison.rs` keeps only building specifics: seat plans (`seats_for_state`, `seat_plan`), validation, window slots, firing, shelter, `exit_spots`, `collapse` (until slice 04).
- `Unit::garrisoned()` (20 call sites) becomes a query on `hold` that is true only for a building host. Rename it if a clearer name fits; record it.

## What a human can see
The garrison lab (`/labs/garrison`) behaves exactly as before.

## Verification
- **Digest probe.** No digest golden covers garrison battles, so first write a throwaway probe in `throwaway/transport/` (the ignored scratch folder). It records the final `Battle::digest` of every battle in the `garrison` and `buildings` test modules. Run it before and after; every digest must match. `garrison::digest` (`garrison.rs:901-923`) must fold the same words in the same order for building hosts.
- Existing tests: `garrison_orders_replay_every_tick_exactly`, `menu_reel::plays_as_recorded`, and the parity record `exact-numbers` must all pass unchanged.
- Rung 2: `cargo test -p sim`.

## Delegated
Internal names (`hold`, `Host`, `Place`, the renamed `garrisoned()`), and how the code splits between `hold.rs` and `garrison.rs`, as long as the timer and phase live only in `hold`.

## Must stay green
Every digest, every garrison and building test, every replay test, the menu reel.

## Feedback that would change it
None expected; it is invisible.
