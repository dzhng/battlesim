# Air-war flight: the map of unknowns

The `explore-unknowns` walk on 2026-10-14, held with the user. It has been sliced into [the build plan](README.md); this page keeps the decisions and their reasons. Every decision below was shown to the user before it closed. "User" means the user answered it, "territory" means the code answered it, and "agent" means the agent answered on the user's behalf (only the loadout table, which the user handed over). Every number is a starting guess to tune against screenshots and play, unless it is marked **hard**.

**Done means fully polished and fully playable** (user):
- the air lab scenes pass;
- a played skirmish has every jet working;
- the aircraft model pass is finished;
- the full test and scene run passes once, at the end.

## 1. Known knowns: the ground as found

- **Height is one rule for the whole battle.** Every aircraft flies at `cruise_agl_m` = 30 m from one `AirRules` (`crates/contract/src/scenario.rs:623-650`, `fixtures/game.json` `air`). Cruise height may not exceed the 40 m ceiling. `HighAir` is a catalog label, not a height (`catalog.rs:183-229`).
- **Jets move like helicopters.**
  - The mover never reads the layer. Jets turn on the spot, their velocity is not tied to their heading, and they brake to a hover at the goal (`crates/sim/src/movement/air.rs:96-195, 305-324`).
  - An attack flies in until in range, then hovers (`battle.rs:2968-3003`).
  - Speed changes are capped at cruise / 4.5 s, which allows only about a 13°/s path turn at jet speed.
- **Speeds today.** Helicopters cruise at 180–240 km/h (`fixtures/units/air/profiles.json`). Every jet shares one profile: 300 km/h, 45°/s, 180 s of fuel (`fixtures/units/air/air-war.json:3-75`).
- **Dogfights.**
  - Each aircraft has one `dogfight_target` (`units.rs:327`).
  - A link forms both ways when an aircraft fires on another within 1200 m (2D) and neither has a Move order. That overwrites the target's existing link (`battle.rs:3228-3257`).
  - While linked, the movement is a chase plus a sine-wave weave (`air.rs:71-88, 145-154`).
- **Ingress and withdrawal.**
  - A jet spawns *on* the map edge nearest its goal, on any of the four sides (`battle.rs:856-875`).
  - It sits invisible for 15–30 s, then snaps to cruise height (`battle.rs:633-654, 937-946`).
  - Withdrawal moves to the entry centre and retires within about 15 m of it, stopped (`battle.rs:675-737, 972-993`).
  - The closed spec's claim that "off-map traversal is visible" is false.
- **Ground attack.**
  - Bombs are rounds fired at 220–240 m/s from up to 1.5–1.8 km away, from a hover (`game.json` `air_bomb_*`).
  - Strafing is refused while bombs remain (`sim/src/weapons.rs:962-1050`).
  - Afterburner gives 1.5× speed and 2× fuel, and widens scatter 1.8× against ground targets (`weapons.rs:1940`).
- **Missiles.** Every missile launches at 100 m/s, accelerates at 400 m/s² and tops out at 600 m/s. Seekers chase the base of the target's box with no lead (`battle.rs:3146`). There is no proximity fuse against aircraft (`damage.rs:255, 297`). Launches don't inherit the launcher's velocity (`flight/solve.rs:618`), and the target velocity's z is dropped (`weapons.rs:491`).
- **Hits are swept** chord against moving hull, at a 30 Hz tick (`flight/sweep.rs:44-82`), so fast jets can't be tunnelled through.
- **Obstacles.** The tallest building is about 61 m (the 20-floor highrise in `prototype-building-templates.json`). Trees reach 12 m, authored relief peaks at 25 m, and generated maps are flat (`mapgen/src/lib.rs:792`).
- **The world.**
  - `MapExtents{playable, physical, rendered}` (`contract/src/map.rs:9-31`).
  - The rendered margin is 1500 m on generated maps, filled with fields and trees; a flat backdrop runs to the horizon (`presentation/.../ground/mesh.rs:125-146`, `scenery.rs:720`).
  - The sim has nothing beyond the physical grid: rounds leaving the map expire (`flight/mod.rs:1225`), off-map orders are refused (`battle.rs:1676`), and a downed aircraft is clamped to the edge (`crash.rs:111`).
