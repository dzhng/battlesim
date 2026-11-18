# Supply helicopter

Each faction can buy a supply helicopter from the shop's LOG tab:
- **US:** CH-47F Supply.
- **Europe:** Chinook HC6 Supply.
- **Eastern:** Mi-8AMTSh Supply.

Each costs what the supply truck costs, carries half its stock, and keeps its
transport's door guns. When it deploys it lands, and its rotor winds down;
packing spins the rotor up and it lifts off. While on the ground it is a ground
vehicle, so tanks can kill it and AA can't. The skirmish AI keeps the truck as
its standing supplier and flies a helicopter in when a fighting group runs low
far from that truck.

The planned decisions and their reasons are in [`decisions.md`](decisions.md).
The choices made during the build where the plan was silent are in
[`choices.md`](choices.md), least certain first.

## Why it works this way

- **Landing is how an aircraft deploys.** There's no new "land" order. A unit
  type that flies and authors `capabilities.deploy.drop_seconds` sets up by
  landing, so the deploy and pack commands, the automatic deploy when a unit
  stops, and the supply rule work as they do for the truck: Stop or Deploy to
  a heli already set up leaves it set up. The drop counts as
  part of the 3 s set-up, so the timer stays exact and replays stay exact.
- **On the ground it's a ground vehicle, by state, not type.** The user chose
  the war-film reading: a parked helicopter is something a tank can shoot and
  a jeep has to drive round. "Landed" is a latch on the unit's deployment. It
  is set when the drop ends and cleared when packing reaches zero. Progress
  alone can't say it, because the same progress means mid-air while landing
  and on the ground while packing.
- **What a side knows follows what it saw.** Enemies are told how far a
  deploying unit looks set up, and whether an aircraft stands on the ground. This
  is the one deliberate exception to "never timers" in the observation.
  Enemy tracks re-file layer and target class at every sighting.
- **The client never decides physics.** Rotor speed, blur, marker and sound
  are worked out from the published `grounded`, set-up progress and the type's
  timings. The simulation never names an animation.
- **The AI's playbook reads mobility, not ids.** A supplier that drives is the
  standing supplier; a supplier that flies answers emergencies. With door guns
  and an equal price the helicopter would otherwise win the AI's ranking and
  become its truck.

## What must stay true

- **One owner per concept:**
  - "Lands": `UnitType::landing` (contract).
  - "On the ground": the deployment latch, read through `Unit::landed`.
  - "Flies" versus "in the air now": `Unit::flies` / `Unit::airborne`.
  - Target class of a unit: `Unit::target_class`.
  - Push class: the hull's, through `Unit::push_class`.
  - Nearest clear spot: `crash::nearest_clear_spot`, shared with wreck
    placement.
  - Rotor speed: `units::rotor::rotor_speed` in presentation.
  - "In the air now" on the client: `inTheAir` in
    `packages/scene-assets/src/units.ts`, and `UnitType::in_the_air` in
    `crates/presentation/src/world.rs`.
  - Standing and flying supplier: `skirmish_ai::purchase::standing_supplier` /
    `flying_supplier`.
- **State that decides the future is digested while it exists**: the landed
  latch, the drop's start height, the landing goal, pending opening wear, and
  each track's layer and class. Units that never land keep their digests.
- **A helicopter lands only on a spot checked where it came to rest.** The
  spot has a clear rotor disc within `air.rotor.landing_reach_m`, and is
  re-checked until touchdown. With no spot it holds packed aloft and shows NO
  LANDING ZONE. It moves sideways only once at the low hover, and the low hover
  clears every type that can land.
- **Killed while landed it leaves a ground wreck, with no crash blast.** Killed
  in its drop, it crashes.
- **No helicopter that never lands behaves, looks or sounds different.** It
  draws byte-identically and its rotor runs at full speed. Battle digests moved
  only through named changes that reach every battle: enemy tracks fold their
  layer and class, and `test_supply_heli` renumbers the test type indices. The
  wire gained `grounded` and an identified `deployment`, so publication and
  capture records were re-recorded.

## Where it lives

- **Rules:**
  - `crates/sim/src/deployment.rs` (`target`, `drop_progress`, `landed`,
    `separation_exempt`);
  - `crates/sim/src/movement/air.rs` (`prepare_landing`, `spot_blocked`,
    `landing_height`, `low_hover`);
  - `crates/sim/src/units.rs`;
  - `crates/sim/src/battle/casualties.rs` (the airborne crash branch) and
    `movement/drive.rs::death_roll` (its early return for a landed unit);
  - `crates/sim/src/knowledge.rs`.
- **Admission:** `UnitType::capability_problem` in
  `crates/contract/src/catalog/capabilities.rs`.
