# Unit concealment and spotting

**Status:** decided, not started. Every question below was settled with the user on 2026-10-28 ([the unknowns map](assets/unknowns-map.html)). Implement it to the end without asking for approvals: judge digests, the reel and the workbench GIF yourself.
**Updated:** 2026-10-28.

This spec sets up the mechanisms. It is not a balance pass: the values below are starting guesses that the spotting report exists to retune.

## Next Agent Prompt

Use `implement-spec`. Run `tweak-mechanics` before slice 2, and `write-tests` (red first) for each behaviour. Update this section before you end each pass.

- [ ] **Slice 0: rules reject unknown keys.** `SensorRules` and `UncheckedRules` (`crates/contract/src/scenario.rs`) get `deny_unknown_fields`, in their own commit. Fix any stale keys it exposes there too.
- [ ] **Slice 0b: no hidden fallbacks outside sensing.** The four fallbacks under "Other fallbacks" below become authored data, and the code that supplied them is deleted. Author exactly the values the fallbacks produced, so every digest stays the same.
- [ ] **Slice 1: the mechanism, with no battle changes.** The contract changes below, the formula as one public function in `sensing.rs`, the forest collapse with every hull at 2.8, required sight bands, `radar_emitter`, and `spotting_report`. Proof: every digest and parity record is unchanged.
- [ ] **Slice 2: what changes battles.** The moving rule, spotting below 1, and the banded values. Re-record parity and the menu reel, then judge them yourself.
- [ ] **Slice 3: tools and proof.** Mechanics editor fields, one large system test, and a scripted workbench GIF.
- [ ] Run everything once, then archive with `close-spec`.

## The contract

