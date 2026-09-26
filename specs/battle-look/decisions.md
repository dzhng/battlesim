# Decisions

The settled interview is [`unknowns-map.html`](unknowns-map.html): the look board, the 24-row decision ledger and 21 landmine cards. This file records what **planning** decided on top of it: the user's answers during drafting, and the calls made when merging four independent drafts. The drafts were risk-first, fewest-slices, asset-pipeline, and seam-quality (the last run on Codex). Each entry says what lost and why. Tuning changes made during the build are appended at the end.

## User decisions during planning (2026-09-25)

- **A playable floor.** Slice 27 fails below 30 FPS average at the default camera, at 1920×1080 on this Mac, measured by the benchmark. Everything else stays "record, don't gate" until the performance-budget spec.
- **Command keys.**
  - Backspace stops.
  - Ctrl+right-click attack-moves; R also arms attack-move.
  - F toggles fire policy.
  - G stays attack-ground.
  - T deploys or packs.
  - Escape disarms.
- **Parallel lanes.** Camera, controls and offline asset tooling run alongside the simulation slices. Any visual slice that consumes a rule still waits for it.
- **A scripted benchmark, runnable from a main menu,** modelled on `~/dev/game/web/src/battle/benchmark/`. It uses a versioned scenario, a real simulation, a keyframed camera tour through named phases, and recorded metrics with a results screen. The game gains a minimal main menu to host it (slice 10).

## Planning calls (from merging the drafts)

- **The port is not mechanical.** The import graph from `~/dev/game/packages/battle-renderer/src/world/` is about 173 files and 25k lines across five packages, against about 2.5k lines in our renderer today. Spike 01 measures the pruned closure before any port. Each manifest entry has a mode:
  - `copy`;
  - `adapted`;
  - `technique` (rewritten from reading the source).

  Rejected: copying whole pillars as they are.
- **Manifest pins go per entry.** Battle-foundation's manifest pins `e93a79aa`, while `~/dev/game` HEAD is `d4394dfc` and moving. Each entry records its own source commit and hash.
- **The fog technique is chosen by measurement (spike 02), against a fixed acceptance bar.** Building sight-shadow edges must be sharp at Broken Arrow ground framing, and cost and agreement with the simulation are measured.
  - Candidates, in preference order:
    1. per-pixel sight lights: per-unit angular horizon maps, culled per screen tile;
    2. a 1 m GPU viewshed texture.
  - Rejected: 100 omnidirectional depth cube maps (unaffordable), and a per-fragment march over every unit.
- **What the fog decides.** Renderer fog only shades the world. Whether a unit is drawn is decided by the simulation's identification, as today. The 8 m sweep stays the rule for learning remains and ground cells. A 1 m ground cell counts as seen when its 8 m fog cell is seen.
- **Craters never enter navigation.** Planning ignores them; their slowdown applies only in movement integration. Learning a crater never rebuilds a side's navigation grid (landmine 3).
- **Animation feed additions beyond Q7:**
  - `Corpse` gains soldier id, unit kind and yaw, so death poses and corpses pick the right model.
  - `VisibleSegment` gains the shooting soldier's id, so squad muzzle flashes land on the right rifle.
  - Integer counters are encoded exactly in 16-bit limbs inside the float32 transport, not trusted to 2²⁴.
- **Infantry model facing is presentation only.** Derived from velocity, else the mount's bearing, else the unit's yaw. Infantry sight stays an even 360° (Q5).
- **Vehicles carry every mount and moving part.**
  - The tank has two turret mounts, cannon and HMG, each with its own published pose and named node.
  - The supply truck has deploy legs and a mast, driven by `DeploymentState.progress`.
  - Wrecks use a burnt material variant.
