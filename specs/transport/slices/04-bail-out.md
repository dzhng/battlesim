# 04 · The bail-out library, applied to building collapse

**Owns:** D11 (the building row and the shared library), the building half of D12, L4 (bail-out draws), L10. **Depends on:** 02. **Outcome change:** named. A collapse now injures everyone and kills some, and it no longer draws from `damage_rng`.

Load `tweak-mechanics` first.

## Contract
- `Rules.bail_out` with the three rows in the README (`building`, `ground`, `air`). This slice uses only `building`.
- `sim::bail_out` (new module):
  - Input: `BailOut { host: Host, from: V3, occupants, rule: &BailOutRow }`.
  - Output: `Vec<Fate>`, where each fate is either `Out { at, hp_lost }` or `Died { cause: SquadLost | Roll | NoRoom | LeftMap }`.
  - A separate `apply` writes health, deaths (`Soldier::fall` at ground height from `surface_at`, never at the host's height), the pinned state (suppression raised to at least `rules.suppression.pinned.level`, as today), and the death records that drive kill rewards and casualties.
- **Random draws.** Each event uses its own generator, `Rng::new(mix(seed ^ BAIL_OUT_STREAM ^ host key ^ tick))`, following `arrangement::rng` (`arrangement.rs:23`). It never draws from `damage_rng`.
- **Order of rolls per event:**
  1. the `squad_loss` roll, if the row has one, once per squad;
  2. then per soldier, in member order: the `death` roll, then the `hp_lost` draw (uniform in the range), then the spot search;
  3. a survivor whose `hp_lost` reaches his health dies.
- **Spot search:** the new shared one (`arrangement::nearest_free` with a clearance that excludes props, live hulls and other soldiers, up to `search_m`, with z from `surface_at`). No spot means death (`NoRoom`).
- **Hard cutover:**
  - Delete `garrison.survival_probability_on_collapse` and `garrison.exit_search_radius_m` from `GarrisonRules` (`crates/contract/src/scenario.rs:1168-1172`) and from `fixtures/game.json:1080-1089`.
  - Add `deny_unknown_fields` to `GarrisonRules`.
  - `garrison::collapse` (`garrison.rs:792-879`) becomes a caller of `bail_out`. Delete its own survival roll and spot search.

## What a human can see
In the garrison or buildings lab, a collapsed building's survivors appear close outside it, injured and pinned.

## Verification
- **Rewrite the collapse tests** to assert the new rule, not the old one:
  - `tests/garrison.rs`: `a_collapse_leaves_a_lower_ruin_and_accounts_for_every_occupant` (:847), `a_survivor_with_no_legal_way_out_dies_rather_than_teleporting` (:939), `a_survivor_squeezes_out_where_a_soldier_fits` (:983);
  - the settings in `tests/buildings.rs` tests `tall_buildings_leave_a_terminal_ungarrisonable_shell` (:1007) and `upper_floor_collapse_deaths_land_on_the_remaining_physical_surface` (:1169).

  Each asserts that every survivor lost health within the row's range and that every occupant is accounted for, alive or dead, once. Search over seeds rather than pin one.
- The "no second survival roll" assertion inside `corpse_support_loss_is_physical_but_enemy_memory_waits_for_sight` (`battle.rs:5660`, at `:5822-5828`) becomes "a collapse never draws from `damage_rng`".
- Re-bless the parity records (`BLESS_PARITY=1`): `exact-numbers.json`, `skirmish-purchases.json`, `skirmish-objectives.json`.
- Update `specs/battle-foundation/slices/11-garrisons-and-ruins.md:56`, which describes all-or-nothing survival.
- Rung 2: `cargo test -p sim`.
- **Digest probe:** battles with no occupied collapse keep their digests (rerun the slice 02 probe and exclude battles with a collapse).

## Delegated
Module and type names; how `apply` reports deaths to the existing `destroyed` and `casualties` paths; spot spacing (today 1 m, `ESCAPE_SPACING_M`, `garrison.rs:38`).

## Must stay green
Garrison entry and exit (`garrison::exit_spots` stays props-only), the menu reel, every digest of battles without an occupied collapse.

## Feedback that would change it
The numbers. They are starting guesses in `fixtures/game.json` and change without code.
