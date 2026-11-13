# Armory: choices

Decisions the implementing agents made where the spec was silent. Each gives
the choice, why, the alternative, and a verdict (sound / provisional /
needs-user).

## Slice 02

- **The flat map's owner is `MapDefinition::flat(size)` in `contract`;
  `WorldGeometry::flat(size, rules)` builds on it.** Why: both test helpers
  take extra map fields (props, forests, buildings), so what they duplicated
  is the map, not the geometry. The helper in `sim/tests/common` merges its
  extra fields over `MapDefinition::flat`, and `world_geometry.rs`'s copy
  calls that helper. Alternative: `WorldGeometry::flat` alone, leaving the
  helpers' JSON literal as a second owner of the defaults. *Sound.*
- **Only the two named copies moved.** About 30 other test files write the
  same `fog_cell_m: 8, height_grid_m: 4, slope_cutoff_deg: 35` literal inside
  whole scenario JSON (maps with props, roads, encounters). Folding those in
  is a test-wide refactor outside this slice. *Provisional:* a later
  test-cleanup pass can point them at `MapDefinition::flat`.
- **`original_carriers` lives in `weapons/operators.rs`** beside
  `assign_operators`, which moved there out of the over-budget `weapons.rs`
  in its own commit first. It takes each mount's spec (`None` for a gun not
  wanted, such as an empty one) and each member's slot (`None` once fallen),
  and returns member indices. `assign_operators` keeps its
  handoff and spare passes. Alternative: the spec's `(specs, slots)` with the
  ammo filter left in `assign_operators`, which would have needed a second
  filtered copy of the specs. *Sound.*
- **The showground's ground is 400 m square and the squad stands about its
  middle,** reported as offsets from it, so `SquadAtRest` is about (0, 0).
  Soldier ids are `slot + 1` and the arrangement draws as battle unit 0. Why:
  the largest squad spread fits with room to spare; flat ground scatters the
  same anywhere, so the test compares against `squad_spots` on its own flat
  world within 1e-9. *Sound.*
- **`Showground` keeps Rust-typed `admit` and `at_rest` beside the wasm
  `new` and `squad`.** Why: `JsError` cannot be built natively, so a native
  test of a refusal needs the `String` error. *Sound.*
