# 02 · One window plan per building

**Unlocks:** a single owner of a building's windows, so slices 03–04 can seat
and swap across squads. Pure refactor: no outcome changes.

## Seam

- New `crates/sim/src/garrison/windows.rs` (`garrison.rs` is near the file
  budget). `Windows`: one plan per building, keyed by structure owner, a field
  on `Battle`.
  - Created when the first squad is seated, from `seats_for_state`; dropped
    when no living squad holds the building (inside or leaving).
  - Each window keeps its own `changed` tick, as `SeatSlot` does today.
- `Garrison` keeps `seats: Vec<Option<usize>>` (each member's window index)
  and `entry`; `slots` moves out.
- The functions that read windows move to `windows.rs` and take `&Windows`:
  plan total, seating (today's `seat_evenly`), `free_seat`, allocation
  (today's `allocate_slots`, still per squad here), `body_position`,
  `facade_eyes`, `faces`, `changing_window`, and the digest fold.
- Thread `&Windows` to the readers: `damage.rs`, `knowledge.rs`, `sensing.rs`,
  `weapons.rs` (window assessment), `weapons/tick.rs`, `battle.rs`,
  `battle/fire.rs`, `supply.rs`, `units/digest.rs`.
- **Digest:** the plan is folded inside each occupant's hold digest exactly
  where `digest_place` folds it today: same order, same fields, empty-window
  history included. A squad stepping in still folds zero windows and seats.

## Confirm first

- The reader list above (drafts counted about ten call sites); re-grep for
  `.slots` and `building()` readers.
- Nothing recomputes a plan after entry today (drafts say `Garrison.slots` is
  computed once at entry).

## Verification

- `UNCHANGED_SINGLE_HOSTS` and `UNCHANGED_OTHER_SCENES`: byte-identical at
  every tick. If anything moves, find the first differing tick and fix it
  before going on.
- `LIVE_SCRIPT_REPLAY` green.
- The relief test stays the one expected red.

## Delegated

How `&Windows` is threaded (context structs, parameters); internal type
names.

## Must stay green

The whole sim crate except the relief test.
