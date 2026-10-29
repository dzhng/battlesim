# Air-war flight

**Status:** shipped 2026-10-16. Rationale record. The user's decisions live in two ledgers: [the map](map.md) (D1–D37 and D29c, loadouts L1) and [the flight-model map](flight-model-map.md) (D29c, D37–D57). [Choices](choices.md) holds the calls made where those were silent, including the whole-spec review's fixes. The build plan (slices, handoffs, checklists) is in git history.

## What shipped and why

Fixed-wing aircraft fly as jets. The parent spec, [air war](../air-war/README.md), gave jets a persistent battlefield but flew them with the helicopter mover: they turned on the spot, hovered over their goal, bombed from a hover 1.5 km out and dogfought in a sine-wave weave. Now:
- **A jet never stops.** It cruises 250 m above the ground (D1), never under a hard 100 m floor while alive (D20), at cruise or afterburner speed only (D3′, D9), and flies where its nose points.
- **Goals are circled.** A move goal, a held position and the end of an attack are circled at full turn rate, so the radius is speed over turn rate (D4/D10).
- **Attacks are runs.** Line up, turn in, commit nose-on under the weapon's ceiling, fire or release, bank away wherever it must and come round (D13, D14, choices: runs are short commitments). Any loadout flies (D23). The gun fires whenever the nose is on a legal target (D30); air-to-air and anti-radiation missiles fire on their own (D16). Bombs are unpowered and leave the jet carrying its velocity (D6′, D21).
- **Dogfights are turning fights** between two fixed-wing aircraft in one layer, with lag pursuit, merge breaks and seeded height wander (D8′, D11, D29c).
- **Arrival and departure use the side's own back edge.** An aircraft enters already flying at the edge point nearest its side's entry site (D39), after an optional authored wait (D38) that the purchase card counts down (D42). A withdrawing jet leaves by the nearest point of that edge (D28′). Off-map flight exists only as a presentation effect (D35).
- **Every planned jet is enabled** with one row per airframe, a historical loadout (L1, D22′), a reviewed model (D26) and a voice. `test_jet` has its own unmarked grey model (D43). Only drones remain planned (D37).
- **The skirmish AI buys and flies jets**, opens with air defence, and buys more when it sees enemy high air (Q14, below).

The build also became a first-principles cleanup of everything jets touch. The user's rule was that behaviour belongs to what physically has it, and that the configuration decides what code must not re-check (D52–D57). So the air-defence tier, role flags (SEAD, `ground_attack`, auto-fire inference), the airborne aspect multiplier, height-chosen layers and several guards are gone. Their meaning is in the data: weapon target classes per layer, `missile` as a target class, radar as a sensor, flares and fuses as row and unit properties, aspect as per-face armour.

The user framed every rule as a war-film moment: a jet you can read from an RTS camera (250 m, not 1,000 m), a bomb the jet has just passed when it lands, a missile that bursts beside a turning jet rather than missing it by a hair, and a side that skipped AA taking heavy damage before it recovers.

### User decisions made during the build

| Question | Answer (user, 2026-10-14) |
|---|---|
| **Q13:** Should bought helicopters arrive the way jets do? | No at first; superseded by D38 and D39. Every aircraft now enters flying at its side's back edge, and only a row with `arrival` waits. |
| **Q14:** Should the skirmish AI buy jets and answer enemy jets? | Yes. It buys high-air-capable units when it sees enemy `HighAir`, and jets join its reinforcement rotation under attack-move. |
| **AA is standard:** | Every standard opening includes at least one AA unit that can reach jets. Jets are tuned so that a side without AA takes heavy damage before it recovers. |

## Principles and invariants