- **Presentation.**
  - The ghost stands HighAir at 1000 m (`apps/battle-lab/src/unitGhosts.ts:20-23`).
  - The shadow assumes `cruise_agl_m` (`presentation.rs:1535`).
  - The jet model already pitches and banks with its acceleration (`presentation/src/units/driver.rs:1197-1230`).
- **Roster.**
  - 19 fixed-wing rows are planned (disabled): "Aircraft movement and air-combat mechanics deferred" (`fixtures/units/model-manifest.json`).
  - All 19 have a Blender script, a GLB with wrecks, references and an icon. None has an `assets/catalog.json` appearance (except Tornado IDS) or a voice.
  - A row is planned exactly when it has a `planned` block (`catalog.rs:1175`).
  - The generic `*_fighter_aircraft` rows and the SEAD rows in `air-war.json` duplicate roster rows.
  - There are at most 10 families per faction per category, and disabled rows count (`catalog.rs:1260`).
- **Tests.** Every test that pins cruise height is a helicopter test. No pinned parity digest includes aircraft, but three parity fixtures embed the `air` rules block, and `AirRules` rejects unknown fields. The air lab scenes check today's behaviour, and their headers name the gaps.

## 2. Known unknowns: the decision ledger

### Height and motion
| # | Decision | Why | Closed by |
|---|---|---|---|
| D1 | Jets cruise at **250 m** above ground. | 1000 m doesn't read from an RTS camera. | user |
| D2 | Ground, low-air and high-air heights are **one set of constants** read by the sim, the ghosts and the shadow. | The ghost (1000 m) and the shadow (30 m) disagree with the sim today. | user |
| D20 | **Hard: a jet never goes below 100 m above ground.** The only exception is a destroyed jet falling. | Nothing on any map can be in its way. | user |
| D34 | A jet holds its height above the ground beneath it, smoothed, with no prop probes. | Nothing reaches 100 m. | agent, shown to the user |
| D3′ | A jet's speed is only ever **cruise or afterburner**, with about a **2 s ramp** between them. It never slows, stops or hovers. | A jet never stops in the air. | user |
| D9 | Jets cruise at **650 km/h** (about 3× helicopter speed); afterburner is 1.5× (about 975 km/h). | Fast but readable. | user |
| D4 / D10 | A goal is **circled at full turn rate**, raised to **72°/s** for a 5 s lap, so the radius is speed ÷ turn rate (about 143 m, or about 215 m on afterburner). The jet circles in the direction it is already turning. A move goal is a fixed point; an attack or dogfight circles the target's current position. There is no orbit-radius field. | The 5 s lap the user asked for. | user |
| D19 | **Jets skip the air grid.** The air rules split into a helicopter block (30 m cruise, 40 m ceiling) and a jet block (cruise, floor, run ceilings, dogfight band, afterburner ramp). The helicopter routing margin is the data value `hull_limits.low_air.half_length_m`; each aircraft is admitted against its own layer limit. Helicopter beats that move get re-recorded. | Nothing at 100 m or above is in a jet's way. | user |
| D32′ | **Every distance between two bodies is 3D, and the layer chooses the rule row.** Ground paths, circles, footprints and the bomb release solve stay 2D. Reach, ranking, dogfight break and radar detection move to 3D, and pinned ground digests are re-recorded. **Height never decides a layer:** the height thresholds (`weapons.rs:528`, `battle.rs:756`) are deleted, and an interceptor engages an incoming interceptable round through its `interception` property, whatever the round's height. | A 2D "in reach" with a 3D fire gate freezes a unit, and the layers already choose what can be targeted. | user |

