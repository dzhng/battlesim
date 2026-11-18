# Supply helicopter: choices ledger

Choices made where the spec was silent, appended per pass. Each is consolidated
against the shipped code at closeout (slice 11).

## Slice 01

| Choice | Verdict |
|---|---|
| "Drives" means the type has ground locomotion (`mobility.locomotion().is_some()`), the one predicate in `skirmish_ai/purchase.rs`. | Sound: it reads the physical property, and every ground mover has locomotion. |
| The drives filter sits inside `pick` for the `logistics` role, so the opening recipes and the truck replacement both get it without each caller filtering. | Sound for now. The role is the doctrine's own word for its standing supplier. Slice 09 selects the flying supplier with its own filter and must not route through `pick("logistics")`. |
| The "already have a supplier" check counts only owned or pending suppliers that drive, so an emergency heli never stands in for a lost truck. | Sound; matches D11. |
| Tests patch `test_heli` inside the test deck into a flying supplier instead of adding a fixture row. | Sound; the slice asked for an in-test patch until slice 02's row exists. |

## Slice 02

| Choice | Verdict |
|---|---|
| `drop_duration` is stored on the deployment's tick lattice (drop ticks × deploy step), so it compares directly with `current`. Codex first stored it in plain ticks, which mismatched the lattice; fixed in integration. | Sound; one unit for progress. |
| The landed latch is set on the tick progress reaches the drop's end while deploying, and cleared at progress 0 while packing. Codex first set it at full deployment; fixed in integration to match Q2 (ground vehicle from touchdown). | Sound; pinned by `its_height_falls_with_setup_and_it_touches_down_before_it_serves`. |
| An aircraft "lands" when its deployment has a drop (`drop_duration > 0`); no separate flag. | Sound; one source of truth. |
| The height law starts from the height held when the deploy timer first moved (`landing_start_z`, folded into the unit digest while set) and eases out quadratically to the ground. | Sound; the start height is state that decides the future, so it is digested. |
| While packed with a movement goal and below the low hover, it climbs straight up before moving sideways (P10). | Sound. |
| Adding `test_supply_heli` renumbers the test catalog's type indices. Three paired-record fixtures (`scenario-overrides`, `publication/stream`, `publication/contact-lifecycle`) were re-blessed. Their digests first diverge at the first sighting, because contacts fold the type index. Codex had instead added a filter that dropped the new row from those tests' catalogs, which is a compatibility shim; it was removed. | Sound; a named re-bless, no outcome change. |
| The lab lists each supplier by its type's name, not by a kind-id string. | Sound; no presentation keyed on an id. |
| The heli variant's camera frames the landing point and the recipients at 150 m. | Reversible; judged from the scene frames. |

## Slice 03

| Choice | Verdict |
|---|---|
| Carriage keeps `airborne()` for its loading-height test; a landed carrier would board as a ground carrier. No supply heli carries, so this never fires today. | Sound; the honest reading of "in the air now". |
| `Unit::push_class(rules)` reads the hull's authored push class for every unit, ground units included, so tree-felling and pressed bodies read one owner. It is the same value `Mobility.push` held, so digests are unchanged. | Sound. |
| Slice 01's in-test flying supplier gained `drop_seconds`, since slice 02's catalog rule requires one. Slice 02 had left that test red, and this pass caught it. | Sound; process note: run the sim lib tests after a catalog rule change. |
| The `heli-fire` encounter overrides `test_supply_heli`'s hull hp to 5000, a lab-only durable airframe, so the demo shows both phases without the tank killing it first. | Sound; lab staging, not game data. |
| Scene checks rewritten to assert the behaviour. Codex's versions would pass even with the heli dead, or without the jeep passing. Now: the tank's main gun stops after lift-off while the AA team fires; the jeep passes without its centre entering the heli's hull. | Sound. |
| The `heli-fire` and `heli-ground` variants use the truck's wide camera. | Reversible. |

## Slice 04

