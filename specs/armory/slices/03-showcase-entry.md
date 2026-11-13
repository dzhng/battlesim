# 03 · Showcase entry point: idle life in the presentation crate

Map rows: Q1′, Q6, D1, L11, and the sound half of L9 (P5). Depends on 02 for
the shape of the squad input; the code can start in parallel.

**This slice is the plan's kill switch.** If the presentation crate can't pose
a unit without a simulation publication, stop and report to the user. Never
copy posing, rotor discs or sound resolution into TypeScript. That rule is the
map's "Confirm before coding" item 1.

## Contract

K4. The presentation crate poses one unit with its idle life and no
publication:
- each soldier's own tempo, phase and facing, plus the occasional watcher;
- turning, blurred rotors on a grounded rotorcraft;
- the faction looks `Fielding` resolved;
- operators carrying their weapon's **carried** kit.

It also names the unit's idle loops, with the battle's sound replacements
already applied.

## Seam

New file `crates/presentation/src/presentation/showcase.rs`, beside `lab.rs`:

```rust
#[derive(Deserialize)] #[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ShowcaseInput {
    pub kind: String, pub side: String, pub at: [f64; 3], pub yaw: f64,
    pub soldiers: Vec<ShowcaseSoldier>,          // from SquadAtRest, offset to the spot
    pub operators: Vec<Option<u32>>,             // from SquadAtRest
}
pub struct ShowcaseSoldier { pub id: u32, pub slot: usize, pub at: [f64; 2] }
impl Presentation {
    pub fn set_showcase(&mut self, fielded: Fielded, input: ShowcaseInput) -> Result<(), String>;
    pub fn clear_showcase(&mut self);
}
// presentation::sound
pub fn idle_loops(tables: &SoundTables, vehicle_class: Option<&str>) -> Vec<IdleLoop>;
pub struct IdleLoop { pub slot: &'static str, pub sound: String, pub gain: f64, pub rate: f64 }
```

**How the showcase works:**
- `Showcase` owns one unit's `PoseDriver` and `Drawn`.
- Each frame it builds a `FeedFrame` / `FeedUnit` (`units/driver.rs:64,94`)
  holding the one stationary unit, instead of reading an `ObservationFeed`.
  Soldiers enter on first sight with the driver's own `rest_manner` and
  `loop_start` (`driver.rs:147-160`), so their idle feel is the driver's.
- **A parked rotorcraft turns its rotors with blur.** Today the driver sets
  `rotor_blur = airborne ? 1 : 0` and advances `rotor` by `feel.rotor.tip_mps`
  (`driver.rs:850-870`).
  - The showcase states the unit **level at z = 0 with its rotors turning**.
    "Rotors turning" is a property of the feed unit, not a role check.
  - The driver blurs whenever rotors turn. Battle frames can't change, because
    in battle the sim's rotors are always airborne, so the two conditions
    coincide.
- **Rotor discs have one owner.** Today `effects.build(…, &rotors, …)` draws
  them (`presentation.rs:1047`). Factor the rotor-disc emission into one
  function that `Loaded::frame` and `Showcase` both call. Never duplicate it.
- **Looks and rows.**
  - `Fielded` is the same type the battle's `load_world` uses (`fielded.rs:30`).
    The browser resolves it with `Fielding` and `factions: [tab, tab]` (L11).
  - Rows are appended after `Lab::append` (`presentation.rs:969`).
- **Idle loops.** `idle_loops` is `vehicle_loops(row, 0, 0, false)`
  (`sound/frame.rs:543`) with `SoundTables::effect` (`sound/tables.rs:396`)
  applied, which is exactly what a stationary battle vehicle starts. Squads have
  no `vehicle_class` and return `[]`.

**The wasm side**, new file `crates/presentation-wasm/src/showcase.rs`:
- `set_showcase(&mut self, fielded: &str, input: &str) -> Result<(), JsError>`;
- `clear_showcase(&mut self)`;
- the free function `idle_loops(setup: &str, kind: &str) -> Result<String, JsError>`.

## The human runs or sees

Nothing visual; slice 04 shows it. Run
`cargo test -p presentation --test showcase`.

## Verification

Write these first, in `crates/presentation/tests/showcase.rs`:
- `a_showcase_poses_without_any_publication`: there is no `push_publication`
  call, and the models are in the frame.
- `showcased_soldiers_idle_each_at_their_own_tempo`: palettes change over time
  and differ between soldiers. No values are pinned.
- `a_parked_rotorcraft_turns_and_blurs_its_rotors_on_the_ground`: a drawn rotor
  disc is emitted, the rotor travel grows, and the model's origin is at z = 0.
- `an_operator_at_rest_carries_his_weapons_carried_kit`: resolves to
  `operated[m].carried`.
- `a_showcase_wears_the_look_fielding_resolved_for_its_faction`.
- `clearing_the_showcase_leaves_lab_and_battle_rows_untouched`.

In `crates/presentation/tests/sound.rs`:
`an_idle_vehicle_sounds_as_a_stationary_battle_vehicle_does`. It compares
`idle_loops` with the start commands a `Sound` emits for a stopped vehicle.

## Delegated to the implementer

- The internal struct layout.
- Whether `Showcase` reuses parts of `Loaded`.
- The exact JSON of `idle_loops`' setup, as long as it is a subset of
  `SoundFrame`'s setup.

## Must stay green

- Every test in `crates/presentation/tests/*`, especially `lab_models.rs`,
  `poses.rs`, `vehicle_poses.rs` and `transit.rs`.
- `records_are_current`.
- Browser publication parity.
- `fixtures/parity/publication/**`, unchanged.

## Feedback that would change it

None is expected here; slice 04 is where idle life is judged.

## Reslice if too big

Split it three ways, each with its own tests:
1. soldiers (the driver feed);
2. vehicles and rotors (`Drawn` and rotor discs);
3. `idle_loops`.

Escalate rather than reslice if the driver needs publication-only state (a
`FrameSample` field the feed can't synthesise).
