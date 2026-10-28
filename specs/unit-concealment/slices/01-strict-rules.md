# 01 Rules refuse unknown keys

**Unlocks:** a stale or misspelled rules key is refused when the rules are loaded. Every later slice deletes rules keys (`penetration`, `unaided_high_air_m`, the forest class split), and without this a leftover key would load without error and a test could pass while testing nothing.

## Seam

- Add `#[serde(deny_unknown_fields)]` to `UncheckedRules` and to **every** rules section struct in `crates/contract/src/scenario.rs`. Today many lack it: `SensorRules` (`:502`), plus the structs at `:231, 311, 343, 385, 466, 612, 1105, 1122, 1169, 1182, 1377, 1436`.
- `fixtures/game.json` also carries five top-level sections that aren't simulation rules: `status`, `seed`, `units`, `controls` and `presentation`. Both native (`sim::fixtures::game()` / `admit()`, `crates/sim/src/fixtures.rs:84,171,197`) and the browser (`web/src/battle/catalog/compose.ts:57,77`) pass the whole file in as rules, and `exact-numbers.json` embeds them. Give each section an explicit owner. Never silently discard arbitrary unknown keys.

## Delegated to you

- Where the envelope handling lives. Either the rules type declares the five sections, or the single place that reads documents strips them. Pick the one that leaves less code.
- Deleting `status`/`units`/`seed` if nothing reads them. `controls` is read only by `reverseZone.ts`.

## Verification

- **Red first:** one contract test, `unknown_rules_keys_are_refused`. It injects a stale `sensors.*` key and a stale top-level section and expects a named error from each, and it checks that the shipped `game.json` still loads. Exercise it through `mechanics_validate` (`crates/sim/examples/mechanics_validate.rs`), the path the editor uses.
- Fix every stale key this exposes, in this slice's commit.
- Every battle digest is identical; see the README's [proof](../README.md#proof-that-nothing-changed). Record files may change only in their embedded inputs.

## Must stay green

Contract, sim, mapgen and presentation crates; editor startup; browser scenario construction.

## Feedback that would change this slice

The user wanting the non-simulation sections moved out of `game.json` entirely. That would be a separate change, not part of this spec.
