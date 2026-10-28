# 04 The concealment mechanism, neutral

**Unlocks:** the whole contract in the README is coded: unit concealment, spotting in both directions, moving, and the new HIDDEN rule. Values are neutral (every hull at 2.8, infantry at 1, moving at 1.0, no spotting below 1), so no battle changes. Slice 06 then only changes data.

## Seam

- **Contract**
  - `UnitType.concealment: f64`, required. A plain `f64` makes it required in the resolved type, because abstract entries are dropped before parsing. Validate finite and > 0.
  - `Sensors.spotting: f64`, defaulting to 1 and left out when 1. It replaces `concealed_range_multiplier`, and the `>= 1` check (`catalog.rs:1730`) goes. Validate finite and > 0.
  - `ForestRule.concealment` replaces the class pair; validate (0, 1] in `ForestRules::check`.
  - `SensorRules.moving_concealment: f64`, required, ≥ 1.
- **Owners, in `crates/sim/src/sensing.rs`:**
  ```rust
  pub fn ground(forest: f64, shelter: f64, s: &SensorRules) -> f64          // min(forest, 1 + (building − 1) × shelter)
  pub fn target(unit: f64, ground: f64, moving: bool, s: &SensorRules) -> f64
  pub fn effective(target: f64, spotting: f64) -> f64                        // both branches capped at 1; libm::pow
  fn sample(world: &WorldGeometry, u: &Unit, at: V3, rules: &Rules) -> (f64, f64)  // (ground, target); airborne → (1, 1) first
  ```
  - `evaluate` calls `sample` once per target sample and `effective` once per observer. It must compute `unit × ground` before multiplying by moving, and moving before spotting. Then the neutral `2.8 × 0.25 × 1.0 × 1.5` reproduces today's `0.7 × 1.5` bit for bit.
  - `concealed` (HIDDEN) becomes "enough samples with `ground < 1 && target < 1`", using the same `sample`. Today hulls take a separate path (`sensing.rs:193`), which goes. Keep HIDDEN's other guards at `battle.rs:4394`: being seen by an enemy and being engaged both suppress it.
- **The cap keeps the observer prefilter valid.** `evaluate` skips targets beyond full sight (`sensing.rs:250`), which is only correct while `effective ≤ 1`.
- **Fold** `sensing::validate` into `SensorRules::check` and delete the panic at `battle.rs:638`.
- **`Unit::moving()`** in `units.rs` reads `state == MoveState::Moving`. Switch `hearing.rs:68` to it. Movement finishes before sensing runs in the tick (`battle.rs:1636`). The existing one-tick stagger in sensing (`SENSE_EVERY`) is acceptable.
- **Forest:** `forest_concealment(x, y)` drops its `infantry` argument (`world/forest.rs:360`); update its callers in `rivers.rs`, `forest.rs` and the sensing tests.
- **Editor:** add `concealment` and `sensors.spotting` fields with explanations (`fields.ts`, which must net-shrink; split it by concept if needed). Rebuild the validator with `bun run build:mechanics`.

## Neutral data

| What | Value |
|---|---|
| Every hull, including aircraft and the test and capture hulls | 2.8 |
| Squads (on profile roots and test bases) | 1 |
| Forest | 0.25 |
| `moving_concealment` | 1.0 |
| Spotting | renamed, values unchanged |
| Planned cards | not touched |

Hand-edit `physical-rules.json` (`concealment` on its units, the forest field, the spotting rename).

## Verification

- **Red first, `effective` unit test.** The formula is a small piece of logic where a unit test pays. Cover both caps, that spotting below 1 never affects an unconcealed target, and that spotting below 1 deepens concealment.
- **Red first, system test.** Create `crates/sim/tests/concealment.rs` with `concealment_controls_identification_and_hidden`. It authors its own values (sniper 0.5, MBT spotting 0.8, moving 1.5), so it tests mechanics, not shipped data. Assertions are relational, never exact metres:
  - a hidden team in the open is identified closer than an ordinary squad;
  - scouts reach it farther, but never past their sight;
  - an MBT with spotting below 1 needs to be closer;
  - a moving unit is identified farther than a still one, and a moving tank is never harder to see than a parked one;
  - the forest product applies, and so does garrison shelter;
  - an aircraft is unaffected;
  - HIDDEN holds while still in forest and is lost by an MBT driving through forest;
  - the result is the same on replay.
  
  Delete any test in `sim/tests/sensing.rs` that this one supersedes.
- **Contract test:** a physical unit that resolves without `concealment` is refused.
- **Tests that change on purpose:** `contract/tests/catalog.rs:209` (spotting below 1 is now allowed), `sensing.rs:149` (rename only; it must stay green otherwise), `sensing.rs:585`, `forest.rs:70-94`, `rivers.rs:271`, `distances_3d.rs`.
- Every battle digest is identical, and HIDDEN in the publication is unchanged ([proof](../README.md#proof-that-nothing-changed)).

## Delegated to you

Internal helper names beyond the four above. Whether `sample` returns a small struct or a tuple.

## Must stay green

Every digest and publication value; the editor's preview and save of inherited `concealment`.
