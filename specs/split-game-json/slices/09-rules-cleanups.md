# 09 Rules cleanups

**Contract:** the rules hold only what the battle reads, each number authored
once. **No battle outcome changes.**

This is one slice because every change in it moves `rules_hash`, and the
stored records are rewritten once. Land it as one commit per change below,
each with its own digest check.

**Changes:**

1. **Catalog limits leave the rules.**
   - `contract::HullLimits::SHIPPED` and the air-speed equivalent become
     constants that catalog admission takes as an argument. They're not
     hashed and not overridable.
   - The limit check runs in `sim::admit` ([07](07-admission.md)), because
     serde's `TryFrom` takes no arguments.
   - `lift_hull_limits` and `with_units_at_limits` (about 13 uses) pass lifted
     limits instead of editing the rules.
   - The helicopter routing margin becomes its own 10 m constant in
     `sim/navigation/air.rs`, replacing `routing_margin(rules)`. Today's value
     is 10.0, so no route moves. Its doc records why it is fixed: `1f521bb78`
     and `70b6de62b` flip-flopped it.
2. **Forest floor placement is deleted.**
   - Goes: `navigation/floor.rs`, `place_forest_bodies`, the four `forests`
     rule fields, `forests.log`/`forests.boulder`, and their checks and tests.
   - Kept: the `log`/`boulder` prop rows, their models, their use as field
     cover, `forest_floor.py` and `trunk_positions`.
3. **`range_m` is derived** from the weapon's largest layer range.
   - The parser refuses a `range_m` that differs from the derived value, so
     resolved rules still round-trip through `Rules::overridden` and the wasm
     handles.
   - Shipped data carries none. Resolved rows emit it for the TS readers.
   - The three reel encounters that override it drop the override.
   - The mechanics editor's field (`fields.ts:290-310`) goes.
4. **`service.round_costs` becomes each weapon row's `round_cost`.**
   - Its 45 entries move; children inherit.
   - `air_gun` has unlimited ammunition, so it gets no cost.
   - The supply check becomes a row check.
5. **Smaller changes:**
   - `decoyable` is dropped where no target carries flares;
   - `buildings.cover_strength` is deleted;
   - `capacity_soldiers` over 32 is refused (the shipped maximum is 32);
   - `shouts.talk_s` becomes one number.
6. **Gravity:**
   - one gravity constant (`cook_off.rs:19`, `units/driver.rs:32`);
   - `gravity_scale`'s doc (`contract/src/ballistics.rs:92`) and the test
     `a_gun_round_is_slowed_for_the_eye_but_flies_the_real_round_s_line`
     say what it is: the arc chosen for the screen. The values don't change.
7. **Appearance data moves:**
   - stand-in tints go into each prop's `appearance` block, authored for all
     59 props with no fallback; the 22 that relied on the fallback get its
     colour explicitly;
   - track gauge goes into the model's appearance, beside `recoil_max_m`.

**Run / see:** battles play the same. A bundle with an authored `range_m`
that disagrees with its layers is refused.

**Verify:**
- Write first: an authored disagreeing `range_m` is refused, and a resolved
  document round-trips.
- Battle digests, `menu_reel.rs` event logs and 01's probe are unchanged.
- A helicopter route comparison is unchanged.
- Tests: `sim/tests/{air,air_flight,catalog,flight_ballistics,maps}.rs`,
  `contract/tests/{rules,weapons}.rs`,
  `mapgen/tests/{generation_physics,compiler,layout_cli}.rs`.
- A quick balance sample, before against after.

**Moves:**
- `rules_hash` and replay config digests;
- the map generator's `config_hash` and its 17 layout hashes, and the
  `config_hash` the saved generated maps pin;
- `catalog.json` text.

No battle digest and no physical map changes. One script,
`throwaway/split-proof/rewrite-rules-shape.mjs`, migrates:
- `skirmish-objectives.json` and `skirmish-purchases.json`. Migrate their
  frozen values in place; never replace their rules with today's shipped
  rules, because both still name `assault_rifle`, `rpg_light` and
  `rpg_heavy`;
- `exact-numbers.json`;
- `scenario-overrides.json`, which overrides `service.round_costs`;
- `parity/map-layout/physical-rules.json`.

Then re-bless with `BLESS_PARITY` and `UPDATE_RECORDS`, and rerun unflagged.

**Depends on:** [07](07-admission.md). Serialize the record rewrite with
[08](08-looks.md).

**Delegated:**
- the constant module for the limits;
- the order of the commits;
- the authored/resolved type factoring for `range_m`.

**Would change this slice:** any battle digest moving. Stop and find the
cause; it is a bug in the move.

## Done — 2026-10-29

All seven changes landed without a battle-outcome change. The final slice-01
probe matches all 77 manifest files byte for byte, including all 69 composed
shaders. All six native plates are byte-identical, and native/browser population
counts pass. The two-seed balance sample retains both battle digests; helicopter
routing, menu-reel event logs, poses, sound and effects remain unchanged.

The prerequisite admission regression was fixed first by completing the frozen
generation rules, preserving their physical values and output identities. Full
rules admission remains the one entry contract; a second physical-only admission
reader would duplicate it. Frozen rule shapes were migrated in place with
`/Users/server/dev/battlesim/throwaway/split-game-json/scripts/rewrite-rules-shape.mjs`;
only the named catalog/parity/record owners were re-blessed, then checked
unflagged. Seven successful generation cases move their config-dependent output
hashes; the other ten corpus cases stay unchanged.

Decisions: `contract::limits` owns explicit hull and air-speed bounds; resolved
weapons keep their computed range, and encounter patches strip computed ranges
before re-resolution. Per the user's decision, every ground-hull appearance owns
`track_gauge` in the source asset catalog and its runtime catalog, with missing
gauges refused (including faction looks); model binaries were not rebaked.
Stand-in colours are required prop appearance data, including explicit former
default colours.

Correction: the two saved generated maps retain their historical layout-16/presets-18 `config_hash` pins; these are provenance, and no reader compares them against current rules.

Focused contract, simulation, map-generation, browser, presentation and shader
ABI checks pass, as do the Wasm build, typecheck, formatting, lint, clippy and
source-size ratchet. The broader simulation run completed its stress test and
found seven stale fixture/reader assertions; all seven were corrected and rerun
successfully in focused suites. The full repository run remains slice 16's gate.
An unrelated stale admission CLI error-path assertion was corrected separately.