- **High air is an elevation layer.** It is the one layer that bends physics: real jets fly around 1,000 m up, lowered to about 250 m for play. Its treatments key on the layer, never on "jet" or a unit type: no ground sight and no ground fog cleared, the soft shadow beneath, its own screen plane above the lower cards, cruise height and floor. A future high-air unit (a blimp) must need no new logic. Read through `Unit::layer` in the sim, `UnitType::high_air` in presentation and `highAir` in the web.
- **The flight model is separate data from the layer.** `mobility.air.flight` (`rotor` | `fixed_wing`, `contract::catalog::FlightModel`) is required, with no fallback from the layer. It is read through `Unit::fixed_wing` in the sim, `UnitType::flight` in presentation and `fixedWing` in the web. Wing behaviour keys on it: the flight law, passes, dogfights, nose-on guns, banking, and no ground marker, drop line or destination circle. The catalog refuses a pair no flight law flies yet: a rotor in high air, or fixed-wing in low air (D37). A `ground`-layer aircraft is refused too.
- **`crates/sim/src/air/` owns fixed-wing behaviour, at its layer's heights.**
  - `fixed_wing.rs` is the only flight law; nothing else writes a winged aircraft's position, heading or speed. `movement/air.rs` flies rotor aircraft only. Jets never use the rotor air grid (D19).
  - Orders reach a jet's flight only through `air/orders.rs` (`task`) and `air/steer.rs` (`Intent`). Weapons never turn a jet.
  - `air/pass.rs` owns pass phases and pass-weapon choice. `flight/release.rs` owns where an unpowered round lands.
  - `air/steer.rs` owns the edge guard and everything that asks where it takes a jet.
  - `air/dogfight.rs` is the only writer of `dogfight_target`. The break distance is rules data.
  - `air/edge.rs` owns the back edge, entry and exit.
