# Unit concealment and spotting

**Status:** in progress (slices 00–02 landed). All decisions were settled with the user on 2026-10-28 ([the unknowns map](assets/unknowns-map.html)).
**Updated:** 2026-10-28.

Every unit type gets its own concealment, which multiplies the concealment its ground gives it. Each unit's spotting can cancel concealment or deepen it, and a moving unit loses some of its cover. Along the way, config stops hiding values in code: forest gets one value, sight bands and the radar signature become authored data, and four other fallbacks go. This spec sets up the mechanisms. It is not a balance pass: the values are starting guesses, and slice 05's report is the tool for retuning them.

## Next Agent Prompt

You are implementing this spec in its pull request. Start at the first unchecked slice and follow its file. Use `implement-spec`. Use `write-tests` (red first) for every behaviour, and `tweak-mechanics` before slice 06. Finish the whole spec without asking for approval: judge the digests, the menu reel and the workbench GIF yourself, and say in each commit what you checked. Fix any failure you see, and fix unrelated failures in their own commit. Before you end a pass, tick what you finished and rewrite this section.

- [x] [00 Every transcendental function is libm's](slices/00-libm-everywhere.md): done. No digest, record, reel or map moved.
- [x] [01 Rules refuse unknown keys](slices/01-strict-rules.md): done. Digest rows and compiled map hashes are identical.
- [x] [02 Fallbacks become data](slices/02-explicit-fallbacks.md): done. The row-level `scatter_mrad` also went, since layers own spread (see choices). Digest rows and map hashes are identical.
- [ ] [03 Sight bands and the radar signature](slices/03-sight-bands-and-radar.md): `low_air_m` and `high_air_m` become required, `radar_emitter` replaces the radar struct, and the TypeScript copies are deleted.
- [ ] [04 The concealment mechanism, neutral](slices/04-concealment-mechanism.md): the whole formula, moving and HIDDEN, coded with neutral data, plus the system test and editor fields.
- [ ] [05 Spotting report](slices/05-spotting-report.md): the balance matrix, generated from real data through the sensing functions.
- [ ] [06 Starting values](slices/06-starting-values.md): data only. This slice changes battles, so re-record and judge.
- [ ] [07 Workbench GIF and closeout](slices/07-gif-and-closeout.md): the scripted GIF, docs, one full run, then `close-spec`.

**Warning:** slices 01–05 must not change any battle, measured against the baseline slice 00 leaves. "Unchanged" means the digest rows inside each parity record. It doesn't mean the record files themselves, which embed rules and units and so change with every schema edit (see [proof](#proof-that-nothing-changed)).

## Slice graph

```
00 libm ─▶ 01 strict rules ─▶ 02 fallbacks ─▶ 03 sight bands + radar ─┐
                                └▶ 04 concealment mechanism ┴▶ 05 report ─▶ 06 values ─▶ 07 GIF + close
```

03 and 04 are independent; land them in either order. Slice 00 may move native digests (it makes native agree with WebAssembly). Slices 01–05 change no battle. Slice 06 is the only one that changes battles, and it is data only. If 06 needs a code change, the mechanism was wrong: stop and reslice 04.

## The contract

Per visibility sample (a hull's two, or each living soldier):

```
ground    = min(forest.concealment, 1 + (building_range_multiplier − 1) × shelter)  // 1 in the open
target    = unit.concealment × ground × (unit.moving() ? rules.sensors.moving_concealment : 1)
            // an airborne unit returns (1, 1) before any of this, as today
effective = spotting ≥ 1 ? min(1, target × spotting)
                         : libm::pow(min(1, target), 1 / spotting)
identified when distance ≤ sight × effective, line of sight is clear, and foliage allows (sees_point, unchanged)
HIDDEN    = enough samples with ground < 1 and target < 1 (squad_hidden_fraction, as today)
```

## Single owners

Each concept has exactly one owner. No slice may add a second.

| Concept | Owner |
|---|---|
| The formula | `sensing::effective(target, spotting)`, plus `sensing::ground(...)` and `sensing::target(...)` (public), all in `crates/sim/src/sensing.rs`. `evaluate`, `concealed` and `spotting_report` all call them. |
| Per-sample concealment | One private adapter in `sensing.rs` returns `(ground, target)` for a sample. `evaluate` and `concealed` both use it. `concealed` loses its separate hull path. |
| Moving | `Unit::moving()` (`state == MoveState::Moving`) in `units.rs`. `hearing.rs` switches to it. |
| Sight per altitude band | `Sensors::reach_m(layer)` in `catalog.rs`. `sight::of_layer` calls it. `UnitType::high_air_sight_m` and the TypeScript `highAirSight` are deleted. |
| Radar signature | `capabilities.radar_emitter`, read directly by `knowledge.rs` (receivers) and `weapons.rs` (anti-radiation). |
| Forest concealment | `WorldGeometry::forest_concealment(x, y)`, which loses its infantry/vehicle argument. |
| Sensor rule validation | `SensorRules::check`. `sensing::validate` folds into it. |
| Report grouping | Only `crates/sim/examples/spotting_report.rs`. None of it goes into `sim`. |

**End state:** this reads as if it had been designed today. Nothing derives a value from a body, class or another field. `development.md`'s "values live on the unit" paragraph holds everywhere this spec touches.

## Data

| Field | Where | Rule |
|---|---|---|
| `concealment` | top level of the unit type, beside `sound` | **Required** in the resolved type, so inheritance through `extends` satisfies it. Finite and > 0, and may exceed 1. It has no neutral value: 1 on a hull would mean "as hidden as a soldier". Planned cards (`"planned"`, parsed as `PlannedUnit`) are not physical units and don't carry it. |
| `sensors.spotting` | renamed from `sensors.concealed_range_multiplier` | Optional: omitted means 1, which is the neutral value. Finite and > 0, and may go below 1. |
| `sensors.low_air_m`, `sensors.high_air_m` | `Sensors` | **Required**, as plain numbers. `0` means blind. |
| `capabilities.radar_emitter` | replaces `capabilities.radar { emits, detection_m }` | `true` makes the unit a radar signature. Each emitter's old `detection_m` moves into its `high_air_m`, and nothing ties the two together. |
| `forests.rule.concealment` | replaces `concealment_infantry` and `concealment_vehicle` | One value in (0, 1]: 0.25. |
| `sensors.moving_concealment` | rules | Required and ≥ 1. 1.0 in slices 04–05, 1.5 from slice 06. |

**The "values live on the unit" rule** (in `development.md`): leaving a field out may only mean "none" or "off", never a value that code picks or copies. Opt-in flags that default to `false`, zero minimums, full gravity, and an unguided round's physical lifetime bound stay optional. Slice 02 lists the fallbacks that break the rule.

**Starting values** (slice 06; all guesses, to tune with the report):

| Who | `concealment` | `spotting` |
|---|---|---|
| Rifle, assault, marksman, weapon and RPG teams | 1 | 1 |
| Scouts and elite scouts (`roster_profile_scout`, `_elite_scout`) | 0.8 | 2 |
| Sniper teams (`roster_profile_sniper`) | 0.5 | 2 |
| Every hull, by box volume (`8 × x × y × z` of `half_extents_m`): ≤ 35 m³ | 1.8 | profile |
| 35–65 m³ | 2.4 | profile |
| > 65 m³ | 2.8 | profile |
| Wheeled and tracked recon profiles | by volume | 1.5 |
| The five `*_mbt` profiles (they reach 16 units, none overriding spotting) | by volume | 0.8 |

"Every hull" includes aircraft and the test and capture hulls, with no special case. An aircraft's value only matters when it isn't airborne. Profiles don't follow size (`roster_profile_wheeled_ifv` holds both the Stryker Dragoon and the S-300), so hull concealment is authored per unit. A one-off script in `throwaway/` writes literal values; only the data is committed.

## Proof that nothing changed

Slices 01–05 must leave every battle identical:

- **Simulation parity records** (`fixtures/parity/{exact-numbers,skirmish-objectives,skirmish-purchases}.json`). They embed the scenario's rules and units, so `BLESS_PARITY` is needed after each schema change. Before blessing, save their `rows`, `digest` and `ticks`. After blessing, compare those fields byte for byte; only the embedded inputs may differ.
- **The menu reel** (`menu_reel.rs`) passes without being re-recorded.
- **Map layout records** (`fixtures/parity/map-layout/`). Generation identity hashes the whole resolved catalog and forest rule (`GenerationPhysics::hash`, `crates/contract/src/generation_physics.rs:109`, which feeds `config_hash`), so `native_sha256` changes. Re-bless them and prove that `map_hash` and the map itself are unchanged. That follows the existing pattern at `generation_physics.rs:46`.
- **Browser parity tests** (`exactNumbers`, `skirmishObjectives`, `skirmishPurchases`, `mapLayout`) run once at the end of slice 05.

## Firewalls

- **Net less code.** The spec as a whole deletes more code than it adds; added config lines are fine.
- **No compatibility layers, migrations or shims.** It's a hard cutover. Re-record fixtures instead.
- **Never narrow a mechanic to keep a recording.** The reel's units (`fixtures/units/capture/`) and scenario may be changed freely to get the scene.
- **Files over the size budget may only shrink.** These include `apps/mechanics-editor/src/fields.ts`, `crates/contract/src/catalog.rs`, `crates/contract/src/scenario.rs`, `crates/contract/tests/catalog.rs` and `crates/sim/tests/air.rs`, all enforced by the pre-commit ratchet. Pay for any growth by deleting code (the fallbacks, `RadarCapability`, `high_air_sight_m` and its TypeScript copy) or by splitting by concept.
- **Every transcendental function under `crates/` is `libm`'s** (slice 00 adds the clippy guard), so that native and WebAssembly agree.
- **Tests test mechanics, not configuration.** No test may pin a roster value. The system test authors its own values.

## Decisions and why (from the walk)

| Decision | Why | Rejected alternative |
|---|---|---|
| Good spotters cancel all concealment, with a cap at 1 | The user wants recon to cancel forest and other buffs | Spotting as a power (never cancels anything); cancelling terrain only (scouts would be no better than a rifle squad at finding a sniper in the open) |
| Poor spotters deepen concealment by a power | Continuous, and never touches an unconcealed target | Multiplying only when the target is concealed: range would jump at the boundary |
| A unit's own concealment counts in the open | A ghillie suit in a field | Only scaling ground concealment |
| One forest value; size lives in the unit's concealment | First principles: smaller vehicles hide better than MBTs | The infantry/vehicle class split |
| Moving = the unit's move order is in progress | Soldiers shuffling inside a held squad shouldn't be noisy | Per-body physical motion (`Moved`) |
| HIDDEN uses net concealment | The readout stays truthful; an MBT driving through forest loses it, which is an emergent property the user likes | Ground only |
| Firing reveals only the contact patch | Verified that the patch can be fired on and can kill (`knowledge.rs:248`, `weapons.rs:1364`, `damage.rs:2`) | A firing concealment penalty |
| Sight bands and the radar signature become data | Config over business logic | A fallback table keyed on body; radar silently overriding sight |
| `concealment` required, `spotting` optional | Concealment has no neutral value; spotting's neutral value is 1 | Both required, or both optional |
| Visual spotting only | User scope | Aircraft and radar stealth |
| Every transcendental function in `crates/` is `libm`'s, guarded by clippy | Native and WebAssembly must agree. The user asked for this on 2026-10-28 after the drafts found `foliage_reach` using the standard library's `exp` | Fixing only `foliage_reach` |
| Aircraft are hulls like any other, valued by volume | One rule with no special case; the value only matters when an aircraft isn't airborne (synthesis decision, 2026-10-28) | Special-casing aircraft at 1, which an earlier draft of the table had |

## Out of scope and follow-ups

- **Aircraft and radar stealth.**
- **Balance.** Retune with the report after this spec.
- **The `low_air_m` / `unaided_high_air_m` naming confusion** goes away with slice 03. No separate rename is needed.
