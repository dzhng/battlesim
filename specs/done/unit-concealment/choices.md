# Decisions ledger

These decisions were made where the plan was silent or wrong. The [README](README.md) holds the design rationale settled with the user; this file holds what was decided during the build.

## Determinism guard

- **The libm guard covers library code, not the native-only tests and examples of the four crates that opt out.** The plan included test code, which would have meant rewriting 176 calls in code that never runs in WebAssembly. Contract, sim, mapgen and presentation each set `disallowed_methods = "allow"` in `Cargo.toml` and `#![deny(clippy::disallowed_methods)]` in `lib.rs`. Every other crate is guarded everywhere, because clippy runs with `-D warnings`. Per-file allow attributes were tried first; the size ratchet refused them in oversized test files.
- **`powi` stays allowed.** It isn't transcendental.
- **The presentation crate had two production violations** (`scars/cache.rs` `log2`, `sound/bank.rs` `log10`). They were converted. The f32 list matches the f64 list.

## Strict rules

- **The rules type declares the three sections that aren't simulation rules.** `UncheckedRules` admits `presentation` (parsed by presentation), `controls` (owned by the browser's `reverseZone.ts`) and `seed` (the battle lab's default). Stripping them in each reader would have meant more places to keep in step.
- **`status`, top-level `units` and `physics.blast_falloff` were deleted.** They were prose annotations nothing read.
- **Two callers embedded `map` inside the rules** (`settlement_tests.rs`, `examples/ground_resources.rs`). Strictness found them, and they now pass the map separately.
- **`mechanics_validate`'s tests run with the workspace suite** (`[[example]] test = true`). Before, `cargo test --workspace` built them but never ran them.

## Fallbacks become data

- **Shared values live on the existing profiles, with child overrides only where a resolved value differed.**
  - Each weapon base row carries `partial_from` and its layer fire; the inheritance-trap rows (children overriding `range_m` but inheriting `targets`) author their own former ground reach.
  - Each ground profile authors `low_air_m` equal to its `ground_m`, and `high_air_m` from the old body table: 600 on foot and for aircraft, 300 for ground vehicles. Profiles that already authored their own (the MBTs' and the wheeled tank destroyer's 0) keep it.
  - Each former emitter carries its old `detection_m` as `high_air_m`.
  - The proof: before/after dumps of every resolved weapon's per-layer range, spread and `partial_from`, every aircraft's layer and afterburner, and every unit's per-band reach and radar signature were byte-identical for the game, test and capture sets.
- **The row-level `scatter_mrad` was deleted**, and the accuracy ceiling holds per layer (`FlightConfig::check_layer`). The duplicate nonnegative-spread check went, because contract admission already refuses a negative layer spread.
- **The editor's range-to-spread coupling was deleted, not ported to each layer.**
  - Re-enabling a layer restores its admitted values.
  - The server's intent check lets a later child edit (a layer's range) supersede that field inside an earlier parent replacement (enabling the layer).
  - Revisit if tuning by landing metres is missed.
- **`capabilities.deploy.pack_seconds` is required too.** It was the one remaining copy (packing fell back to setup time); every deploying unit already authored it, so digests didn't move.
- **The editor seeds a newly enabled layer at the row's flight bound**, the one reach admission always accepts (zero on an unpowered row), instead of a fixed 300/100/500 m.
- **Every movement slowdown became a speed factor, set per locomotion.** Asked after the close (2026-10-29): `movement.speed_factors.{foot,wheeled,tracked}` holds each kind of ground's factor (`surfaces`, `forest`, `swamp`, `crater`, `slope`), replacing the rules' `surfaces` table, the forest infantry/vehicle multipliers, staging's `movement.swamp`, `ground.crater_vehicle_mult` and the hard-coded slope formula; `reverse_fraction`, `turn_slow` and the afterburner multipliers were renamed `*_factor` to match. Every value was copied, so no digest moved. The user chose per-locomotion road surfaces too, which makes the route planner's per-cell and per-road data carry surface kinds instead of a factor.
- **Tests that set a weapon's reach go through `crates/sim/tests/common` helpers.** One sets the flight bound and the ground reach together, so oversized test files don't grow.
- **The fallback deletions landed as one commit, not one per fallback.** They share files, fixtures and re-blessing.