- **Wire:** `IdentifiedUnit.deployment`, `grounded`, `MoveState::NoLandingSpot`
  in `crates/contract/src/observation.rs`. Columns in
  `crates/sim/src/publication.rs` (own units' `grounded`),
  `publication/identified_row.rs` and `publication/deployment.rs`.
- **Presentation:**
  - `crates/presentation/src/units/rotor.rs`;
  - `units/driver/vehicle.rs`;
  - the sound emitter's rotor slot: written in `presentation/sound.rs`,
    decoded in `sound/frame/motion.rs`, applied in `sound/frame/loops.rs`.
- **Client:**
  - `packages/scene-assets/src/units.ts` (`inTheAir`), used by
    `apps/battle-lab/src/battleOverlay.ts`;
  - `web/src/battle/present/panelRows.ts` (NO LANDING ZONE);
  - `web/src/battle/present/purchaseFilter.ts` (`shopOrder`: ground movers
    before aircraft in a tab).
- **AI:**
  - `crates/sim/src/skirmish_ai/airlift.rs` (`emergency`, `far_side`,
    `respond`, `orders`);
  - `purchase.rs`, `support.rs`;
  - tuning in `fixtures/skirmish-ai.json`.
- **Data:**
  - roster rows extend an abstract `<transport>_airframe` in
    `fixtures/units/roster/*.json`;
  - `test_supply_heli` in `fixtures/units/test/aircraft.json`;
  - `air_light_logistics` in `fixtures/audio.json`.
- **Tests and labs:**
  - `crates/sim/tests/supply_heli.rs`;
  - `crates/sim/src/battle/airlift_tests.rs`;
  - `crates/sim/src/skirmish_ai/tests/airlift_tests.rs`;
  - `crates/presentation/tests/rotor_poses.rs`, `landing_picks.rs` and
    `sound_rotor.rs`;
  - `crates/sim/tests/publication_wire.rs`, `web/tests/purchaseFilter.test.ts`;
  - battle lab `/labs/supply?variant=heli|heli-fire|heli-ground|heli-shift|heli-crowded`,
    scene `web/scenes/supply.mjs`;
  - `/labs/supply-ai` with `web/scenes/supply-ai.mjs`.

## How the build diverged from the plan

- **Rotor speed ignores the deployment direction.** P12 had the client read
  the target too. Instead rotor speed follows grounded progress on one curve
  both ways, so a landing reversed mid-wind-down is continuous. The wire still
  carries the target, with the own unit's shape.
- **Skirmish openings gained authored wear (`condition`), and an AI seat with
  an authored opening now readies with it.** It wasn't planned. Skirmish units
  arrive only by purchase, so this is the only way a lab can start with a
  damaged group.
- **Over-budget files gave up whole concepts to make room.** The file-size
  limit forbids any growth of an over-budget file, so these moved out, with no
  behaviour change: wear, lanes and treads, the merge rules, publication rows,
  the doctrine, recon, the support post, the airlift, soldier threat, and the
  sound motion records.

## Dead ends

- **A "landed" test from height.** It needs world access and breaks on bridges
  and slopes; the latch replaced it.
- **Choosing the landing spot as soon as Stop is pressed.** At cruise the heli
  glides about 140 m before hovering, and landed on unchecked ground. The spot
  is now chosen at rest.
- **Showing the hop to the landing spot as an internal order.** A hidden goal
  switched on by the published move state needed three filters, and leaked
  through queued orders. It became plain flight code.
- **Counting the reserve heli as a supplier in the AI's reach check.** That
  check suppressed every nearby emergency. Re-aiming a refused far side at the
  unit in need never flew either; a refused far side now sends the heli home.
- **Pinning tuned numbers in tests**, such as two audio rows being equally
  loud. Each now reads its numbers from data.

## Known gaps

- The landing search treats a forest's bounding box as forest, and tests the
  ground at the spot's centre against the map's slope limit.
- On a bridge it would touch down at the ground below. No supply-heli map has
  one.
- The battle lab doesn't draw props added by an encounter's events, so the
  `heli-shift` and `heli-crowded` crates act but aren't visible.
- After a refused far side the AI may shuttle the heli between its post and the
  same spot while the need lasts.

## Visual provenance

There was no reference image. The standard was set by the user in the
planning walk, from two pages kept here:
- [`visualizations/map.html`](visualizations/map.html), the decision map;
- [`visualizations/landing-mock.html`](visualizations/landing-mock.html), the
  clickable side-view mock where the user chose a fast drop with a flare, the
  rotor slowing from touchdown, the truck's panel words, and the ground marker
  once landed.

Each visual step was judged by an unprimed screenshot critique against the
before shot. The approved pictures are the scene baselines:
- `web/scenes/baselines/panels/panels-own-states.png` (NO LANDING ZONE);
- `web/scenes/baselines/unit-roster/log-cards.png` and `log-supply-info.png`;
- `web/scenes/baselines/armory/supply-*.png`.