### Attacks
| # | Decision | Why | Closed by |
|---|---|---|---|
| D14 | **An attack run** goes: line up on a line through the target → dive under the weapon's ceiling → release → pull out, climb back to cruise and come round for the next pass. | A run, not a hover. | user |
| D13 | Run heights are **ceilings, not targets**: about **130 m to strafe and 200 m to bomb**, with the 100 m floor below. Each pass randomises its height between the floor and the ceiling, using seeded randomness. A jet already below the ceiling doesn't climb. | Hollywood variety. | user |
| D30 | **The gun fires whenever the jet's nose is on a legal target within gun range.** There is no weapon-pairing rule. Nose-on missile passes and dogfights fire the gun on their own; a bombing pass flies past high and never does. Today's "no strafing while bombs remain" rule is deleted. | One rule. | user |
| D23 | **Any loadout works.** Limited-ammunition ground weapons choose the pass ahead of the unlimited gun; a pass flies under its chosen weapon's ceiling, and every other weapon that can engage during the pass fires too. | A data choice must never confuse the sim. | user |
| D15 | **Orders.** An explicit attack repeats passes until the target dies. Attack-move picks up new targets along the way. When the attack-move goal is reached or the target is dead, the jet **holds by circling where it is**. A holding jet starts no runs on its own. | Player orders win. | user |
| D15″ | A holding jet that is fired on **attacks the attacker**: the unit itself if its side can see it, otherwise its contact area (every shot is disclosed map-wide: `knowledge.rs:146`). Hold fire stops this. | The best knowledge the side has. | user, territory |
| D16 | **Weapons that fire on their own** (air-to-air and SEAD missiles) fire whenever an enemy is in range and in their field of view, during any order or run. | | user |
| D18 | **Mid-run:** under an explicit attack the jet keeps making passes whatever fires on it. On attack-move or holding it finishes the pass, then turns on its attacker: a dogfight against an aircraft, runs against a ground unit. A pass is never abandoned halfway, and a Move order breaks off at once. | | user |

### Weapons
| # | Decision | Why | Closed by |
|---|---|---|---|
| D6′ | **Bombs are unpowered.** They leave the jet carrying its velocity and fall under boosted gravity, about **3 s** from a 200 m pass, landing about 250 m ahead. The sim works out the release point. Bomb rows drop `range_m` and `speed_mps` and author `gravity_scale`. Cluster bombs scatter around the aim point. | It looks dropped; the jet is just past when the bomb hits. | user |
| D21 | **Every round fired or dropped from an aircraft starts with the aircraft's velocity.** Ground shooters are unchanged. | Physical, and needed for bombs and missiles. | user |
| D21b | New row **`air_ground_missile`**: ground target, anti-armour, guided while moving, about 2000 m range, its motor adds about 200 m/s on top of the jet's speed, 4 rounds. `air_bomb_guided` becomes the unguided iron bomb **`air_bomb`**, with a big single blast. | Guided precision versus unguided area. | user (missile), agent (iron bomb) |
| D12 / D17′ | Every missile that targets aircraft has a **290 m/s** top speed, authored per row, with lifetimes matched to range (about 9 s medium, about 18 s long). Anti-tank missiles stay slow. **No test pins a balance outcome;** an afterburner escaping a SAM is tuned by eye. | The user retunes it often. | user |
| D33 | **Proximity fuse.** A missile aimed at an aircraft detonates within its `blast_radius_m` of the target and steers at the centre of the target's box. The z part of the target's velocity is kept (fixes `weapons.rs:491`). | A turning jet must not be missed by a hair for no visible reason. | user |
| D29 | Air-to-air missiles can also target **low air** (helicopters). | | user |

### Dogfights
| # | Decision | Why | Closed by |
|---|---|---|---|
| D8′ | **One target per jet; links don't have to be mutual.** Firing on an aircraft makes it your dogfight target. It adopts you only if it has none. Others join by targeting someone already engaged. The 1200 m break becomes one number in the air rules. | Simple, and nobody is shut out. | user |
| D11 | **Organic look.** Lag pursuit, so jets chase where the target was. On a head-on merge both break away and come round. Height wanders within about 150–350 m, seeded per jet. | A turning fight, not a charge. | user |
| D29b | ~~**Dogfights form only between jets.** A jet attacks a helicopter with runs.~~ Superseded by D29c. | Helicopters fly below the floor. | user |
| D29c | **A dogfight forms only between aircraft of the same layer, and the pursuer flies fixed-wing.** Against a target in another layer, or on the ground, a fixed-wing aircraft attacks in passes; a rotor aircraft never dogfights, it hovers and shoots. Read from the layers, never from height. | The flight model is data apart from the layer; for today's catalog the outcome is D29b's. | user, 2026-10-15 |