## The mechanism

- **HIDDEN runs over the same samples for hulls and squads.** A hull's two samples share a ground point, so its answer didn't change.
- **`sensing::validate` folded into `SensorRules::check`**, which also admits `moving_concealment ≥ 1` and `building_range_multiplier` in (0, 1]. `ForestRules::check` bounds forest `concealment` to (0, 1].
- **Clearing Spotting in the editor writes the neutral 1**, and the editor reads an omitted value as 1. A nullable reset was rejected because admission refuses null.
- **The system test authors its own values** and sweeps reach coarsely, relationally, against live battle samples. It never pins exact metres.

## Tools

- **The spotting report takes no flags.** Heat shades the fraction of full front sight, and each group lists its unit names and ids. Only groups containing a squad get a garrisoned row. Ground-blind observers (fast jets author 1 m of ground sight) stay as their own column.
- **`concealment_shots` is its own example, sharing `examples/common/canvas.rs` with `movement_shots`.** Adding a scene to the movement table was rejected: sensing transitions need their own checkpoints. `movement_shots` output stayed byte-identical after the canvas moved.

## Starting values and recordings

- **All 159 physical units across the game, test and capture sets match the table:**
  - hulls (129): 10 at 1.8, 34 at 2.4, 85 at 2.8;
  - squads (30): 22 at 1, 6 at 0.8, 2 at 0.5.

  The game catalog alone has 139: 117 hulls (5, 30, 82) and 22 squads (16, 4, 2).

  A throwaway script wrote literal values, and a second run changed nothing.
- **The test and capture scouts are scouts at 0.8**, like the roster's.
- **Frozen skirmish inputs got only the changed fields.** Replacing their whole rulesets would have lost unrelated frozen values.
- **The reel was judged, not tuned.**
  - Every Market Town and Paris Corner shot was compared before and after.
  - The unprimed critique's framing findings (an empty field shot, a roof-dominated Paris shot, a blast filling a frame) appear identically before the change.
  - The followed tank still dies at tick 1249.
- **The GIF checkpoint wasn't opened in Preview.** The user asked on 2026-10-28 not to be asked to approve anything.

## Closeout

- **The full run:** `bun run check` passed format, lint, wasm, typecheck, 1,957 Rust tests and 1,310 web tests. Four web tests needed LFS assets missing from the checkout: third-party audio, disabled-card source models and the launcher skeleton. With those pulled, they passed.
- **The balance sweep:** `battle_sweep` (10 seeds, 30 s) completed 114 battles. Its 6 refusals are map generation's own (transit, sight certificate, settlement placement), and none read this spec's fields.
- **Net code:**

  | Area | Added | Deleted | Net |
  |---|---:|---:|---:|
  | Library (Rust and TS, no tests or examples) | 975 | 1,030 | −55 |
  | Examples | 1,005 | 272 | +733 |
  | Tests (code, no recorded fixtures) | 1,147 | 924 | +223 |
  | Docs | 78 | 23 | +55 |
  | Authored data | 569 | 459 | +110 |

  This is an accepted deviation from "net less code": the library shrank, and the growth is the two tools and the system test the spec asked for.
- **Superseded records were repointed.** `specs/done/air-defence-radar/README.md` and `specs/done/air-war-flight/README.md` now describe the radar signature and authored sight. Decisions D47 and D48 in `specs/done/air-war-flight/flight-model-map.md` are marked superseded.
- **Prop `blocks` refuses unknown keys too.** The audit found it was the one rules-reached struct without `deny_unknown_fields`. Every catalog still loads.
- **The starting-values script left 68 misindented `"concealment"` lines** in the roster JSON. Nothing formats `fixtures/`, so they were reindented by hand, with values unchanged.
