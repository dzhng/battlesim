# 02 · Showground: the squad at rest, from the simulation

Status: done 2026-11-13, commit bd5420dd0.

Map rows: Q1′, D1. No dependencies.

## Contract

K3. The browser gets the game's own squad scatter, and the original operator of
each special weapon, from the simulation, on flat ground. No TypeScript copy of
either exists.

## Seam

**New file `crates/game-wasm/src/showground.rs`:**

```rust
#[wasm_bindgen] pub struct Showground { world: WorldGeometry, rules: Rules }
#[wasm_bindgen] impl Showground {
    #[wasm_bindgen(constructor)]
    pub fn new(rules_json: &str) -> Result<Showground, JsError>; // admits once, flat world
    /// The squad of `kind` at rest about (0, 0), as a fresh battle squad stands.
    pub fn squad(&self, kind: &str, seed: u32) -> Result<String, JsError>; // SquadAtRest
}
// SquadAtRest = { soldiers: [{ id: u32, slot: usize, at: [f64; 2] }],
//                 operators: Vec<Option<u32>> /* per mount: a soldier id */ }
```

**The owners stay in `sim`:**
- **The scatter.** `sim::arrangement::squad_spots` (`arrangement.rs:215`) and
  `rng` (`:27`) are unchanged. `Showground` calls them exactly the way spawn
  does (`battle/spawn.rs:26-44`).
- **The flat world.** `WorldGeometry::flat(size_m, rules)` is promoted out of
  the test helper `crates/sim/tests/common/mod.rs:147`. That helper and the
  copy at `crates/sim/tests/world_geometry.rs:15` become calls to it.
- **The original carriers.** The first pass of `assign_operators` (`weapons.rs:1903`, "living original carriers get their own weapon") is extracted as `pub fn original_carriers(specs, slots) -> Vec<Option<usize>>`. `assign_operators` calls it, so battle digests can't move.
- **Hulls.** A kind with a hull is refused with an error.

## The human runs or sees

Nothing visual. Run `cargo test -p game-wasm --test showground`.

## Verification

Write these first, in `crates/game-wasm/tests/showground.rs`:
- **`a_squad_at_rest_stands_as_the_battle_scatters_it`.** It must equal
  `squad_spots` on `WorldGeometry::flat` with the same `rng`, over many seeds.
  It compares against the owner, never against numbers.
- **`every_special_weapon_starts_with_its_original_carrier`.** Generative over
  the game catalog: each non-squad mount's operator has a slot in its
  `carriers`, and squad guns have none.
- **`a_kind_with_a_hull_is_refused`.**

## Delegated to the implementer

- The size of the flat world.
- Function names inside `sim`.

## Must stay green

- `crates/sim/tests/soldier_bodies.rs`.
- `crates/sim/tests/world_geometry.rs`.
- Every sim replay and digest test, run with **no bless**.

## Feedback that would change it

None is expected; this slice isn't visible.
