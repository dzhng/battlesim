# Supply helicopter: decisions

The decisions this plan carries as givens. **User** rows were answered by the
user in the planning walk on 2026-11-17. **Agent (for user)** rows were answered
on the user's behalf under their confirmed preferences:
1. realism where it changes how the fight plays;
2. match the supply truck wherever there's an equivalent;
3. the cheapest option when it's only about looks;
4. otherwise the agent's recommendation.

**Plan** rows were settled while synthesising the plan from four independent
drafts. They follow the same preferences and record the alternative they beat.

The walk's map and the landing mock are kept in
[`visualizations/`](visualizations/).

## What it is

| # | Decision | Why | By |
|---|---|---|---|
| R1 | One supply helicopter per faction: CH-47F (us), Chinook HC6 (europe), Mi-8AMTSh (eastern). Each costs 150 (the truck's price), carries supply stock 500 (half the truck's 1000), deploys in 3 s and packs in 1 s (the truck's times), has roster category `sup` and role `logistics`, and so shows in the shop's LOG tab. | The user's request. The Chinook is the helicopter people picture carrying supplies; each faction already has these transports. | User |
| G | It keeps its transport's door guns as working weapons and reuses the transport's model unchanged. It has no passenger seats. | The guns are built into the mesh, so a gun-free look would need re-exporting three models. Door gunners on a supply Chinook read as war film. | User |
| D8 | Card names "CH-47F Supply", "Chinook HC6 Supply", "Mi-8AMTSh Supply". | They fit the 18-character label cap (`crates/contract/src/labels.rs`). | Agent (for user) |
| D9 | Voices: each faction's transport-pilot set. | The supply driver's lines say "truck". No new recordings. | Agent (for user) |

## How it lands

| # | Decision | Why | By |
|---|---|---|---|
| Q1 | While landed it is a **ground vehicle**: layer Ground, target class Vehicle (tanks and ATGMs engage it, AA can't), a ground footprint (ground units route around it), and ground cover and concealment apply. | What a war-film viewer expects. | User |
| Q2 | Deploy takes 3 s. It drops for the first ~2 s (fast, with an ease-out flare at the bottom). At touchdown it becomes a ground vehicle; the rotor spins down over the rest, and it serves at 3 s. Pack takes 1 s: the rotor spins up on the ground, then it is an aircraft again and climbs. | Keeps the truck's times; the deploy timer stays exact, so replays stay exact. | User |
| Q3 | It lands on the nearest clear spot within 40 m. If no spot is clear, it stays packed in the air and its panel says why. Moving to the spot is ordinary packed flight, and the deploy timer starts once it is stationary over the spot. | What a pilot does. 40 m is half the supply radius, so it still serves roughly where it was sent. | User |
| Q4 | It deploys by itself whenever it stops, like the truck. Pack keeps it in the air. | One deploy rule for every unit. | User |
| D1 | Killed while landed, it leaves a ground wreck with no crash blast. Killed during the drop, it crashes as an aircraft. | Landed means ground vehicle; a crash blast would hit the units it was supplying. | Agent (for user) |
| D2 | Told to stop in mid-flight, it brakes to a hover first; the deploy timer starts only once it is stationary over its spot. | It glides ~140 m from cruise; landing while sliding would land in the wrong place. | Agent (for user) |
| D3 | Its height follows the deploy timer only while landing (bypassing the climb-rate clamp, with its vertical velocity equal to the forced sink, because shots lead with it). Packing holds it on the ground. Once packed it climbs at its normal rate and moves sideways only once clear (see P10). Packed mid-drop, it resumes normal flight from its current height. | No 45 m/s jumps, no flying at ground level. | Agent (for user) |
| D4 | Each helicopter that can land authors a rotor radius; the clear-spot search uses it. | Values live in data (`development.md`). | Agent (for user) |
| D5 | The low hover (the height helis hold over a supplier) clears every type that can land, not only ground-layer hulls. | Today a landed Chinook clears that height by 0.7 m only because a Pantsir happens to be the tallest ground hull. | Agent (for user) |
| D6 | A landed helicopter takes no part in air separation, as pusher or pushed. | Separation would slide it along the ground; a supplier that moved stops serving. | Agent (for user) |

## What others see

| # | Decision | Why | By |
|---|---|---|---|
| Q6 | The rotor slows with the deploy state, from touchdown; the blur hands off to still blades; the rotor sound fades with rotor speed down to silence. It gets its own audio row (class `air_light_logistics`). The client works this out; the simulation never names an animation. | A winding-down rotor is the moment it becomes vulnerable. | User |
| Q7 | Enemies that have identified a deployable unit see how far set up it looks (a new `IdentifiedUnit.deployment`), a named exception to the observation rule "never timers". Enemy knowledge tracks take layer and class from what is seen at each sighting. | A heli landing is as visible as a vehicle reversing. One general rule also covers a truck or an ATGM team setting up. | User |
| P12 | The shape of Q7's field is the own unit's `DeploymentState` (progress and target), plus a `grounded` flag on own and identified units. See P12 below. | Amends Q7's `Option<f64>`. | Plan |
| T1 | The drop is fast with a flare at the bottom; the rotor starts slowing at touchdown; panel words are the truck's (DEPLOYING / SUPPLYING / PACKING); once landed it wears the ground-vehicle marker with no drop line. | Picked in the landing mock. | User |

## Shop, tutorial, AI

| # | Decision | Why | By |
|---|---|---|---|
| D7 | The truck stays the first LOG tile; the heli follows. | The truck is the main supplier. | Agent (for user) |
| D10 | The tutorial's LOG line reads "supply trucks and helicopters". | The tutorial calls LOG "supply trucks". | Agent (for user) |
| Q5 | The skirmish AI buys the heli only for emergencies. A unit in a combat group is below half of any finite ammo or below half health, and no supplier can reach it within 30 s (AI tuning values). The heli flies to the group, lands on the side away from known enemies, serves, then returns to the truck's support spot. The truck stays the AI's standing supplier. | It's for frontline emergencies where supply is needed fast. | User |
| D11 | The AI's opening buy, truck replacement and truck parking consider only supply units that drive, decided by mobility, not id. The flying one is emergency-only. Tests assert which card was bought. | With guns it outranks the truck on value, and at equal cost the id tie-break picks it. | Agent (for user) |

## Settled while planning

Each was raised by at least one draft as under-specified.

| # | Decision | Alternative it beat, and why |
|---|---|---|
| P1 | **"Landed" is a latched flag on the unit's deployment** (`Deployment`), owned by `deployment.rs`. It is set on the tick the drop completes while deploying, and cleared when packing reaches 0. `Unit::landed()` reads it. It is folded into the unit digest only while set, so no truck digest moves. | A pure function of `deployment.current` can't work: the same progress means mid-air while deploying but on the ground while packing. Reading "height at ground level" needs world access and is fragile on bridges and slopes. |
| P2 | **The drop time is authored on the unit**: `capabilities.deploy.drop_seconds` (roster value 2). The catalog requires it on an aircraft that deploys, keeps it below `seconds`, and refuses it on a ground unit. | A rules-wide value or one derived from height breaks "values are authored on the unit" (`development.md`). |
| P3 | **Two predicates, one meaning each.** `Unit::flies()` (motion is air: flight law, routing, placement, orders) and `Unit::airborne()` (flies and not landed: no footprint, air layer, no ground concealment, air separation). Every current `airborne()` caller is reassigned to one of them. | Keeping one `airborne()` makes the ground-movement code crash on a landed heli (`Unit::ground()` refuses an aircraft). |
| P4 | **Ground rules never ask a landed heli for ground movement.** Sites reached through its footprint read hull properties through one owner (`Unit::push_class()`); the death roll returns early for a landed unit. A landed heli is never shoved and knocks down no trees (push class `none`, as authored). | Inventing a ground `Mobility` for an aircraft would make every vehicle `expect` reachable. |
| P5 | **Target class is per unit**: `Unit::target_class(rules)` returns Vehicle while landed and the type's class otherwise. Weapons and knowledge read it. The skirmish AI's purchase planning keeps reading the type's class. | Planning is a playbook about unit types, not a rule. |
| P6 | **`rotor_radius_m` lives on air mobility and is required only on an aircraft that deploys.** It is the radius of the circle the rotors sweep about the hull centre, the whole tandem for a Chinook. The landing clearance is that disc; the landed footprint stays the hull. | Required on every rotor aircraft: data nothing reads. Per-rotor discs: more data for a few metres. |
| P7 | **The 40 m reach is `air.rotor.landing_reach_m`** in `fixtures/rules.json`. | Next to the other rotor flight rules. |
| P8 | **The clear-spot search is one deterministic nearest-first ring search**, generalised from the crash wreck's `crash::resting_place`, so both use one owner. It reads the true world, because the heli is looking at the ground under itself. A spot is blocked by: any body footprint that blocks a vehicle, forest ground, ground a wheeled vehicle can't stand on, live ground hulls and soldiers, other landed aircraft, and the off-map margin. The spot is re-checked every tick until touchdown; if it becomes blocked, the heli reverses toward packed and searches again from where it is. | Searching the side's belief would land it on an unseen tank. Landing anyway onto a unit that drove under it breaks traffic. |
| P9 | **"Can't land" is a move state**: `MoveState::NoLandingSpot`. The panel row reads NO LANDING ZONE: a warn-tone row like ROUTE BLOCKED, with the deploy mark as its icon (its own look, so the two problems don't read alike). It holds while the heli wants to deploy, is packed, and has no clear spot. | A separate deployment field would be a second place to look for one state. |
| P10 | **After packing, it moves sideways only once it has climbed to the low hover over its ground.** That clears every ground unit; the height target still lifts it over props on the way. | Cruise height: needlessly slow. No rule: it flies off at ground level. |
| P11 | **An explicit Deploy order in flight acts as Stop**: brake, search, land. | Follows Q4 and D2. |
| P12 | **The publication shape for Q7.** `IdentifiedUnit.deployment: Option<DeploymentState>` uses the own unit's type (progress and target). Own and identified units also carry `grounded: bool`: an aircraft standing on the ground, which is visible. The client derives rotor speed from (grounded, progress, target, the type's `drop_seconds`) in one helper. | Q7's bare `Option<f64>` can't tell spinning up from spinning down, and without `grounded` the client would have to re-derive the P1 latch, which needs history. |
| P13 | **LOG order is a rule**: within a shop tab, cards whose type moves on the ground sort before aircraft. Only LOG mixes the two today. | An authored sort key: data for one tab. |
| P14 | **Rows extend a common parent.** Each transport row becomes an abstract `<id>_airframe` holding everything but `carriage`. The transport and the supply row both extend it. The resolved transport rows stay byte-identical in `fixtures/catalog.json`. | `carriage: null` breaks the fixtures README rule that a variant can't drop an inherited key (`fixtures/README.md:158`). |
| P15 | **The supply rows reuse the transport's appearance id.** The model exporter (`catalog_frames.py::_variant`) accepts several unit types drawing one appearance when they agree on hull and mounts, and refuses otherwise. Card icons are rendered per card by the asset CLI from the baked model; this needs the runtime bundles pulled from LFS. | A second appearance with the same source: the exporter would write the same file twice and its paint scheme is keyed by unit id. |
| P16 | **The arrival delay is the transport's**: neither row authors one. | Same airframe; Q5's 30 s threshold already makes the AI buy early. |
| P17 | **AI emergency details.** One flying supplier at a time: bought only if the side owns none. After serving it returns to the support spot, where it deploys as a reserve (Q4), and it is re-sent for the next emergency. A combat group is the AI's existing party grouping. "Below half of any finite ammo" counts each finite weapon kind on any mount. Squad health is living members against squad size; vehicle health is hp against max hp. Reach is each own supplier's route time to the unit, setup excluded. The far side is a point half the supply radius from the group's centroid, away from the centroid of the side's known enemy tracks, or toward its own entry if none are known; the clear-spot search does the rest. If engaged on the way in, it carries on (it has door guns). | Several emergency helis at once: a credit sink the user didn't ask for. |
| P18 | **The test unit `test_supply_heli`** extends `test_heli` with deploy 3 s / 1 s / drop 2 s, half of `test_supply`'s stock, and a rotor radius. Its hull is no taller than `test_supply`'s, so the test catalog's low hover doesn't move. | Giving `test_heli` new capabilities would move every digest that fields it (`fixtures/README.md`). |
| P19 | **A damaged landed heli keeps trailing smoke**: `Unit::smoking` keys on `flies()`. | Damage smoke is what a viewer sees of damage; landing doesn't put it out. |
| P20 | **Enemy trucks' and ATGM teams' set-up becomes visible too**, by Q7's one general rule. | Accepted by the user with Q7. |
| P22 | **Air separation also skips a heli whose deploy timer is running**, not only a landed one. | Drift during the drop would move its touchdown spot and count as moving. |
| P23 | **Rifles and HMGs plink at a landed heli.** It is a Vehicle, and unlimited-ammo guns fire at what they can't hurt (`fires_regardless`). | Intended; harmless plinking is cinematic. |
| P24 | **A landed supply heli is never served by a truck**, and it serves helis that hover over it. Both follow from the existing supply rules (a supplier is never a recipient) with D5's low hover. | No new rule. |
| P21 | **No backward compatibility and no migrations.** Hard cutover: parity, capture and corpus fixtures are re-recorded whole; no fallback decoder. | The user's choice. |

## Options the user turned down

- **Q1:** a landed heli that stays an aircraft (only AA and small arms hit it); a half rule where ground units collide with it but it's still targeted as an aircraft.
- **Q2:** the descent on top of the 3 s deploy (about 8 s before it serves); ground vehicle only once fully deployed.
- **Q3:** refusing to deploy when not clear; landing on top of obstacles.
- **Q4:** hover when stopped and land only on an explicit Deploy.
- **Q5:** ranking suppliers by stock per credit (the AI never buys the heli); a heli that always follows the fighting group; buying it only when the truck is dead.
- **Q6:** a low idle whine on the ground; instant rotor cut and start.
- **Q7:** a rotor-only field for helicopters; sending enemies nothing new.
- **G:** guns drawn but never firing; a gun-free model re-export.
- **Look:** an open ramp with pallets, or a slung cargo net; heli-specific panel words (LANDING / LANDED / LIFTING); keeping the heli marker on the ground.

## Non-goals

- No new art or model export.
- No landing for any other helicopter.
- No deploy animation for the real trucks.
- No retune of the truck.
