# Unit concealment and spotting

**Shipped 2026-10-29** on `spec/unit-concealment` ([PR #20](https://github.com/dzhng/battlesim/pull/20)). The decisions were settled with the user on 2026-10-28 in [the unknowns map](assets/unknowns-map.html). The [decisions ledger](choices.md) records what was decided where the plan was silent.

Every unit type carries its own visual `concealment`. It multiplies the concealment its ground gives it. Each observer's `spotting` cancels concealment or deepens it, and a unit under a move order loses some of its cover. A sniper team is hard to find even in a field. A scout finds it farther out, and a buttoned-up MBT has to come close. A small vehicle hides in a wood better than a main battle tank, and a tank driving through forest gives itself away.

To make that a value on each unit rather than a branch in code, config stopped hiding values in code. Forest has one concealment value, sight into each altitude band is authored on every unit, the radar signature is a flag, and the weapon, air and rules fallbacks are authored data. This was a mechanism change, not a balance pass: the shipped numbers are starting guesses.

## The contract

For each visibility sample (a hull's two, or each living soldier):

```
ground    = min(forest.concealment, 1 + (building_range_multiplier − 1) × shelter)   // 1 in the open
target    = unit.concealment × ground × (unit.moving() ? sensors.moving_concealment : 1)
            // an airborne unit returns (1, 1) before any of this
effective = spotting ≥ 1 ? min(1, target × spotting) : pow(min(1, target), 1 / spotting)
identified when distance ≤ sight × effective, line of sight is clear, and foliage allows;
            a squad needs squad_identification_fraction of its living soldiers seen,
            and an observer that doesn't see the ground skips ground targets
HIDDEN    = more than squad_hidden_fraction of samples have ground < 1 and target < 1,
            and no visible enemy observer or known attacker reveals the unit
```

## Why it works this way

- **Values live on the unit.** `development.md`'s "values live on the unit" paragraph was written for this spec. When a value differs between units, author it on each unit type, or once on a shared profile (`extends`). Code must not derive a value from a body, a class or another field; the known exceptions are under Follow-ups. Leaving a field out may only mean "none" or "off". This is why forest lost its infantry/vehicle split, sight lost its body table, radar stopped silently raising sight, and the fallbacks became data. Bulk retunes are scripts over the data, not rules in code.
- **Good spotters cancel all concealment, capped at full sight.** The user wanted recon to cancel forest and every other buff. A spotting power would never cancel anything. Cancelling terrain only would leave scouts no better than a rifle squad at finding a sniper in the open.
- **Poor spotters deepen concealment by a power.** A power is continuous, and it never touches an unconcealed target (`pow(1, x) = 1`). Multiplying only when the target is concealed would make range jump at the boundary.
- **Both branches cap at 1.** Without the cap, a 2.8 hull seen by a 0.8 spotter would come out at 3.6× sight.
- **The moving multiplier applies to the whole product, and only the final value is capped.** Capping earlier would let a moving tank hide better than a parked one.
- **A unit's own concealment counts in the open.** It's the ghillie suit in a field. Ground still decides HIDDEN, so a sniper team can be hard to identify in the open without the readout saying HIDDEN.
- **Moving means a move order is in progress** (`MoveState::Moving`), not per-body motion. Soldiers shuffling inside a held squad aren't moving.
- **HIDDEN uses net concealment.** The readout stays truthful, so an MBT driving through forest loses it. The user called that a nice emergent property.
- **Firing reveals only the contact patch.** It was verified that the patch can be fired on and can kill, so there's no separate firing penalty.
- **`concealment` is required, `spotting` optional.** Concealment has no neutral value: 1 on a hull would mean "as hidden as a soldier". Spotting's neutral value is 1, which the "none or off" rule allows to be omitted.
- **Aircraft are hulls valued by volume, with no special case.** Their value matters only on the ground, because an airborne unit returns (1, 1).
- **Visual spotting only.** Aircraft and radar stealth were out of scope by the user's call.
- **Every transcendental function in library code under `crates/` is libm's.** Native and WebAssembly must agree bit for bit. The drafts found `foliage_reach` using the standard library's `exp`, and the user asked for the whole rule, guarded, not a point fix.

## Invariants

- **One owner per concept:**

  | Concept | Owner |
  |---|---|
  | The formula | `sensing::ground`, `sensing::target`, `sensing::effective` |
  | Per-sample `(ground, target)` | the private `sensing::sample`, read by both `evaluate` and `concealed` |
  | Moving | `Unit::moving()` (also read by `hearing.rs`) |
  | Sight per altitude band | `Sensors::reach_m(layer)` |
  | Radar signature | `capabilities.radar_emitter`, read by `knowledge.rs` and `weapons.rs` |
  | Forest concealment | `WorldGeometry::forest_concealment(x, y)` |
  | Sensor rule admission | `SensorRules::check` |
  | Report grouping | the spotting report example; none of it lives in the `sim` library |

- **The evaluation order is fixed:** unit × ground, then moving, then spotting. Reordering would change digests.
- **Rules refuse unknown keys**, through `deny_unknown_fields` or, for the flattened weapon rows, the key probe in `resolve_weapons`. A deleted or misspelled key fails at load instead of loading silently.
- **Resolved data is complete.** A weapon row's `partial_from` and each target layer's `range_m` and `scatter_mrad` are required, and so are an aircraft's `mobility.air.layer`, both multipliers of a `capabilities.air.afterburner` when one is authored, `low_air_m` and `high_air_m` on every unit, and the rules' squad fractions, formation, vehicle aim height and `moving_concealment`. Inheritance may supply them; code never does.
- **A layer's reach never exceeds the row's `range_m`**, which is the round's flight bound. Each layer owns its spread, and there is no row-level spread. A powered, unguided layer's spread at its reach is no tighter than `physics.min_spread_at_max_range_m` (`FlightConfig::check_layer`).
- **Admission ranges:**
  - `forests.rule.concealment` lies in (0, 1].
  - `moving_concealment` is at least 1, so moving never improves concealment.
  - `building_range_multiplier` lies in (0, 1].
  - Unit `concealment` and `spotting` are finite and above 0, and both may exceed 1.
- **The guard:** `crates/clippy.toml` disallows every f64 and f32 transcendental method. Clippy runs with `-D warnings`, so the guard covers every crate under `crates/`: `game-wasm`, `presentation-wasm` and `pipelines` on every target. `native/client` sits outside `crates/` and outside the guard, deliberately: it is native-only, and its two transcendental calls convert audio gain, which no battle or digest reads. Contract, sim, mapgen and presentation set `disallowed_methods = "allow"` in their `Cargo.toml` and `#![deny(...)]` in `lib.rs`, so the guard holds on their library code (including `#[cfg(test)]`) but not on their native-only tests and examples.
- **Tests test mechanics, not configuration.** The system test authors its own values. No test pins a roster value.

## Pointers into the code

| Area | Where |
|---|---|
| Formula, samples, HIDDEN | `crates/sim/src/sensing.rs` (`effective`, `ground`, `target`, `sample`, `concealed`, `evaluate`) |
| Moving | `crates/sim/src/units.rs` `Unit::moving` |
| Unit fields | `crates/contract/src/catalog.rs` `UnitType::concealment`; `crates/contract/src/catalog/sensors.rs` (`Sensors`, `spotting`, `reach_m`) |
| Rules | `crates/contract/src/scenario/sensors.rs` (`SensorRules`, `check`); `crates/contract/src/scenario/forest.rs` `ForestRule::concealment` |
| Forest ground | `crates/sim/src/world/forest.rs` `forest_concealment` |
| Weapon layers | `crates/contract/src/weapons.rs` (`partial_from`, `LayerFire`, `against`); `crates/sim/src/flight/mod.rs` `FlightConfig::check_layer` |
| Data | `fixtures/game.json` (`sensors.moving_concealment`, `forests.rule.concealment`); `fixtures/units/**` (`concealment`, `sensors.*`); `fixtures/parity/map-layout/physical-rules.json` (frozen, hand-edited) |
| Docs | `crates/sim/README.md#sensing`, `fixtures/README.md`, `development.md#game-rules` |
| Editor | `apps/mechanics-editor/src/fields.ts` (Concealment, Spotting, per-layer range and spread) |

The tests that pin it:

- **System:** `crates/sim/tests/concealment.rs` `concealment_controls_identification_and_hidden`. One functional test covers open ground, forest, garrison, scouts, poor spotting, movement, aircraft and replay.
- **Formula:** `crates/sim/src/sensing.rs` `spotting_cancels_or_deepens_concealment_within_full_sight`.
- **Sight bands:** `crates/sim/tests/radar.rs` `each_bands_sight_is_the_units_own_and_zero_is_blind`.
- **Admission:**
  - `crates/contract/tests/weapons.rs` `resolved_weapons_require_explicit_damage_and_layer_values`;
  - `crates/contract/tests/air_admission.rs`;
  - `crates/sim/examples/mechanics_validate.rs` `unknown_rules_keys_are_refused` (example tests run with the workspace, `test = true`).

The tools:

- `cargo run -p sim --release --example spotting_report` prints the balance matrix, through the sensing functions, and writes `throwaway/spotting/report.html`. Rows group targets by concealment, columns group observers by (front sight, spotting), and each target appears in the open, in forest and, for squads, garrisoned, both still and moving.
- `cargo run -p sim --release --example concealment_shots` films real approaches into GIFs (canvas shared with `movement_shots` via `examples/common/canvas.rs`).

## Where the build diverged from the plan

- **`sensors.concealed_range_multiplier` became `sensors.spotting`**, and its meaning widened: values below 1 now deepen concealment by a power, where the old field only allowed 1 or more.
- **The row-level weapon `scatter_mrad` was deleted.** It wasn't planned. Once each layer authored its own spread, nothing in a battle read the row's, and the editor's main "Landing spread" field would have edited a value with no effect. The accuracy ceiling moved to each layer. The editor's range-to-spread coupling went with it: each layer's range and spread now edit independently.
- **Emitting no longer raises sight.** Test patches that made a unit an emitter now author `high_air_m` themselves where they relied on it.
- **HIDDEN runs one path for hulls and squads**, over the same samples sighting uses. A hull's two samples share a ground point, so its answer didn't change.
- **Several oversized files were split by concept** to stay under the size ratchet, not squeezed:
  - `contract::catalog::sensors` and `contract::scenario::sensors`;
  - `sim::units::digest`;
  - `contract::scenario::forest`;
  - the editor's `fieldSchema.ts` and `mountFields.ts`;
  - `contract/tests/air_admission.rs` and `contract/tests/common/catalog.rs`;
  - `web/tests/sceneAssets/unitType.ts`.
- **Net code:** the library shrank (−55 lines), but the two developer tools (+733) and the system test (tests +223) grew the total. That was accepted: the spec asked for the tools, and added config was allowed.
- **The published HIDDEN can trail a direct `sensing::evaluate` by one tick**, because it reads the battle's own sightings. The GIF lanes run on until the readout clears.

## What the neutral half proved

Everything before the starting values changed no battle. In each change:
- the digest rows of every simulation parity record were identical;
- every map-layout outcome was identical, both its map hash and its body with identity hashes stripped;
- the menu reel passed without re-recording.

Record files re-blessed only their embedded inputs. The neutral data was hulls 2.8, squads 1, forest 0.25 and moving 1.0, since `2.8 × 0.25` reproduces the old vehicle forest value of 0.7 exactly. Browser parity (`exactNumbers`, `skirmishObjectives`, `skirmishPurchases`, `mapLayout`) passed with only the records' embedded inputs re-recorded; no digest moved.

## Starting values (data only, to retune with the report)

| Who | `concealment` | `spotting` |
|---|---|---|
| Rifle, assault, marksman, weapon and RPG teams | 1 | 1 |
| Scouts and elite scouts | 0.8 | 2 |
| Sniper teams | 0.5 | 2 |
| Every hull by box volume (`8·x·y·z` of `half_extents_m`): ≤ 35 m³ / 35–65 / > 65 | 1.8 / 2.4 / 2.8 | profile |
| Wheeled and tracked recon | by volume | 1.5 |
| The five `*_mbt` profiles | by volume | 0.8 |

`sensors.moving_concealment` is 1.5 and forest is 0.25. Hull values are authored per unit, because profiles don't follow size. The values changed the Market Town menu reel's battle (from tick 1) and the street fog vectors (from tick 62), and the browser capture records were re-recorded. The simulation parity records' stored digests and the Paris Corner battle are unchanged. The reel was judged before and after: every followed unit is alive and framed at its shot.

## Dead ends

- **Per-file `#![allow]` attributes for the libm guard in test files.** The size ratchet refuses growth in oversized test files. The guard moved to package-level allow plus a library-level deny.
- **A fallback table keyed on body, and a radar that silently overrides sight.** Both put values in code.
- **An infantry/vehicle forest split.** Size lives in the unit's concealment.
- **Per-body physical motion (`Moved`) as "moving".** It was too noisy.
- **A firing concealment penalty.** The contact patch already reveals and can be fired on.
- **Making both fields required, or both optional.** See the reasons above.
- **Special-casing aircraft at 1.** An earlier draft of the values had it.
- **Porting the editor's range-to-spread coupling to every layer.** It was more code than the convenience was worth.

## Follow-ups (outside this spec)

- **Balance.** With these values, a 500 m MBT with spotting 0.8 identifies a still forest sniper team at about 37 m. The spotting report shows every such cell.
- **Suppression's `move_penalty`** is the one movement slowdown left outside `movement.speed_factors`' naming: it is subtracted from full speed, and turning the pinned tier's 0.7 into a factor of 0.3 would move digests by one unit in the last place.
- **Map generation's `roads.dirt_track_speed_factor` preset** copies the dirt-track factor to time journeys while laying a map out; it doesn't read the game rules.

## Visual provenance

- [assets/unknowns-map.html](assets/unknowns-map.html) is the four-quadrant map from the 2026-10-28 walk with the user. It records the decisions and landmines the shape came from. There was no external reference image: the standard was the war-film test in `.agents/skills/tweak-mechanics`, judged on the GIF and the menu reel.
- [assets/concealment-shots.png](assets/concealment-shots.png) is the contact sheet from `concealment_shots`. It shows, for each observer, the frame before identification beside the frame of identification, plus the forest MBT parked (HIDDEN) and driving (not HIDDEN). The scenes use equal 600 m eyes, so they compare the mechanism alone:

  | Observer | Open | Forest edge |
  |---|---:|---:|
  | Rifle | 302.7 m | 82.7 m |
  | Scouts | 602.7 m | 152.3 m |
  | MBT | 252.5 m | 53.8 m |