| Choice | Verdict |
|---|---|
| One search, `crash::nearest_clear_spot(world, at, reach, clearance, blocks)`, rings at 1 m steps. The crash wreck's `resting_place` and the landing both call it; wreck placement is byte-identical. | Sound; one owner (P8). |
| The landing spot is an internal `Unit::landing_goal`. `movement_goal()` returns it only while the unit moves toward it, and the owner's view hides it (`ordered_goal()`, `ordered_route()`), so it shows no order line or queue entry. It is folded into the unit digest only while set. Codex first folded an empty marker for every unit, which moved every battle digest; fixed. | Sound. |
| While it holds packed with no spot, the search re-runs each tick; the cost is bounded by the 40 m reach at 1 m steps. | Provisional: fine at the lab's scale; revisit if a battle fields many hovering supply helis. |
| The admission rule reads mobility directly: an aircraft with deploy needs `drop_seconds` and a positive `rotor_radius_m`; nothing that drives authors a drop. | Sound. |
| NO LANDING ZONE uses the deploy mark as its icon, in the warn tone, not ROUTE BLOCKED's mark: the unprimed critique found the two read alike. Decision P9 is updated. | Sound; one look per concept. |
| The paired records that embed the rules gained `air.rotor.landing_reach_m`. Their digests are unchanged. | Sound; named. |
| The battle lab doesn't draw props added by an encounter's events (props are built once, as static world, from the map). The `heli-shift` crate and the `heli-crowded` crate grid act in the simulation but can't be seen. This predates the feature. | Known lab limitation. The panel workbench judges the slice's visual variable, the row. |
| The scene closes each heli section's page. Seven open WebGPU pages stalled the seventh's readiness. | Sound. |

## Slice 05

| Choice | Verdict |
|---|---|
| Tracks refresh layer and class at each continuing sighting; a last-seen contact takes the track's layer. Both are folded into the knowledge digest. Proof: with the fold removed, the whole sim crate passed unchanged. Four sim paired records were re-blessed for it in the commit. The capture recordings, checked outside the sim test binary, were re-recorded with the publication change; their digests first move at the first sighting. | Sound; named. Process note: after a digest change, also run the presentation and web capture tests. |
| The wire columns: own rows add `grounded` after `deployTarget`; identified rows add `deployProgress`, `deployTarget` and `grounded` before the limbs. `IdentifiedUnit.deployment` uses the own unit's `DeploymentState` (P12). | Sound. Publication records changed only their layout hashes; battle digests are unchanged. |
| `knowledge.rs` fills the identified unit's `deployment` and `grounded` where it builds the row, not a patch afterwards in `battle.rs`. Codex first patched them in battle.rs; moved. | Sound; one owner. |
| Room for the ratchet: battle.rs `wear` moved to `battle/spawn.rs`; identified-row packing to `publication/identified_row.rs`; its decoding to `publication/reader/identified.rs`; the wire test to `tests/publication_wire.rs`; a shared AI test fixture to `skirmish_ai/tests/observed.rs`. | Sound; pure moves by concept. |
| Cook-offs and shooter dust read the published `grounded`, not the type. | Sound. |

## Slice 06

| Choice | Verdict |
|---|---|
| One calculator, `units::rotor::rotor_speed(grounded, progress, target, drop_seconds, seconds)`, smoothstep-eased. Full while airborne; after touchdown it eases from 1 to 0 by full set-up; while packing it eases from 0 to 1 over the pack. A parked aircraft with no deployment is stopped. | Sound; one owner (P12). |
| The articulation field `rotor_blur` (once a 0/1 switch) became `rotor_speed` (0 to 1). It was renamed in Rust and the TS mirror, and the key in `fixtures/parity/capture/poses/paris-corner-poses.json` follows. The effect style named `rotor_blur` is a different concept and keeps its name. | Sound; the name says what it holds. |
| Blur hands over to the authored blades at speed 0.2 (`BLUR_CROSSOVER`, shared by Rust and TS), chosen from a probe strip. The disc's opacity and minimum chord fall with speed. | Reversible; tune by eye in motion. |
| `airborne` is recomputed every frame from the published `grounded`; the flight attitude state resets on take-off; helicopter gear never stows. `FeedUnit::grounded` now carries the published landing latch, and the showcase's parked aircraft is its zero-speed case. | Sound. The first-sight freeze is deleted, not wrapped. |
| Critique triage. A paused still draws the blur as sharp blades: an existing property of stills. The blades' shadow appears only once the geometry draws, and at zoom it breaks into soft blobs (shadow sampling of thin blades, as the showcase's parked aircraft already shows). Accepted at normal size. | Known; low priority. |

## Slice 07

| Choice | Verdict |
|---|---|
| One lab/client predicate, `inTheAir(units, u)` in `battleOverlay.ts` (a flying type not published as grounded), drives own views, always-shown aircraft marks, enemy aircraft marks and enemy click rings. Codex first repeated the expression at each site; folded. | Sound; one owner. |
| The production game and the lab share these overlay builders (`BattleView → useBattleOverlay → battleOverlayParts`), so both follow. | Sound. |
| The presentation pick flag `aloft` reads the published `grounded`. Type-level "is an aircraft" sites stay type-keyed: the catalog, purchase ghosts, drivetrain and idle-loop selection. | Sound; classified by Codex, spot-checked. |
| The scene helper `overlaysOnly` moved from air-hover to `_lab.mjs` so supply can count drop lines; supply measures one before touchdown and none after. | Sound. |