- **The sim never holds a position off the map.** Off-map flight is only the published own-side `AirTransit` record and the effect drawn from it (`crates/presentation/src/effects/transit.rs`). Nothing senses, targets or kills a transit. Rounds and downed jets that cross the edge just end (D31).
- **A fighting jet stays inside the square.** The edge guard turns every intent but an exit away at the jet's point of no return: the last moment a full-rate turn still keeps it `edge_margin_m` inside a straight edge (D36). In a corner, turning away from one edge carries it toward the other, into the margin but never out.
- **A run is a short commitment** (`pass::PassPhase`). Lining up (`LineUp`, `TurnIn`) commits to nothing. A run (`Dive`) begins the moment the nose comes on the aim, short of the release point and clear of the guard, however much run-in is left. It ends when its weapon has done its job (the bomb released, the gun past the aim) or when it is broken: by the edge guard, by its target dying or being lost, by a new order. Every end is the same: the next run is lined up from wherever the jet is, by the same logic as the first. No phase waits on a condition the edge guard can block.
- **Every order ends a jet's run.** No pass, release cue or dogfight link survives an order a jet is given, whatever its kind, even the same order repeated or a route upgraded (`air::break_off`, called through `battle::take_order` wherever an order reaches a unit, and where a jet turns for home). A jet still under an attack order lines up afresh.
- **One height owner.** `AirRules::cruise_agl_m(mobility)` answers "how high": a row's own `mobility.air.cruise_agl_m`, else its layer's height, never under high air's floor (D40). The sim reads it (`Unit::cruise_agl_m`, set at spawn); the web's `ghostAloft` mirrors the same rule over the same `air` data. No literal heights remain. Height never chooses a layer.
- **Distances between two bodies are 3D**, through `math::body_distance` (D32′). Reach, ranking, dogfight break and radar detection use it. Ground paths, circles, footprints and the release solve stay 2D. A 2D "in reach" beside a 3D fire gate freezes a unit.
- **Configuration, not guards.** Never re-check in code what the data already decides (D57). What a weapon engages is `engages(layer, class)` on its row's per-layer target classes (D53), and what a jet can attack is its rows with rounds left that engage the target (`air::engaging_rows`), which both its pass and its attack-move pick read; interception is the `missile` class a layer lists (D54); a unit sees high air by its authored `sensors.high_air_m` and a receiver finds emitters (`radar_emitter`) by `radar_receiver` (D56; D47 superseded by [unit concealment](../unit-concealment/README.md)); auto-fire is the row's `fires_on_its_own`; a fuse is the row's `proximity_fuse_m`, and a diverted seeker meets its flare at `guided.flare_meet_m`. A restriction is expressed by what the data gives each unit. Load refuses the unsupported pairs: an unpowered row on anything but a winged aircraft, a `missile` row without a fuse, a seeker interception row that isn't guided.
- **One aim rule, keyed on the target** (`Unit::aim_height`): anything in the air at its hull centre, a ground vehicle at a fraction of its hull height, a soldier at a fixed height. Target resolution, launcher guidance, seekers and the return-fire threat all read it.
- **Reach is judged where the shot meets its target** (the lead solution's meeting point), for every shot.
- **A round's starting velocity is set once, at launch.** Every round fired or dropped from an aircraft starts with its velocity (`Unit::flight_velocity`, D21). A bomb launches from the jet's muzzle, never from the solved release point; the pass must release on time.
- **A side fires at and guides onto only what it sees.** An interceptor fires at, and guidance steers at, a round only while its side sees it (`sensing::side_sees_round`: from each unit's eyes, never a ground round from high air), as for any other target. A seeker's contact is its own side's.
- **The nose-on cone is one number**, `air.fixed_wing.nose_on_deg`. The fixed gun's fire gate and a pass's dive both read it. A hull-fixed row the nose aims (unguided and powered) fires only nose-on; a seeker aims by its cone, an operator-guided row by its guidance, a bomb by its pass's release cue.
- **One rule for the AI and the player.** Arrival, purchases and orders follow the same rules for both (D45). The AI remembers its jets' targets itself (`BalancedDoctrine::jet_targets`), as a player would.
- **One row per airframe.** No generic or duplicate rows, and no fixed-wing entries in the model manifest.
- **Every number lives in data**, rules or rows, and is a starting guess, except the 100 m floor, which is a hard requirement (D20). Tests pin mechanisms, never balance outcomes.
- **A scenario may override any rules entity** (D51). The menu reel's encounters pin the stats they were filmed with, so a stat change doesn't force a reshoot; a mechanics change is never held back to keep the reel.

## Where it lives

- **Simulation:**
  - `crates/sim/src/air/` (`step_fixed_wing`, `FixedWingLimits`, `steer`, `Intent`, `pass::{choose_weapon, advance}`, `dogfight::{on_fire, breaks, intent}`, `edge::{back_edge, entry, arrival, crossed}`, `orders::task`)
  - `crates/sim/src/flight/release.rs`, `crates/sim/src/math.rs` (`body_distance`), `crates/sim/src/units.rs` (`aim_height`, `flight_velocity`, `fixed_wing`)
  - `crates/sim/src/weapons.rs` (`nose_on_bearing`), `crates/sim/src/sensing.rs` (`side_sees_round`), `crates/sim/src/sight.rs` (high-air sight by radar, own sensor or body)
  - `crates/sim/src/skirmish_ai/mod.rs` (`jet_targets`, the saving rotation, `relocate_after_s` for held launchers)
- **Contracts and data:** `contract::scenario` (`AirRules`, `FixedWingFlight` under `air.fixed_wing`, the layer heights under `air.low_air` and `air.high_air`), `contract::catalog` (`FlightModel`, `mobility.air.arrival.delay_s`, `mobility.air.cruise_agl_m`), `contract::weapons` (target classes, `WeaponDefinition::engages`), the observation's `AirTransit` and the pending record's `arrival_tick`; `fixtures/game.json`, the roster files under `fixtures/units/roster/`, the profiles in `fixtures/units/air/profiles.json`, `fixtures/skirmish-ai.json`.
- **Presentation and browser:**
  - `crates/presentation/src/effects/transit.rs` (approach and departure), `effects/high_air_shadow.rs` (the shadow beneath), `units/driver.rs` (attitude from the flight path), `HIGH_PASS` (the high-air model batch).
  - The high-air screen plane: `HIGH_PLANE_ATTR` in `web/src/battle/present/readouts.tsx` and `postHighPlane` in `packages/battle-renderer/src/world/post.ts`. Each plane's card layout is the pure `readoutLayout.ts`; the edge rule (`presentation.hud.panel_edge_hide_px`) is applied in `readouts.tsx`.
  - The countdown card in `web/src/battle/present/armyDeck.tsx`; the order ghost's height in `apps/battle-lab/src/unitGhosts.ts`; the labs' reach grouping (`airRole`, `airReach`) in `packages/scene-assets/src/units.ts`.
- **Tests that pin behaviour:**
  - `crates/sim/tests/fixed_wing_flight.rs` (never slower than cruise, afterburner ramp, floor, circling, edge), `attack_passes.rs` (passes, the gun, bombs, orders and reactions; `every_loadout_fires_at_a_legal_target` flies every fixed-wing row at an enemy it engages, and `every_fixed_wing_row_arrives_and_withdraws_on_low_fuel` buys each, which arrives, turns home on low fuel and is refunded; `repeated_passes_each_dive_in_nose_on_and_fire` and the edge and corner `runs_*_keep_firing`; `every_order_ends_the_run_at_once`; `a_bomb_leaves_the_jet_and_falls_onto_the_aim`; `a_jet_circles_where_its_attack_ended`), `dogfight.rs`, `distances_3d.rs`, `flight_ballistics.rs`, `air_missiles.rs` (fuse, seeker, interceptors), `air_weapons.rs`, `air.rs` (heights, sight, back edge, ground fog), `radar.rs`, `skirmish.rs` (arrival, transit, withdrawal), `launch_clearance.rs`, `catalog.rs` (loadout rules, `only_drones_are_planned`).
  - `crates/sim/src/skirmish_ai/tests.rs` (openings, AA reaction, jets on attack-move, saving).
  - `crates/contract/tests/weapons.rs` (unpowered rows, lifetimes that reach their range).
  - `crates/presentation/tests/transit.rs`, `pose_behaviour.rs` (pitch and bank), `high_air_shadow.rs`.
  - `web/tests/unitGhosts.test.ts`, `armyDeck.test.tsx`, `aircraftMarks.test.ts`, `readoutLayout.test.ts`.
  - The test kit `crates/sim/tests/common/air_field.rs` (`AirField`, `Track`).
- **Labs:** the `/labs/air-*` checkpoints on the `air-jet` map (3×3 km; jet scenes) and `air` (helicopters), scenes `web/scenes/air-*.mjs`. Each header names the sim test behind its claim.
- **Art:** `packages/scene-assets/blender/aircraft_parts.py` (shared airframe parts and materials), the jet family scripts in `blender/roster/`, `blender/test_jet.py`.

## How the build diverged from the plan

- **"Jet" became two owners.** The plan had an `air.jet` rules block, `air/jet.rs` and `step_jet`. The user then named high air an elevation layer, so layer treatments moved to `air.low_air`/`air.high_air` and the layer, and fixed-wing flight to `air.fixed_wing`, `air/fixed_wing.rs` and the flight model. "Jet" survives only on `test_jet`, the `air-jet` lab map, scenarios that fly one, and the AI's jet bookkeeping (`jet_targets`, `is_jet`). The plan's `jet_flight.rs`, `jet_runs.rs`, `jet_dogfight.rs` and `common/jet.rs` shipped as `fixed_wing_flight.rs`, `attack_passes.rs`, `dogfight.rs` and `common/air_field.rs`.
- **Dogfights key on the flight model** (D29c superseding D29b). Today's outcome is the same as D29b's, but a fixed-wing drone in low air would dogfight another, never a helicopter.
- **Every aircraft arrives the same way.** Helicopters were to keep the four-sides spawn and `traversing` (Q13). Per-unit arrival (D38–D42) replaced both for every aircraft; `traversing` is deleted outright, panel row and card included. A waiting aircraft leaves the purchase queue instead of blocking it.
- **The jet's shadow is not cast by the sun.** The plan threw it along the sun from the actual height. From 250 m that lands about 300 m away and reads as disconnected, so a high-air unit casts none and shows a soft fighter silhouette directly beneath it (the user's choice, [choices](choices.md)).
- **High air got its own screen plane** (unplanned, user). The DOM cards sit between two layers of models, so high-air models draw into a second transparent canvas between the lower cards and the high-air cards.
- **A pass turns in before it dives, and has no release phase.** The planned bearing was fixed when a pass began; repeat passes then dived 40–50° off the nose and never fired. `LineUp` now flies on until a full-rate turn leaves a run-in, `TurnIn` holds the turn until the nose is on, and the release cue is the dive's last tick.
- **Interception became a target class.** The plan had an interceptor use its farthest-reaching air row against a round. D54 made `missile` a target class each layer lists, and guided rows may intercept; the Patriot PAC-3 and Pantsir-SM got their own rows (D55).
- **The scope grew into weapons and sensors.** No air-defence tier (D52), radar as high-air sight and sight by body (D47–D49), role flags replaced by physical properties (D56), guards removed and aspect as armour (D57), reloads capped near 5 s, except artillery and the long SAM (D50), scenario overrides (D51). This landed [rpg-weapon-contract](../rpg-weapon-contract/README.md) early and is now archived.
- **The proximity fuse is its own row field** (`proximity_fuse_m`), not the blast radius, and it bursts near whatever the round was fired at, ground units included. `aa_ballistic` rounds therefore airburst over ground units they are fired at.
- **The closeout fixed a fog hole under every jet.** The observation published a high-air unit's ground eye, so the drawn fog cleared a disc beneath it that read as a marker on the field; high air now lights no ground fog (`a_high_air_unit_lights_no_ground_fog_in_its_sides_picture`).
- **The AI saves for its next card.** Skipping unaffordable roles had spent every credit on the cheapest cards, so jets were rarely bought. Every purchase, the player's and the AI's, now goes where a move from the side's entry would end (the battle snaps the destination), which stopped the AI's repeated refusals of a blocked destination. The tournament runner had hard-coded a 400-credit opening; it reads `fixtures/skirmish.json` now.
- **The scene runner rebuilds its native tools on every run** (cargo skips a fresh one); stale tools caused three false scene failures during the build.
- **The whole-spec review** confirmed 68 findings, all fixed, and a second round reviewed the fixes. The rule-level ones are the last rows of [choices](choices.md): bombs leave the jet, bombs hold for friendlies, interceptors and guidance need the side's sight of a round, fused interceptors, the nose-on cone as data, seeker-first ranking by `engages`, passes only with a row that engages the target, circling where an attack ended, and bomb scatter around where the fall lands.

## Dead ends

- **Releasing a bomb at the solved point.** The bomb teleported up to about 150 m to the ideal release point and always fell on its aim. It now leaves the muzzle, and the pass must be on time.
- **Fixing a pass's approach bearing when it begins**: see the pass divergence above.
- **A tangent approach from outside the circle.** Prototyped and reverted, never committed. A jet flies at the centre until it nears the circle, then turns onto it in its current turn direction (D4/D10), so it can loop through the centre on entry. See the open items.
- **A tournament `--sticky-jets` filter.** Replaced by the AI remembering its jets' targets.
- **Keying the nose-on rule on every hull-fixed row.** It held seeker missiles and bombs to the gun's 2° cone.
- **An air-only own-sight rule for AA crews.** Guided fire already needs the operator's line; the special case was deleted (D52).
- **Reach tolerance at a bomb's release point for non-wing carriers.** Refusing unpowered rows on anything but a winged aircraft at load needed no new number.
- **A catalog scan for the helicopter routing margin.** The sim's test catalog and the browser's full catalog routed differently; it is one data value, `hull_limits.rotor.half_length_m`.
- **For the high-air plane:** drawing cards on the GPU, CSS `z-index` on the canvas (it rises above the HUD too) and a second frame were rejected.
- **For the shadow:** a per-model top-down outline bake (a blurred procedural plan reads as a jet) and a receiver-side shadow-reach cap (it would change every shadow). Halving `blur_per_m` did not change the "doesn't read as high" verdict either.
- **For the airframe look:** flattening the paint's baked vertex colour did not change the fuselage's darkness; that is sun-side against shade-side lighting.
- **For the dogfight panel mark:** two equal planes read as a smudge and nose-to-nose chevrons as a bow tie.

## Visual provenance

- **The behaviour mock** [`visualizations/flight-mock.html`](visualizations/flight-mock.html): a Canvas2D sketch with sliders, written during the unknowns walk and copied here from the scratch folder. It shows move-goal circling, bomb passes, missile passes with the gun, a dogfight a third jet joins, fly-in and withdrawal, and the SAM chase. It was the comparison target for circling, passes, dogfights and the approach effect. The user flagged nothing in it, and its logic was checked: bombs land within 2 m, the gun never fires on bomb passes, and it does fire on missile passes and in dogfights.
- **Aircraft references** in the repo's `assets/references/<family>/` drove the model pass. Thin sets (F-15EX, F-35A/B, Su-30SM2, Su-57, Tornado IDS, J-20) were filled with Duet-generated views, marked `-generated` in the file name and `"source": "generated"` in the family's `references.json`. Each family was judged against them with `compare-screenshots` and unprimed critiques.
- **The panel marks** in [`assets/panel-marks/`](assets/panel-marks/): `pair-radar-3x.png` and `pair-dogfight-3x.png` compare the old and new radar dish and dogfight marks at 3×, and `after-aircraft-2x.png` is the aircraft-states sheet after the change, at 2× (its TRAVERSING card has since been deleted). The approved standard is the baseline `web/scenes/baselines/panels/panels-aircraft-states.png`, re-approved with the radar-receiver panel.
- **No baseline exists for jets in flight.** The air scenes assert the side's observation and write their traces and GIFs to the scratch folder, which is not kept; the closeout's three AI-vs-AI skirmish GIFs (D44) were reviewed and discarded with it.
- **Market Town's menu reel** was re-recorded after D57 retimed it from its first shot; the user approved the new footage.

## Balance at closeout

Measured with `skirmish_tournament` shaped battles (seeds 1, 2, 4 and 5, all six faction pairs, 900 s, fixed buy lists), never as a test. Against a side with no AA, ground-attack jets did about 2.3 times the kill value they did against a side with one AA unit, and one AA unit shot down about 41% of sorties ([choices](choices.md), the tuning row). Radar AA (Gepard, Pantsir) blunts jets far better than optical short-range AA, so the Stryker M-SHORAD and Boxer Skyranger 30 dropped to 180 credits; no other price moved.

## Open items and future work

- **Idle fire-at-will jets don't turn on enemy jets they see** (D15: a holding jet starts no runs on its own). Accepted; revisit after play testing.
- **Circle entry can loop through the centre** once before it settles: a jet circles the way it is already going round the centre (D4/D10, [choices](choices.md)). A tangent entry was prototyped and reverted; revisit after play testing.
- **A jet with no weapon able to hit its attacker circles over it** until the track is lost.
- **A jet near the top screen edge shows no card**: the 36 px edge rule, kept as the user asked.
- **Browser info cards don't read scenario overrides.**
- **The Stryker M-SHORAD is weak**: optical sight only, 900 m of high air.
- **Flares are resupplied only by hulled ground supply units.** A jet never stands still in reach, so it refills only by withdrawing.
- **Native has no info cards or high-air plane**: [native gaps](../../native-gaps/README.md) item 6.
- **Radar networks**, launchers that see far only through separate radar units: [air-defence radar](../air-defence-radar/README.md), deferred and superseded as an active plan.
- The archived [rpg-weapon-contract](../rpg-weapon-contract/README.md) records the Eastern RPG and blocked-launch closeout.
- **Drones** (low-air fixed-wing at about 100 m) need their own spec to give low air fixed-wing heights (D37).