- **Bundle encoding.** Port the appearance-bundle *contract*, not its encoding. `~/dev/game` ships JSON-float animation (about 5.5 MB per appearance), repeated in every tier GLB, which is how it reached 3.3 GB. Ours is binary and content-addressed, with clips shared once per skeleton.
- **Impostors are baked by our own renderer** in the model workbench. Rejected: three.js as a dev-only authoring dependency, which is `~/dev/game`'s route.
- **Missing clips are authored in Blender** on the chosen CC0 rig. Quaternius's free Universal Animation Library tier has no rifle-hold, kneel-fire or prone. CMU mocap is excluded because its licence is custom, not CC0. Mixamo is excluded as not redistributable.
- **Model-to-hitbox fit tolerance lives in the asset catalog, not the fixture.** It is a presentation check on art, not a rule. This is a named exception to "every number in the fixture".
- **Model sheets are judged less-wrong, not by pixel goldens.** Hardware Metal renders are not bit-stable, unlike `~/dev/game`'s SwiftShader baselines.
- **Spikes are throwaway.** Ricochet and grass feasibility are kill gates inside their own slices, not separate spikes, because their fallbacks are local:
  - ricochet falls back to a next-tick child round, a named deviation that needs the user;
  - grass falls back to near cards plus a painterly far texture.
- **Fog is judged after light.** Whether fog could be mistaken for sun shadow can only be judged once sun shadows exist, so 14–15 follow 13. The `FogTerm` stub lands in 12, so every later shader inherits it.
- **Rejected: the fewest-slices ten-slice ladder.** Its merged slices, for example one covering ricochet, the event feed and the ground layer, had decision budgets too large for one pass. Its refusal to merge any two visual variables into one verdict is kept.

## Tuning log

Append entries during the build: observation, before/after values, paired seeds or frames, consequence, and why.
- **Slice 00, the user's rule: HE against armour.** A tank firing at a tank loads AP while any remains and HE after. HE that fails to penetrate still deals a fraction of its damage (`armor_fraction`, provisionally 0.15 for tank HE: 12 per hit against AP's 40). So "can damage a vehicle" includes any kind with a nonzero armour fraction, and a cannon out of AP keeps engaging, ordered or not. Rejected: an ordered-fire-only fallback with HE doing nothing to armour. It was built, then removed within the same slice, because the user wanted partial damage. Pinned by `a_cannon_loads_ap_against_armour_and_he_once_ap_is_spent` (`crates/sim/tests/weapons.rs`).
- **2026-09-25, the user accepted the Quaternius licences (CC0-1.0)** for the Universal Animation Library and Universal Base Characters. Each file taken from them gets a `third_party` manifest entry with `accepted_by: "user, 2026-09-25"`. Spike 03 and slice 21 are unblocked.
- **Slice 04, directional sight: tank and truck `{front 1.0, side 0.5, rear 0.3}` (`sensors.sight_shape`), infantry an even 360°.** Paired reports, main `0ad189b` against this slice (each build reading its own fixture), seeds 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 900 s:
  - Village. Only `scout-suppress-flank` moves: **7/10 → 6/10 captured**, blue cost lost 1052 → 1065. Seed 5 goes from captured at 482 s to inconclusive; seed 13 captures at 754 s instead of 722 s (cost 88 → 100); seed 21 at 498 s instead of 513 s. Every other script is unchanged seed for seed (unsupported push 0/10 at 4762, ambush 0.75 s 0/10 at 1205, ambush 3 s 0/10 with 3 tanks lost, crossfire 0/10 at 1205 with no tank lost).
  - Why seed 5: a single surviving red soldier falls back to about (1200, 950) and contests the zone. Blue's halted tanks face east, and he stands about 345 m away and 24° off their turret, where the lobe reaches 321 m (it reached 350 m before). Nobody else is close enough to see him, so the hold never starts. Probes: with the tank shape isotropic the seed captures at 481.9 s again, and with the supply shape alone isotropic it does not; with tanks looking along the hull instead of the turret it is still inconclusive. So it is the tank's shape, not the forward rule, and not the idle-turret question in `choices.md`.
  - Consequence: encounter.md's "supported captures ≥ 7/10" target, which the baseline met exactly, now misses by one seed. The shift is one seed at the edge of tank range, not "a lot", so no number was retuned: widening the side band enough to reach him (side ≥ 0.9) would undo most of the rule the user asked for. Flagged for the user; the script may need to sweep a contested zone it cannot see.
  - Endurance (`endurance_report 10`, seed 1): outcomes are identical (172 → 121 living units, 476 corpses, 26 wrecks, 70,778 rounds launched). Whole-run tick p50 19.0 → 12.9 ms, p95 42.1 → 35.3, p99 51.0 → 40.1: each fog-sweep ray now stops at its own reach, so vehicles sweep less ground astern. The timings were taken on a machine shared with other agents, so treat them as direction, not magnitude; a first pair taken earlier showed the same drop (20.7 → 13.3 ms).
  - Tests: no existing pinned result flipped. `ground_range_bounds_identification_by_observer_class` (the `sensing.rs:27` pin) now names the tank's front multiplier on purpose. Red vehicles placed east of blue in labs and tests (`damage.rs`, `contacts.rs`; the weapons, sensors and contacts labs) now face west deliberately (landmine 6). No pixel check was retuned; the sensors scene gained two lobe checks.
