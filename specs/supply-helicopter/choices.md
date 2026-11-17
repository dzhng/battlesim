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
