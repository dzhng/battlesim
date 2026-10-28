# 00 Every transcendental function is libm's

**Unlocks:** native and WebAssembly builds compute the same simulation and map generation, by construction. `crates/presentation` already holds this rule ("every transcendental function is `libm`'s", `crates/presentation/README.md`), but the simulation, contract and mapgen crates still call the standard library's versions, whose results can differ by platform. Slice 04 adds `libm::pow`, so the rule must hold before it.

This slice is first and separate because it **may** move native digests. Every slice after it is proven against the baseline it leaves.

## Seam

Replace each standard-library call with its `libm` equivalent. All 13 sites as of 2026-10-28:

| File | Calls |
|---|---|
| `crates/sim/src/sensing.rs:105` | `exp` (`foliage_reach`) |
| `crates/sim/src/sensing.rs:266` | `asin` |
| `crates/sim/src/navigation.rs:825` | `hypot` |
| `crates/sim/src/map_analysis.rs:51` | `cos`, `sin` |
| `crates/sim/src/flight/mod.rs:251` | `tan` |
| `crates/sim/src/flight/mod.rs:1524` | `acos` |
| `crates/sim/src/world/terrain.rs:474` | `tan` |
| `crates/sim/src/world/mod.rs:525, 533` | `acos` |
| `crates/contract/src/map.rs:223` | `tan` |
| `crates/mapgen/src/joints.rs:1629, 1776-1777` | `cos`, `sin` |

Test code follows the same rule (e.g. `skirmish_ai/tests/backline.rs:128`, `hypot`). `sqrt`, `to_radians` and `to_degrees` are exactly rounded or plain multiplication, so they stay.

**Guard, so the rule can't regress:** a `crates/clippy.toml` with `disallowed-methods` covering the `f64`/`f32` transcendental methods (`exp`, `exp2`, `ln`, `log2`, `log10`, `powf`, `sin`, `cos`, `tan`, `sin_cos`, `asin`, `acos`, `atan`, `atan2`, `sinh`, `cosh`, `tanh`, `hypot`, `cbrt`). Clippy finds the file from each crate's directory upward. That covers every crate under `crates/` (the presentation crate already complies) and leaves the native client in `native/` alone. The pre-commit hook already runs `cargo clippy --workspace --all-targets -D warnings`.

**Docs:** state the rule once, where the simulation's determinism is described (`crates/sim/README.md`). Point `crates/presentation/README.md`'s sentence at it rather than repeating it.

## Delegated to you

Where `clippy.toml` sits if `crates/` doesn't scope it as expected (prove it with one deliberate violation, then remove it). The exact list of disallowed methods.

## Verification

- **Red first:** add a deliberate standard-library `exp` call. Clippy must fail with the disallowed-method message. Then remove it.
- Run the simulation parity records and the menu reel.
  - **If a digest row moves:** native was computing something different from the browser. Re-record with `BLESS_PARITY` and the reel, name the call that moved it in the commit, and confirm the browser parity tests (`exactNumbers`, `skirmishObjectives`, `skirmishPurchases`) pass against the new records.
  - **If nothing moves:** say so in the commit.
- Mapgen: `map_hash` may move through `contract/src/map.rs` and `joints.rs`. If it does, re-bless the map-layout records and confirm `web/tests/mapLayout.test.ts` passes. If not, confirm it didn't.

## Must stay green

The whole workspace, clippy included. After this slice, the browser and native parity tests agree.