- **Slice 12, retuned pixel check: `sensors` "fog darkens ground behind the ridge".** Under flat shading it required a display-luminance drop above 20 (of 255). Fog now applies in the HDR world before AgX, which compresses the 0.68 darkening: the same fogged ground drops 10 from 124.4 (8%), clearly grey in `throwaway/evidence/sensors/frame-fog-1280x800.png`. The check is now relative: the hidden ground's luminance falls by more than 5%, and seen ground still moves less than 3. Relative, so slice 13's exposure and slice 15's unseen look can move it without another retune. The only pixel check retuned; `foundation`, `contacts`, `ballistics` and `geometry` pass unchanged.
- **2026-09-25, the user after spike 03.**
  - Models get a real modelling budget, and the unprimed critique stays a pass/fail gate for slices 21–22. Rejected: a deliberately stylised soldier.
  - The tank muzzle moves to a realistic gun length, about [5.9, 0, 2.0], in slice 22. That is a named rule change with paired reports and a digest change.
- **Slice 05, the animation feed: digest grows, no rule changes.** `Mount::digest` gains each mount's `elevation` and `shots`; a fallen soldier's record (`Soldier.corpse`, now `Fallen {at, yaw}`) gains its yaw. Every other addition (segments' kind, shooter, hit and normal; blasts; member ids; weapon poses) is presentation data derived each tick from authoritative state. Digest values change; outcomes do not. Paired reports, `bb65e60` against this slice, same seeds:
  - Village (seeds 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 900 s): every row identical, script by script and seed by seed (unsupported push 0/10 at 4762, scout-suppress-flank 6/10 at 1065, ambush 0.75 s 0/10 at 1205, ambush 3 s 0/10 at 1582 with 3 tanks lost, crossfire 0/10 at 1205).
  - Endurance (`endurance_report 10`, seed 1): outcomes identical (172 → 121 living units, 476 corpses, 26 wrecks, 70,778 rounds launched). Whole-run tick p50 17.6 → 14.2 ms, p95 58.7 → 47.3, p99 88.7 → 65.8. The "before" ran while a full `cargo test` was running beside it at load about 22, so the drop is contention, not the slice; the feed adds only per-tick bookkeeping in `fly` and the observation.
  - Tests: no existing pinned result flipped and no pixel check was retuned. `garrison.rs` reads a corpse's place through `Fallen::at`; the garrison scene and the lab's impact marks read `hit !== "none"` where they read `impact`.
- **2026-09-25, the user's future direction: soldiers move individually, as in Company of Heroes,** each finding cover on its own. That is a future spec. Here, presentation must stay per soldier (README firewalls). Rejected: formation-slot-driven soldier placement or facing in the renderer.
- **2026-09-25, the user: use the pmndrs `math` package wherever it fits** (`math@0.1.0`, MIT, https://github.com/pmndrs/math at `983a607`; its skill is in `.agents/skills/math`). It is the single owner of TypeScript vector, matrix, quaternion, shape, culling, noise, seeded-random, easing and spring math. New TypeScript math uses it from now on, following the skill: plain tuples, `out`-first, no allocation in hot paths. Hand-rolled equivalents (`renderer-core` `mat4.ts`, `camera3d.ts`, the helpers in `math.ts`, and the ported shader-side CPU math) migrate in slice 28, and no second math owner survives it. Rust simulation math is out of scope, and WGSL is unaffected. Rejected: adopting it only where the benchmark shows CPU cost.