### Arriving, leaving and the edge
| # | Decision | Why | Closed by |
|---|---|---|---|
| D35 | **Off-map is presentation only.** The sim spawns a bought jet at its **own back edge**, at cruise height and speed, heading in, after the existing off-map delay. The presentation shows that side about **10 s of approach** as an effect that arrives at the spawn point and tick: drawn over the rendered margin and backdrop, own side only, never sensed, targeted or killed. Deleted: the four-sides spawn, the invisible wait at the edge, and the mid-air snap. | The simplest architecture: the sim never holds an off-map position. | user (agent chose the shape under "choose the simplest") |
| D28′ | **Withdrawal** flies to the **nearest point on the jet's own back edge**. The sim refunds and retires it as it crosses, and the presentation carries it on for about 10 s as an effect. The fuel-return estimate uses the distance to that edge. The entry-centre retirement is deleted. | Leave by your own side, visibly. | user |
| D36 | **A fighting jet stays inside the square.** Circles, run line-ups and dogfight steering turn back before the edge. | No fighting off the map, and no circling half outside to dodge AA. | agent, shown to the user |
| D31 | **Rounds and downed jets that cross the edge just end.** The crash clamp (`crash.rs:111`) is deleted. | There's no enemy outside the square. | user |

### Roster and art
| # | Decision | Why | Closed by |
|---|---|---|---|
| D22′ | **Each faction fields at least one ground-missile jet and one bomb jet**, dedicated or multi-role; quirks are welcome. No shipped jet carries both bombs and ground missiles. | Faction character, roughly historical. | user |
| D24 | **Delete** the generic `*_fighter_aircraft` rows, `air_bomb_guided` (it becomes `air_bomb`), and the duplicate SEAD rows. Repoint the encounters, voices, icons and parity fixtures that name them. | One owner each. | user |
| D25 | **No jet is disabled when this spec ends:** all 19 planned fixed-wing rows are enabled (manifest entry removed, full row, catalog appearance, voice, icon). Drones are out of scope (D37). | | user |
| D37 | **Drones, for a later spec:** low air, fixed-wing, flying at about 100 m, which clears every roof (the tallest is about 60 m), so they need no building avoidance. The drone spec gives low-air fixed-wing its own cruise height and pass ceilings relative to its layer; until then the catalog refuses fixed-wing in low air, since low air has no fixed-wing cruise height, floor or ceilings. | The flight model and the layer are separate data; the pair needs its own heights before it flies. | user, 2026-10-15 |
| D26 | **A model-quality pass on every aircraft**, working from `assets/references/<family>/`. Generate references with the Duet CLI where they're thin (F-15EX, F-35A/B, Su-30SM2, Tornado IDS, J-20), following the generated-view contract in `packages/scene-assets/src/references.ts`. | User asked; this is the exception to "models are good enough". | user |
| L1 | **Loadouts** (below). | | agent on the user's behalf |

**L1 loadouts.** Answered by the agent on the user's behalf, applying D22′, history and the role menu. Prices are tuned in the closing balance pass.

| Faction | Fighter (gun + air missile) | Ground missile (+ gun; air missile where marked) | Bombs (+ gun; air missile where marked) | SEAD |
|---|---|---|---|---|
| US | F-15EX, F-22A | **A-10C**; F-16C (AAM), F/A-18E (AAM) | **F-15E** iron (AAM); F-35A iron (AAM); F-35B cluster (AAM) | EA-18G |
| Europe | Typhoon | **Tornado IDS**; Rafale C (AAM); F-16C (shared) | **Mirage 2000D** iron; Gripen E cluster (AAM) | Tornado ECR |
| Eastern | J-10C, Su-27SM3, Su-35S, MiG-31BM; J-20 (no gun) | **Su-25SM3**; Su-57 (AAM) | **Su-34** iron (AAM); MiG-29SMT cluster (AAM) | Su-30SM2 |

The user saw these preferences and kept answering questions themselves. The only delegation was the loadout table: Hollywood over exact, numbers as guesses, simple rules, player orders win, self-firing weapons always fire.

## 3. Unknown knowns: what was drawn out

