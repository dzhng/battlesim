# Map: flight model, aircraft arrival, closeout (2026-10-15)

The four-quadrant map from an explore-unknowns walk with the user. [Slice 20](slices/20-aircraft-arrival.md) builds it, and slice 19 closes the spec. D-numbers continue [the main map](map.md).

## Known knowns
- **High air is an elevation layer.** Layer treatments key on the layer:
  - it can't see the ground;
  - its shadow lies beneath it;
  - its cards draw on top;
  - its cruise height is 250 m, with a 100 m floor.

  See the README invariants.
- **The flight model is its own data:** `mobility.air.flight`, either `rotor` or `fixed_wing` (branch `awf/flight-model`). Wing behaviour keys on it:
  - which flight law steps the unit;
  - arriving and leaving by the back edge;
  - attack passes;
  - nose-on fixed guns;
  - banking;
  - the destination circle.
- **Only rotor aircraft get a ground marker and drop line** (`aircraftMarks.test.ts`).
- **The back edge** is the map edge nearest the side's ground entry site (`crates/sim/src/air/edge.rs:14`).
- **Today's arrival wait** is a constant in code: 15–30 s, seeded, high air only (`crates/sim/src/battle.rs`, `dispatch_reinforcements`).
- **Enabled jets already have pilot voices** (`fixtures/voices.json`). Enabling the planned jets means assigning existing sets.
- **The tallest roofs are about 60 m.** The 20-floor towers top out at 57 m (`fixtures/prototype-building-templates.json`).

## Known unknowns: decision ledger
| # | Decision | Why | Closed by |
|---|---|---|---|
| D29c | A dogfight forms only between **two fixed-wing aircraft in the same layer**. Any other target, including a rotor aircraft in the pursuer's own layer, gets attack passes: line up, dive, fire nose-on, pull out, come around. Rotors never dogfight. | A turning fight needs two turning aircraft. Against a hovering target the pursuer would only circle a point. | user (A) |
| D37 | Drones are **low air, fixed-wing, at about 100 m**, with no building avoidance. Low-air fixed-wing stays refused at validation until the drone spec gives low air its fixed-wing floor and pass ceilings. | 100 m clears every roof. Today's pass and dogfight heights are high-air values. | user, territory |
| D38 | **Arrival wait:** `mobility.air.arrival.delay_s: [min, max]`, optional on any aircraft. It is seeded from the battle. When absent, the aircraft appears at the edge at once. Jets keep 15–30 s through their profile; helicopters have none. | Data over constants, and usable by any aircraft. | user (A) |
| D39 | **Every aircraft spawns at the back-edge point nearest its side's ground entry site** (the highway), already in flight at its cruise height, then flies to its destination. The approach drawing uses the same point. | Arrivals come from your own side's entry, not from wherever the destination happens to be. | user |
| D40 | **Per-unit cruise height:** any air unit may set `mobility.air.cruise_agl_m`; otherwise it takes its layer's height. A single lookup answers "how high does this unit cruise". Strange values (low air above high air) must behave without crashing. High air's 100 m floor stays hard: a lower override flies at the floor. | An override to a default. Layer rules never read height. | user |
| D41 | **A waiting aircraft leaves the purchase queue** and waits on its own clock. Purchases behind it keep spawning, and several aircraft wait in parallel. | Today the queue front blocks everything behind it (landmine 1). | user (1A) |
| D42 | **The pending card counts down:** "Arriving in N s", from an arrival tick published on the pending-purchase record, in the existing card style. The approach is still drawn over the edge. | The countdown didn't exist (landmine 2). | user (2A) |
| D43 | **`test_jet` gets its own model:** a generic, unmarked, neutral-grey fighter from the shared aircraft parts (single fin, swept wing, one nozzle). | It shouldn't read as a real roster jet. | user (A) |
| D44 | **Closeout proof stays lean.** Extend `every_loadout_flies_a_pass` so every fixed-wing row also arrives and withdraws on low fuel; no new test file or scene. Record three AI-vs-AI skirmishes, one per faction pair, as GIFs in the scratch folder. | The user just deleted duplicate verify scenes. | user (A) |
| D45 | **The AI follows the same arrival rules as a player.** No special timing logic. | One rule for both. | user |
| D46 | **"Done"** is slice 19 as written, ending with the full `verify` run. | | user |
| D47 | **An emitting radar is the unit's high-air sight.** `capabilities.radar.detection_m` is how far it sees high air, and an emitter never also authors `sensors.high_air_m`. Every emitter's reach was set to its old high-air sight; the Gepard's search radar gives it 3,200 m. | Emitting is what a radar costs (SEAD jets find it); seeing far is what it buys. | user |
| D48 | **Without radar, a unit sees high air by its body.** The user (2026-10-15): "most units without radar should have very low high air sight, like 300m. Tanks should actually have high air sight of 0. Inf can have 600m." Amended: helicopters see 600 m too. High-air sight is an emitting radar's `detection_m`, else the unit's own `sensors.high_air_m` (optional: absent is unaided, 0 is blind), else the rules' `sensors.unaided_high_air_m` by body and flight: `on_foot` 600, `aircraft` 600, `ground_vehicle` 300. Tanks author 0 (main battle tanks, the three light tanks, the wheeled tank destroyer); jets author 3,000 m (down from 5,000, user correction) and the optical short-range AA 900 m. The ground-sight fallback is gone; low-air sight still falls back to ground sight. | Soldiers on foot and aircrew look up; a crew under armour sees a jet only nearly overhead, and a closed-up tank crew not at all. At 250 m cruise, 600 m reaches about 545 m across the ground and 300 m about 165 m. | user |
| D49 | **SEAD radars reach 5 km**, down from 7 km: `capabilities.air.radar.detection_m` is 5,000 on every SEAD aircraft (the Su-30SM2, the Tornado ECR and the EA-18G). Jet-to-jet high-air sight is 3 km (D48). | At 7 km a SEAD jet's radar covered the whole map. | user |
| D52 | **There is no air-defence tier or flag.** `capabilities.air_defense` (and its `tier`), `capabilities.air.aa_tier` and the weapon rows' `aa_tier` are gone. An air-defence unit is a unit whose weapons reach the air; how short- or long-range it is follows from its rows' `targets.<layer>.range_m`. What the flag gated is now physical, or gone: any ground mount carrying a `can_intercept` row arms it against an incoming round; radar emitters that SEAD finds are any ground unit whose radar emits (`radar` owns radar detection and emission); and the rule that an AA crew needs its own view of an air target is deleted, not generalised. A guided missile already needs its operator's line to its target at any layer (the guided `guidance_clear` check and the guidance hold); a ballistic gun firing at an aircraft is direct fire like any other. Launching from under cover is a launcher-side, per-weapon check (`requires_clear_launch_path`) left for a follow-up. Weapon-row validation loses its tier rules and keeps the physical one: a seeker `can_intercept` row must be guided. The weapon field `interception` is renamed `can_intercept`, a hard cutover with no alias ([rpg-weapon-contract](../rpg-weapon-contract/) slice 1, landed early here). Every faction fielding short-, medium- and long-range AA is a roster guideline, shown by the air-roster lab's reach column, not a test. | The user (2026-10-15): "there shouldn't be an explicit 'tier', the only tier is how their stats are different… What mechanic does it actually provide?" The sim never read the tier for behaviour, only for validation, one catalog test and the labs. An air-only sight rule is a named special case; the user asked for none: "no special case for targeting air units at all". | user |