## Slice 08

| Choice | Verdict |
|---|---|
| The emitter's spare `motion[3]` carries the pose's `Articulation::rotor_speed`, or -1 for an engine that isn't a rotor. It passes through the native and scripted (wasm/TS) sound paths; there is no second calculator. | Sound; one owner. |
| The engine loop's gain and playback rate scale by rotor speed: silent when stopped, unchanged at full speed. Jets and ground vehicles are untouched. | Sound. |
| `air_light_logistics` copies `air_light`. The vehicle-class test now requires a row for every class any catalog derives, and drops the old list pinning rows by weight. That was configuration, and the generated guard covers what is fielded. | Sound; tests mechanics, not data. |
| Motion records and emitter decoding moved together into `sound/frame/motion.rs` for the ratchet. | Sound; a concept move. |

## Slice 09

| Choice | Verdict |
|---|---|
| `skirmish_ai/airlift.rs` owns the emergency: combat groups (the team planner's detached parties plus the main group), the trigger, `far_side`, and the mission (idle, inbound, serving, returning). Purchase calls it before the role loop; its orders come after the support post's. | Sound; one owner (P17). |
| Tuning keys `emergency_ammo_share` 0.5, `emergency_health_share` 0.5 and `emergency_reach_s` 30 live in `fixtures/skirmish-ai.json`. | Sound; data. |
| **Contract addition:** a skirmish `OpeningPurchase`, and the `PendingPurchase` it becomes, may carry a `condition` (authored starting wear), applied when the unit enters. Skirmish units arrive only through purchases, so a lab can start a damaged group only this way, as encounters already can with `UnitSetup.condition`. | Sound; small and general. Not in the plan; named here. |
| **Behaviour change:** an AI seat with an authored opening now sends `Ready` with its opening batch, as any seat readies. Without it the supply-ai lab never left preparation in time. The game skirmish corpus (`mapgen --test skirmish`) and every skirmish test are unchanged. | Sound; affects labs with authored openings only. |
| The end-to-end battle test (`battle/airlift_tests.rs`, under `battle/tests.rs`) stages the damaged group by placing units inside the battle. It asserts the bought card doesn't drive, the far-side bearing, the group recovering, and the return to the post. | Acceptable; a crate-internal test. |
| `skirmish_ai/mod.rs` gave up its doctrine and recon concepts to `doctrine.rs` and `recon.rs` for room. Codex had also shortened `battle.rs`'s module doc to pass the ratchet; the doc is restored and the test moved instead. | Sound. |
| A card icon for `test_supply_heli` lets the lab's deck draw it. | Sound; test art. |

## Slice 10

| Choice | Verdict |
|---|---|
| Ids: `us_ch_47_chinook_supply`, `europe_ch_47_chinook_supply`, `eastern_mi_8_hip_supply`. Each extends `<transport id>_airframe`, an abstract parent that holds everything but carriage; the transports extend the same parent. All 139 existing resolved unit rows are byte-identical in `fixtures/catalog.json`. | Sound (P14). |
| Rotor radii come from the model build scripts' rotor parameters plus hull-centring offsets: 14.947 m for the Chinooks (the whole tandem) and 10.997 m for the Mi-8. | Sound; authored data, re-measure if the models change. |
| Voice admission (validator, sound workbench, generator CLI) reads the resolved `fixtures/catalog.json`, not raw roster rows, so abstract parents are skipped and inherited membership counts. | Sound; one owner of resolution. |
| `shopOrder(cards, typeOf)`: within a tab, ground movers sort before aircraft. Only LOG mixes the two; the truck stays first. | Sound (P13). |
| The model exporter accepts several unit types drawing one appearance when hull and mounts agree; a Python test covers it. Card icons are rendered by the asset CLI from the baked models. | Sound (P15). |
| Re-recorded: the catalog; five publication records and `scenario-overrides` (layouts and type indices); both capture recordings and the Paris manifest (layouts list three more ids; samples unchanged). Removing only the supply rows restored the old records. No corpus fixture changed: the low hover rise doesn't move any recorded skirmish. | Sound; named. |
| New baselines: the LOG picker, the supply hover card, and the Armory sheet for each faction. Old baselines were restored byte for byte. | Sound. |