Per visibility sample (a hull's two, or each living soldier):

```
ground    = min(forest.concealment, building shelter)     // 1 in the open; aircraft always 1 (early return stays first)
target    = unit.concealment × ground × (moving ? rules.sensors.moving_concealment : 1)
effective = spotting ≥ 1 ? min(1, target × spotting)
                         : min(1, target) ^ (1 / spotting)
identified when distance ≤ sight × effective, line of sight is clear, and foliage allows (sees_point, unchanged)
```

- **`moving`** means `unit.state == MoveState::Moving`, a whole unit following a route under an order. This is the same signal the sound cue reads (`hearing.rs`). Waiting, halted, packing, planning and route-blocked units are still. Soldiers shuffling inside a held squad don't count.
- **Caps.** The moving multiplier applies to the whole product, and only the final value is capped. Capping the unit's own value first would make a moving tank harder to see than a parked one. Both spotting branches are capped at 1.
- **One owner.** The formula is one public function in `crates/sim/src/sensing.rs`. `evaluate` and `spotting_report` both call it. Target-side concealment is computed once per sample; spotting is applied per observer.
- **HIDDEN** (`sensing::concealed`) lights when `ground < 1` **and** the net target concealment (unit × ground × moving) is below 1. It reads `unit.state` at publication, so no new state is stored. As a result, an MBT driving through forest (0.7 × 1.5) loses HIDDEN, which is intended.

## Data

| Field | Where | Rule |
|---|---|---|
| `concealment` | top-level on the unit type, beside `sound` | **Required** in the resolved type, so inheritance through `extends` satisfies it. Finite and > 0, and may exceed 1. It has no neutral value: 1 on a hull would mean "as hidden as a soldier". |
| `sensors.spotting` | renamed from `sensors.concealed_range_multiplier` | Optional: omitted means 1, which is normal sight and the neutral value. Finite and > 0, and may go below 1. |
| `sensors.low_air_m`, `sensors.high_air_m` | already on `Sensors` | **Required.** The `ground_m` fallback and `rules.sensors.unaided_high_air_m` are deleted. |
| `capabilities.radar_emitter: true` | replaces `capabilities.radar { emits, detection_m }` | A pure signature: radar warning receivers and anti-radiation weapons read it. Each emitter's old `detection_m` moves into its `sensors.high_air_m`, and nothing ties the two together. |
| `forest.concealment` | replaces `concealment_infantry` and `concealment_vehicle` | One value, 0.25. |
| `sensors.moving_concealment` | rules (`fixtures/game.json`) | 1.5. |

### Other fallbacks

The rule: leaving a field out may only mean "none" or "off". It never stands for a value that code picks, or copies from another field, a class or a rules table. Opt-in flags that default to `false`, zero minimums, full gravity, and the physics bound on an unguided round's lifetime are all "off" or "physical", so they stay optional. These four do not:

| Field | Today when omitted | After |
|---|---|---|
| weapon `partial_from` | the rules' `penetration.partial_from` (0.6); 31 of 49 weapon rows rely on it | required on every row, with base rows (`extends`) carrying it; the rules' `penetration` section is deleted |
| `targets.<layer>.range_m` and `scatter_mrad` | copied from the row's own values; 30 layer entries rely on this | required in every layer entry |
| air `mobility.layer` | low air (`default_air_layer`) | required |
| afterburner `speed_multiplier` and `fuel_multiplier` | 1.5 and 2.0 (default functions) | required |

Starting values (guesses, to tune with the report):

| Who | `concealment` | `spotting` |
|---|---|---|
| Rifle, assault, marksman, weapon and RPG teams | 1 | 1 |
| Scouts and elite scouts (`roster_profile_scout`, `_elite_scout`) | 0.8 | 2 |
| Sniper teams (`roster_profile_sniper`) | 0.5 | 2 |
| Hull, box volume ≤ 35 m³ (`8 × half_extents_m` product) | 1.8 | profile |
| Hull, 35–65 m³ | 2.4 | profile |
| Hull, > 65 m³ | 2.8 | profile |
| Wheeled and tracked recon profiles | by volume | 1.5 |
| The five `*_mbt` profiles | by volume | 0.8 |
| Aircraft profiles, drones | 1 | 1 |

Profiles don't follow size: `roster_profile_wheeled_ifv` holds both the Stryker Dragoon and the S-300. So hull concealment is per unit, written into the roster files by a one-off script in the scratch folder. Only the data is committed. In slice 1 every hull, including the test and capture hulls, is set to 2.8; `2.8 × 0.25 == 0.7` is exact in floating point, so no battle changes.

## Spotting report

`crates/sim/examples/spotting_report.rs` follows `crates/sim/examples/README.md`. It reads `fixtures/catalog.json` and `fixtures/game.json` and calls the sensing function. Rows group the target units that share an identical `concealment`, with their names listed, and each row is split into open, forest and garrisoned, both still and moving. Columns group observers that share an identical sight range and spotting. Cells are metres. It writes an HTML matrix to the scratch folder and a text table to the terminal. This is the balance tool, so make it quick to rerun.

## Sharp edges

- **Required low air changes inheritance.** About 25 test patches change only `ground_m` and relied on low air following it, and each now needs its low-air value written out. `crates/sim/tests/air.rs` `jeep_sees_the_helicopter(None)` tests the deleted fallback; delete it.
- **Radar readers.** `knowledge.rs` (receivers), `weapons.rs` (anti-radiation), `catalog.rs` (`high_air_sight_m`, emitter validation), `packages/scene-assets/src/units.ts` (the TypeScript copy of `highAirSight` becomes `sensors.high_air_m`, and its optional types become required), `apps/battle-lab/src/routes/airWar.tsx`, `web/scenes/air-roster.mjs`, and 9 roster entries.
- **The frozen file `fixtures/parity/map-layout/physical-rules.json`** is edited by hand and never regenerated (`fixtures/README.md`). Change its forest fields, the spotting rename and the required air bands. Map generation doesn't read them, so its pinned hashes stay.
- **Shouts and rounds** keep passing concealment 1 (`shouts.rs`, `side_sees_round`).
- **The menu reel** uses its own units (`fixtures/units/capture/`), built on the test units. Change any of their stats or scenario to get the scene; never narrow the mechanic for the reel.
- **Tests that change on purpose:** `contract/tests/catalog.rs` (spotting can't reduce sight, emitting radar is high-air sight, unaided by body), and `sim/tests/{sensing,forest,rivers,air,radar,distances_3d}.rs`.
- **Parity records slice 2 moves:** `fixtures/parity/exact-numbers.json`, `skirmish-objectives.json`, `skirmish-purchases.json` (`BLESS_PARITY`).
- **Docs:** `crates/sim/README.md` (sensing and high air) and `fixtures/README.md` (high-air fallback, forest rule).
- **Checked and clean:** the fog sweep never reads concealment. Map analysis, map generation, the presentation fog and the skirmish AI read only sight range or foliage, so the formula has no second copy.

## Confirm before coding

- Every concrete unit resolves `concealment` through `extends`. The 13 roster units with no `extends` (drones and others) and every test base author it directly.
- The MBTs are exactly the units that inherit the five `*_mbt` profiles.

## Out of scope

- Aircraft and radar stealth. This spec covers visual spotting only.
- Firing reveals only the rough contact patch (`knowledge.rs`). That patch can be fired on, by order or automatically, and can kill the shooter, so firing gets no concealment penalty.
- Balance. Retune with the report afterwards.