**Settled unless the user objects:**
- At the edge, an aircraft heads toward its destination if that points into the map; otherwise it heads straight in.
- Withdrawal is unchanged: jets leave by the back-edge point nearest them, and helicopters fly home to base.
- `flight` is required, with no fallback from the layer.

## Unknown knowns: what the walk extracted
- **Drones are the coming consumer** of the flight-model split. The user expects low air to hold rotor and fixed-wing units side by side, and per-unit heights.
- **One rule for AI and humans.** Never a special case for the AI.
- **Fewer, bigger proofs.** Verify scripts were just pruned for duplication, so proof goes into existing tests.

## Unknown unknowns: landmines
1. **Decided (D41).** A waiting jet blocks every purchase behind it. `dispatch_reinforcements` only handles `reservations[side].front_mut()`.
2. **Decided (D42).** No countdown exists. The pending card says only "Will be deployed" (`web/src/battle/present/armyDeck.tsx`), and `PENDING_FIELDS` carries no arrival time.
3. **Sharp edge.** The per-unit height must reach every caller:
   - sim: the flight law, the spawn and approach height;
   - presentation;
   - web: the `ghostAloft` order preview.

   Any caller left asking by layer draws an overridden unit at the wrong height.
4. **Sharp edge.** `air-roster` timed out (60 s) under overlapped verify and passes alone (50 s), and enabling the planned jets makes it heavier. Measure it after enabling; mark it `serial` if it's still over.
5. **Merge order.** The flight-model branch, with D29c applied, merges before slice 20 starts.

## Confirm before coding
- The skirmish AI's purchase logic doesn't assume purchases spawn in order. Check `crates/sim/src/skirmish_ai`.
- The approach record (`AirTransit`) and the pending record can carry the same arrival tick, so the card and the drawing never disagree.
