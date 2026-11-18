# 00 Make room

## Contract

Automatic target choice has one owner, `crates/sim/src/weapons/choice.rs`,
and `weapons.rs` has room for the later call sites. Behaviour is identical.

## Seam

Move `select`, `share_through`, `automatic_lock` and `catchable_round` out of
`crates/sim/src/weapons.rs` into a new `weapons/choice.rs`, keeping their
signatures and visibility within `weapons`. `choose_lock` may stay or move;
move it if `weapons.rs` still lacks headroom for slices 01 and 03.

Also make room in `crates/contract/src/scenario.rs` (over budget) for the one
field and one `mod`/`use` line slice 01 adds: move a self-contained rules
section out to its own file under `crates/contract/src/scenario/`, as
`air.rs`, `sensors.rs` and the others already are.

## Verify

- `cargo test -p sim --test sim` filtered to `target_priority`, and the sim's
  digest and replay tests: digests identical, replays identical.
- `cargo test -p contract` for the scenario move.
- The file-size check passes; both files shrink.

## Stays green

Everything. This slice changes no outcome.

## Delegated

- Which rules section moves out of `scenario.rs`.
- Module-internal helper placement and `use` lists.

## Feedback that would change it

None expected; it is mechanical.
