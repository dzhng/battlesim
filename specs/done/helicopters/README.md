# Helicopters

**Status:** shipped 2026-10-10. Rationale record: [decisions](decisions.md) holds the user's rules (D1–D43), and [choices](choices.md) holds the calls made where they were silent.

## What shipped and why

All 19 roster helicopters are playable for the player and the skirmish AI, and they play the way Broken Arrow and WARNO helicopters do:
- They cruise 20 m over the ground, nap-of-the-earth, and pop up over roofs.
- They fly round anything taller than 30 m.
- They strafe: the airframe turns to bring its rockets, missiles and any gun fixed in the hull to bear while it flies on. Chin and door guns are turrets (D28).
- They fire missiles and rockets on the move.
- They resupply by sinking to a low hover at a supply truck.
- They shrug off rifles and fall to autocannons. Heavy snipers and machine guns can hurt them too.
- They come down in a spinning arc, and the wreck flattens the trees it lands on.

The user framed every rule as a war-film moment rather than a ballistics table:
- Rifles keep firing at a helicopter they can't hurt, because sparks off armour read well on screen (D2).
- A tank's main gun never engages a helicopter. D1 makes this a data rule (each weapon row's `targets`), not a gun-elevation limit. A tank's HMG still does engage one.
- One altitude rule replaced two flight modes (D4). The old [battle-foundation slice 18](../../battle-foundation/slices/18-air-movement.md) had normal and low flight with a stealth trade-off; the user replaced it.

