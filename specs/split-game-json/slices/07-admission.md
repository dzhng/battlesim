# 07 One admission owner

**Contract:** bad rules are refused with a named error at every place rules
arrive from outside. No path reaches a panic.

**Seam:** `sim::admit(&Rules) -> Result<(), RulesError>`.

`RulesError` names the path, for example
`weapons.rpg_at_light.targets.ground.range_m`.

**What it runs:**
- the contract's section checks;
- `sim::weapons::check_rules` (the flight checks);
- the supply check;
- every range `Battle::from_prepared` asserts today: `cover.rs:271`,
  `damage.rs:66`, `ground.rs:104`, `units/motion.rs:107`,
  `flight/mod.rs:367`, `supply.rs:116`. The pure ones become contract
  section `check()`s, following the pattern at `scenario.rs:731-742`;
- `admit_world`'s rules checks;
- the missing ranges:
  - `infantry_movement`: `wander_period_s: 0` gives NaN positions;
  - `bail_out`: a negative `hp_lost` heals;
  - garrison, pushing, launch, and the pace caps;
- a guided missile's lifetime covers its range;
- `helicopter_crash` is present when the catalog can field an aircraft hull.

**Callers:**
- `fixtures::admit`;
- encounter overrides: `sim::maps` and wasm `encounter_scenario`;
- uploaded bundles: `check_scenario_bundle`, after the contract's own
  validation;
- the wasm battle and world handles: `BattleHandle::new`/`from_replay`,
  `PreparedWorld::*`;
- `mapgen`'s rules reader. `mapgen` takes `sim` as a normal dependency;
- native loaders.

`Battle::new` keeps its signature. Its asserts become `debug_assert!`s of
invariants these paths guarantee. `Arsenal::new`'s `.expect`
(`weapons.rs:119`) becomes one of those invariants.

**Run / see:** upload a bundle with a bad weapon row. It is refused with the
row's path instead of a panic.

**Verify:**
- Write first: `crates/game-wasm/tests/admission.rs` (or wherever it reaches
  every entry). It drives one bad weapon row and one `wander_period_s: 0`
  through every entry path and expects refusal, not a trap or a partial
  handle.
- Accepted rules keep every battle digest; 01's probe is unchanged.
- `contract/tests/{rules,weapons}.rs`, `sim/tests/catalog.rs`,
  `game-wasm --test bundle` and `sim/tests/maps.rs` stay green.

**Moves:** `engine_build` only. Valid rules and outcomes are unchanged.

**Depends on:** [04](04-split-files.md).

**Delegated:**
- `RulesError`'s shape;
- where each range check lives;
- whether `admit` returns the `FlightConfig` it builds, so `Arsenal::new`
  stops rebuilding it.

**Would change this slice:** an entry path that can't reach `sim`. Name it
here and ask; don't add a second admission owner.

The burst-cadence check is not here: the simulation never sees recordings. It
lands at fielding in [15](15-content.md).

**Done (2026-10-29):** `sim::admit` is the external rules boundary. Contract
section checks return a shared `RulesError` (relative path and message); simulation
adds weapon flight, supply, and forest-placement admission. Fixture, encounter,
upload, WASM world/battle, generation, and native rules readers use it. The
constructor-only range validators are deleted; constructors retain invariants.

Proof: all 77 unrounded probe/shader files and all six native plates are
byte-identical to slice 01's controls in shared scratch (`after-07/`); native draw
counts also match every browser plate. No stored record, rules identity, battle
digest, or presentation output changed. Only `engine_build` may move. Contract,
focused native/WASM admission and encounter tests, generation-physics tests,
clippy, typecheck, lint/format, and the size ratchet pass. The broader sim run
passed 1,151 tests; its two assertions for superseded panic/error wording were
updated to admission refusals and passed in focused reruns. Full repository
closeout remains at 16.

Decisions:
- **User clarification:** require `helicopter_crash` only when the catalog can
  field an aircraft hull. Ground-only rules do not need an aircraft crash row;
  stored-record rewrites remain in 09.
- Keep `admit`'s `Result<(), RulesError>` seam. `Arsenal` builds its flight
  configuration from admitted rules rather than threading a cached configuration
  through every caller.
- Keep pure section checks and their error type in contract so deserialization
  and simulation reuse one owner; do not duplicate the ranges in each loader.
- Check guided lifetime against the existing motor-distance calculation (or
  constant-speed distance without a motor), rather than adding another flight
  formula.

**Review follow-ups (closed):**

- Mapgen resolves through `sim::admit` before extracting generation inputs; the earlier reader regression was fixed by 09 (confirmed).
- `ForestRules::check` rejects non-finite/non-positive trunk spacing; Rust admission and WASM world/battle entry tests cover refusal (0308b96ee).
- The workbench shared checker, tournament inputs, city report and encounter report call `sim::admit` before constructing worlds (8911f21f7).
- `CoverRules::check` admits finite building spread >= 1 and fragment probability in [0, 1], with refusal tests (0308b96ee).
- `ForestRules::check_placement` now checks only catalog placement; 09 already removed the repeated numeric check (confirmed).
- `admit_world` checks map geometry and map/catalog relationships without repeating `Rules::check` (8fa8d29dd).
