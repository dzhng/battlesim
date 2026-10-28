# 02 Fallbacks become data

**Unlocks:** "leaving a field out means none or off, never a value code picks or copies" holds for weapons, air mobility and the rules. Each fallback below becomes authored data, and the code that supplied it is deleted. Author exactly the value each **resolved** type got before, so no battle changes. Use one commit per fallback.

## Seam and data

| Fallback | Delete | Author |
|---|---|---|
| weapon `partial_from` (falls back to the rules' `penetration.partial_from` = 0.6) | `PenetrationRules`, `Rules.penetration`, the `unwrap_or` in `damage.rs:126`, the assert at `damage.rs:96`, and the "a row with a blast authors its own" check (`weapons.rs:417`) | `WeaponDefinition.partial_from: f64`, required. Base rows carry it and children inherit. Inline test weapon rows author it too. |
| `targets.<layer>.range_m` and `scatter_mrad` (copied from the row) | The copying in `WeaponDefinition::against` | `LayerFire.range_m` and `scatter_mrad` become `f64`. **Inheritance trap:** 14 child rows override the row `range_m` but inherit `targets` (`ground_tank_*`, `advanced_tank_*`, `light_tank_*`, `kornet`, `spike`, `javelin`, `bastion`, `heli_atgm`, `akeron_mp`, `air_ground_missile`, `ifv_he`, `marksman_rifle`). Each must author its **own** former resolved layer range, or it silently inherits its parent's. The reel's encounter weapon pins (`maps/{market-town,street,paris-corner}/encounters/*.json`) need the values too. |
| Bombs | The rejection of any authored range on an unpowered layer (`weapons.rs:445`) | The former resolved value. Don't invent a bomb reach. Replace the rejection with whatever check keeps the old meaning. |
| air `mobility.layer` (low air when unnamed) | `default_air_layer` (`catalog.rs:328`) | `layer` on `test_heli` and the five helicopter profiles. |
| afterburner multipliers (1.5 / 2.0) | `afterburner_speed_multiplier` and `afterburner_fuel_multiplier` (`catalog.rs:446`) | 1.5 and 2.0 on `roster_profile_fixed_wing` (`air-war.json`, which authors `{}` today) and on the test literals. |
| rules-side defaults | The serde defaults on `SensorRules`' two squad fractions (`scenario.rs:505`), `formation` (`FormationRules::default`, `scenario.rs:957,1133`) and `vehicle_aim_height_fraction` (`scenario.rs:393`) | `game.json` already authors the first two. Author `vehicle_aim_height_fraction` with its former default. |

**Editor (same slice, or it offers invalid edits):** in `apps/mechanics-editor/src/fields.ts`:
- stop marking `partial_from` and the layer range and spread as optional, and delete the "None uses…" text (`:564-568, 606-617`);
- give the `initial` objects for new `targets.ground` and `targets.low_air` entries (`:582, 596`) their range and spread;
- update `web/tests/mechanicsEditorFields.test.ts` if it pins the optional flags.

`fields.ts` is over the size budget, so this slice must net-shrink it.

**TypeScript:** in `packages/scene-assets/src/units.ts:327`, the layer `range_m` becomes required.

## Delegated to you

Which inheritance level carries each value (the smallest set of rows), and removing intermediate types that become redundant.

## Verification

- **Red first:** replace `contract/tests/weapons.rs` `an_explosive_row_states_its_own_partial_from` with `resolved_weapons_require_explicit_damage_and_layer_values`. It covers an inherited child, an unpowered bomb, a missing `partial_from` and a missing layer range. Extend the existing air admission coverage with a missing `layer` and an empty afterburner.
- **Before blessing:** dump every resolved weapon's effective per-layer range, scatter and `partial_from`, and every aircraft's layer and afterburner multipliers. Do this for the game, test and capture sets, both before and after, in `throwaway/`. The two dumps must be identical.
- Regenerate the catalog with `BLESS_CATALOG=1 cargo test -p sim --test sim catalog::`. Only the weapon and air rows may change.
- Every battle digest is identical ([proof](../README.md#proof-that-nothing-changed)).

## Must stay green

Damage, air weapons, editor admission, preview and save, and every digest.