Research behind the rules:
- [Broken Arrow dev diary #2](https://forum.slitherine.com/viewtopic.php?p=936391): helicopters as fragile cavalry that fly nap-of-the-earth.
- [Broken Arrow on NamuWiki](https://en.namu.wiki/w/Broken%20Arrow(%EA%B2%8C%EC%9E%84)): low flight still moves and rises over obstacles, and heavy air defence can't hit helicopters. This is the basis of the layer split.
- [WARNO helicopters](https://warno.fandom.com/wiki/Helicopters): unmask to fire, then drop back.

## Principles and invariants

- **An aircraft is a hull with no ground footprint.**
  - `Unit::ground_footprint` returns `None` in the air, and `Unit::airborne` is true. Through one or the other, traffic, shoving, cover, treads, forest lanes and forest concealment never see it.
  - Its motion is `units::Motion::Air`. A site only a ground mover reaches asks `Unit::ground()`, which refuses an aircraft loudly rather than giving it vehicle rules by accident.
  - Pinned by `crates/sim/tests/air.rs` for traffic, shoving and concealment. Cover and treads are not pinned by a test.
- **One altitude rule, measured from the ground.**
  - Cruise height, clearance and ceiling come from one battle-wide `air` block in `fixtures/game.json` (`contract::scenario::AirRules`). The low hover's 2 m clearance over the tallest ground hull is a code constant (`movement::air`).
  - The aircraft holds cruise height and climbs to the clearance over any body top or forest canopy it is about to pass, never above the ceiling (`movement::air::height_target`). The look-ahead reads the true world's bodies and trees, not the side's belief; only routing uses belief.
  - A body taller than the obstacle height is a wall in the side's air grid (`navigation::air::AirGrid`, built per side from what that side believes stands, `SideGeometry::air_grid`).
  - Pinned by `air_flight.rs`.
- **What a weapon can engage is data.**
  - Every weapon row states `targets` or inherits it through `extends` (`ground`, `low_air`), and only `weapons::reaches` reads it. Target choice, explicit orders, threat ranking and the area-fire hold all go through it.
  - Flight never reads `targets`, so a stray round still strikes a helicopter in its way.
  - `high_air` is left out until something flies high (D43).
  - Pinned by `air_weapons.rs`.
- **Plinking is a feature** (D2). Default unlimited-ammo guns (`fires_regardless`) fire at a helicopter they can't hurt. Never gate it.
- **A gun fixed in a hull bears where the body faces, on every vehicle** (D8).
  - `weapons::hull_fixed` defines it. Such a mount reports `Reach.face` whenever it holds a lock on a target it can fire on, and the body turns to that bearing: an aircraft keeps flying its route, an idle tracked hull pivots, and wheels hold fire.
  - Pinned by `hull_guns.rs`.
- **Guidance is a row property, not a consequence of `stationary`** (D38).
  - A guided row states `guidance: "stationary" | "on_the_move"` (`contract::ballistics::Guidance`), and loading refuses a guided row without it.
  - Ground missiles guide only while standing still; `heli_atgm` guides on the move.
  - Pinned by `air_missiles.rs`.
- **Moving means changing place on the map** (D37).
  - The tick's `moved` vector (`battle.rs`) compares XY, so an aircraft climbing or sinking stands still for supply and guidance. Ground units' digests stayed unchanged under this; no test pins it, because a ground hull never moves in Z.
  - The resupply sink (`movement::air::low_hover`, derived from the tallest ground hull) depends on it, or sinking would cancel service.
  - Pinned by `air_supply.rs`.
- **A crash is simulation state, published, and data-driven damage** (D3, D31).
  - `sim::crash::Crash` falls under gravity, keeping the horizontal momentum it died with, and spins at a fixed rate whose direction the unit sets. It glances off immovable bodies, slides off roofs (untested), and stays on the map.
  - On impact it bursts as the `helicopter_crash` weapon row through `damage::detonate`, which shares `damage::casualties` with flown rounds. Trees fall through the same destruction path shell bursts use.
  - The wreck rests on the nearest ground clear of live hulls and immovable bodies, or where it hit if nothing within 30 m is clear (`crash::resting_place`).
  - The kill and every crash casualty go to the shooter; no test pins the casualties' credit. Its own side, and the sides that saw it go down, learn the wreck and the felled trees.
  - The publication's `crashes` group lets the browser draw the fall.
  - Pinned by `air_crash.rs` and `air_closing.rs`.
- **Contacts carry height.**
  - A contact holds its cause's `z` and `layer`, and the simulation publishes `aloft` (in the air, above the low hover).
  - An aloft contact is drawn as the keyed-disc sign facing the camera (`packages/battle-renderer/src/contactGlyph.ts`) and is picked by a sphere (`web/src/battle/input/contactPick.ts`).
  - Any contact in the air layer offers no attack, aloft or not, because area fire can't reach the air (D22). The refusal keys on `layer`, so a report heard from a helicopter at the low hover is drawn on the ground and still can't be attacked.
- **Objective eligibility has one owner.** `objectives::can_hold` decides it: ground combat units only, never supply or aircraft (D20). The skirmish AI counts only such units toward an objective's load. Helicopters reinforce through the `helicopter` role (`skirmish_ai.rs`).
- **A firing report's heard mask spans two 24-bit words** (`heardLow`, `heardHigh`; `publication::MAX_WEAPON_ROWS` is 48), because a float holds an integer exactly only below 2^24. Adding a weapon row renumbers the rows after it, which moves round digests without changing behaviour. Prove that by renaming the new row to sort last before re-recording.

## Where it lives

- **Simulation:**
  - `crates/sim/src/units.rs` (`Motion`, `Flight`, `AirState`, `Unit::airborne`, `Unit::layer`, `Unit::smoking`)
  - `crates/sim/src/movement/air.rs`
  - `crates/sim/src/navigation/air.rs`
  - `crates/sim/src/crash.rs`
  - `crates/sim/src/damage.rs` (`detonate`)
  - `crates/sim/src/weapons.rs` (`reaches`, `hull_fixed`, `Reach.face`)
  - `crates/sim/src/objectives.rs` (`can_hold`)
- **Contracts:**
  - `contract::catalog` (`Mobility::Air`, `AltitudeLayer`)
  - `contract::scenario` (`AirRules`, `HullLimits.air`)
  - `contract::weapons` (`targets`)
  - `contract::ballistics` (`Guidance`)
  - observation fields `crashes`, `smoking` and contact `z`, `layer`, `aloft`
- **Data:**
  - `fixtures/units/air/profiles.json`: five helicopter profiles carrying D5 armour and HP and D26 speeds.
  - The 19 cards in `fixtures/units/roster/*.json`.
  - `fixtures/units/test/aircraft.json` (`test_heli`).
  - Weapon rows `heli_atgm`, `rocket_pod` and `helicopter_crash` in `fixtures/game.json`.
  - The rotor sound `uh60-b-roll` in `fixtures/sounds.json`.
- **Browser:**
  - Rotor articulation (`packages/scene-assets/src/articulation.ts`, the `rotor` input).
  - Drop line and ground marker (`packages/battle-renderer/src/orderOverlay.ts`, `buildAircraftMarks`).
  - The fall and the smoke trail (`packages/battle-renderer/src/models/poseDriver.ts`, `effects/effectFrame.ts`; the lab's `poseFeed.ts` and `effectFeed.ts`).
  - The contact sign (`packages/battle-renderer/src/contactGlyph.ts`).
  - Labs `/lab/air`, `/lab/air-hover`, `/lab/air-crash` and `/lab/air-closing` on the `air` test map, with scenes `web/scenes/air*.mjs`.
  - The hover and closing labs share `apps/battle-lab/src/labOverlay.ts`.
- **Tests:**
  - `crates/sim/tests/air*.rs` and `hull_guns.rs`
  - `web/tests/aircraftMarks.test.ts`, `contactGlyph.test.ts` and `unitControl.test.tsx` (an airborne contact offers no attack)
- **Art:** `packages/scene-assets/blender/aircraft_parts.py` (rotor, chin gun, door gun, wreck helpers) and the helicopter exporters in `roster/` (`apache.py`, `chinook.py`, `havoc.py`, `hind.py`, `hip.py`, `ka52.py`, `little_bird.py`, `h1.py`, `tiger.py`, `z10.py`, `utility_helis.py`).

## How the build diverged from the plan

- **Every helicopter weighs `light`.** D3 gives every helicopter a `light_wreck`, and the catalog requires a wreck's cover to match its unit's weight. So all helicopters, the Chinook included, use the light rotor sound row. This is an open user call ([choices](choices.md)).
- **The closing scene places real roster units by hand.** The scene (`air-closing`, D14) uses a T-72B3 and a BMP-2M rather than bought test units, and no pinned-seed skirmish replay exists: two probe battles found no natural shoot-down. The deterministic lab is the proof.
- **The air hull limit grew to 3.65 × 9.94 m half extents**, to fit the Ka-52 and the Merlin. The air grid widens walls by the limit's half length, so routes keep further from towers.
- **The crash was built simulation-first.** Publishing it came later, with drawing the fall, because the contact-height work was changing the publication layout at the same time. Slice numbers here refer to the build plan, which is in git history.
- **A falling crash must teach the sides that saw it the trees it felled.** Otherwise a lone helicopter's owner, left with no eyes on the wood, draws its wreck under standing trees.
- **Hull guns hold their target throughout.** The first hull-gun rule reported the needed heading only while traversing, so a strafing gunship swung back to its heading between shots. The closeout made it report the heading whenever the gun holds a lock on a target it can fire on.
- **Two closeout findings remain unsound** ([choices](choices.md) U1, U2): an aircraft's path bows up to 0.18 m wide as it closes on a goal round a last corner, and only 84% of the Apache wreck's covered pixels hold their colour as a fogged ghost (the check wants 90%).

## Dead ends

- **Two flight modes** (normal 60 m, low 15 m, low flight trading sight for stealth): replaced by one low layer that pops up over roofs (D4).
- **A gun elevation limit instead of a layer column:** a tank could still hit a distant helicopter 2° up. The user wanted a hard rule.
- **A third `MoverClass::Air` in ground navigation:** about ten sites read "not infantry" as "vehicle" and would have given a flier water and push rules. Aircraft got their own simple grid instead (D21).
- **Making the shared `atgm` row `stationary: false`:** it would have changed infantry missile teams. That is why `heli_atgm` is its own row, with guidance as a property.
- **Deriving guidance from `stationary`:** the user rejected it in favour of an explicit, extensible property (D38).
- **Stopping an aircraft exactly on its goal:** a step-crossing snap and settling within 0.25 m did not remove the curved approach's bow, because the sideways momentum bleeds off about a metre out. A steering law that cancels sideways velocity first is the recorded fix (U1).

## Visual provenance

- **The airborne contact sign** came from a design spike. Five variants, A to E, were drawn as Canvas2D mockups over real game captures. They are not renderer output, and the script that drew them is not shipped. The user did not reply at the non-blocking checkpoint, so the agent picked **D, the keyed disc**: the ground glyph's recipe turned to face the camera, with a dark keyline. D was the unprimed critique's first choice, and the pick is reversible. The files in [`assets/contact-sign/`](assets/contact-sign/):
  - `sheet.jpg`: the five variants over six backgrounds (fog, a village and a tower, each at map and close zoom).
  - `crops-3x.jpg`: each sign with three times its size around it, enlarged 3×.
  - `full-*-fog-close.jpg` and `full-*-tower-map.jpg`: 1920×1080 frames. The fog frames also show the real ground glyph for comparison.
- **The built sign keeps D's recipe but sizes it differently.** It follows the contact's uncertainty, clamped to a 20–44 px radius, which the spike's checkpoint asked for. It also adds a pale stem to the ground, raises fill and hatch, and draws a lit grey-blue rim instead of D's white ([choices](choices.md)).
- **The look and feel** were judged against the Broken Arrow and WARNO descriptions above, with no reference images, and against the user's own words: "almost like a red cloud, a perfect circular red sign that's kind of floating in the air", rifle sparks, and a wreck that flattens trees.
- **The drop line** was redrawn after the closeout. The user disliked the solid 3D line; five Canvas2D concepts over a real capture ([`assets/drop-line/sheet.jpg`](assets/drop-line/sheet.jpg): ring only, a fading hairline, dots, a short stem, a centre pin) were offered, and the user picked the dots, rising from the ring's centre rather than its edge, then asked that they be round, in the ring's own yellow, and hold full for the first fifth of the height and fade out by four fifths of it rather than tie on to the airframe. The dots are lit as marks on the ground, not as solids, and drawn in the marker's colour at full brightness, so they read as the ring's glowing paint does ([`assets/drop-line/chosen-and-built.jpg`](assets/drop-line/chosen-and-built.jpg): the concept, the shipped build, and close over the ring).
- **The panel workbench's pictures** (`web/scenes/baselines/panels/`) were re-recorded at the closeout to include the test helicopter's panels.

## Balance at closeout

Balance was out of scope: the user asked only that helicopters not be clearly unbalanced. The helicopter numbers are first-pass placeholders for a later balance spec ([choices](choices.md) S4, S19).

- **Ground battles still play.** The generated-map battle sweep (`crates/mapgen/examples/battle_sweep.rs`, 10 seeds × 3 map types × 4 sizes, 30 s each) ran on the closed branch: 115 of 120 complete, 563 rounds fired. Four maps were refused by generation (transit or sight furnishing) and one assault order was refused at placement, both before any battle rule runs. The sweep fields only test ground units, so it shows that the shared rule changes (hull-fixed facing, guidance as a row property, XY-only `moved`) left ground battles working. It does not measure helicopters.
- **No paired before/after was run.** Another session's roster admissions landed on main during the build, so a main-versus-branch pairing would not isolate this spec's effect.
- **Helicopter balance evidence is the scenes:** rifles spark harmlessly, an autocannon IFV shoots an Apache down, and the Apache kills a tank with rockets and its chin gun (`air_weapons.rs`, `air_closing.rs`). In two 8-minute skirmish probes the AI's Apache killed a Tigr and was not shot down (choices N6), so it is neither useless nor untouchable.

## What comes next

[Transport](../../transport/README.md), still a placeholder, plans to carry infantry in ground carriers and helicopters, and to land helicopters through `movement::air::low_hover`. Landing on a point will first need U1's exact stop.
