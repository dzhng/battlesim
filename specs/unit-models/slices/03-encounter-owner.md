# 03 Encounter owner

**Unlocks:** the opponent and referee every encounter battle uses (the menu
reel included) live under their real name, so retiring the village (slice 06)
deletes only village code.

## Contract

`village::Defender` and `village::Referee` move to `sim::encounter`
unchanged. They are already role-keyed (`at`, `mbt`), not id-keyed. `battle.rs`
(`:43`, `:271-274`, `:762-768`) and the encounter planner
(`crates/sim/src/encounter/mod.rs:859-873`) call them there. A move only: no
behaviour, field or name change beyond the module path.

## Verify

Every existing digest and parity fixture is byte-identical (this is the proof
the move moved nothing); `sim` tests that touch encounters
(`crates/sim/tests/encounter.rs`, the menu encounter's native run).

## Delegated

Module layout inside `sim::encounter`.