- **The other player flies the enemy's jets.** No computer jet behaviour is needed. The encounter defender (`crates/sim/src/encounter/defender.rs`) buys nothing; its jets would follow D15″.
- **The camera isn't a blocker.** The walk's numbers stand until the user plays them.
- **No balance tests.** Tests pin mechanisms (a missile burns out at its lifetime, the afterburner ramps, the floor holds), never which unit wins. This is now in the agent's memory.
- **Aircraft get a real model pass,** unlike other units.
- **The behaviour mock** `throwaway/air-war-flight/flight-mock.html` sketches circling, bomb passes, missile passes with the gun, a dogfight a third jet joins, fly-in and withdrawal, and the SAM chase, with sliders. The user didn't flag anything in it. Its logic was checked in Node: bombs land within 2 m, the gun never fires on bomb passes, and it does fire on missile passes and in dogfights.

## 4. Unknown unknowns: landmines

| Finding | Evidence | Status |
|---|---|---|
| Off-map: rounds expire, orders are refused, ground height reads 0, fog doesn't publish, crashes are clamped. | `flight/mod.rs:1225`, `battle.rs:1676`, `air.rs:345`, `visibility.rs:85`, `crash.rs:111` | Decided by D35, D36 and D31: the sim never goes off the map. |
| At 290 m/s, today's 6 s lifetimes fall short of the authored ranges, so the launch solver refuses shots in range. | `flight/solve.rs:305` | Decided by D17′: lifetimes matched to range. Re-run weapon admission. |
| Missiles have no proximity fuse and chase the base of the box. | `damage.rs:255`, `battle.rs:3146` | Decided by D33. |
| The flight model can't turn at 72°/s or never stop: speed changes are capped, and turning rotates only the nose. | `air.rs:96, 305` | Sharp edge: jets need their own motion law (D3′, D10). |
| Withdrawal never completes for a jet that doesn't stop. | `battle.rs:980-993` | Decided by D28′. |
| Dispatch picks the entry by list index, but withdrawal searches by side. | `battle.rs:848` vs `find` | Sharp edge: goes away with D35/D28′. Keep one lookup. |
| Mixed 2D/3D distances freeze a unit. | `weapons.rs:1101, 1218, 1242`, `battle.rs:663, 3231`, `knowledge.rs:326` | Decided by D32′. |
| A layer is decided by absolute height compared against an above-ground threshold. | `weapons.rs:528`, `battle.rs:756` | Decided by D32′: deleted. |
| `AirRules` rejects unknown fields, and three parity fixtures embed it. | `scenario.rs:627`, `fixtures/parity/*.json` | Sharp edge: re-record the fixtures (recompute over compatibility). |
| The shadow and the ghost read their own heights; native capture passes `cruise_agl_m` through. | `presentation.rs:1535`, `unitGhosts.ts:23`, `native/client/src/capture.rs:190` | Decided by D2. |
| The deck limit is 10 families per faction per category, and US is already at 10 counting disabled rows. | `catalog.rs:1260` | Sharp edge: deleting the generic and SEAD rows frees slots. Check after D24/D25. |
| Every playable row needs a voice, and an icon that isn't stale. | `voiceCatalog.ts:167`, `asset check` | Sharp edge: part of D25. |
| The dogfight lab field is smaller than the 1200 m break distance. | `web/scenes/air-dogfight.mjs` header | Sharp edge: restage it. |
| The lab scenes check today's numbers (e.g. `CRUISE = cruise_agl_m`). | `web/scenes/_air.mjs:48` | Sharp edge: they're rewritten as the checkpoints for the new claims. No air screenshot baseline exists to break. |
| A stale comment says "sight runs on past the edge". | `worldPass.ts:39-43` | Sharp edge: fix it while there. |

**For the builder to confirm before coding:**
- How the off-map delay is published, and where an effect can hook in to draw the 10 s approach and departure (presentation crate, own side only).
- What `interception` names today, and which row an interceptor should use against a round once the height thresholds are gone (D32′).
- Whether any scripted encounter relies on the four-sides spawn (`fixtures/maps/air/encounters/*.json`).
- That every family script builds in `vehicle_export.run_disabled` before the model pass starts.

## Panel follow-ups (carried over)

See [the README](README.md#panel-follow-ups): the radar dish icon, the alignment of the UNKNOWN row, and the dogfight icon.
