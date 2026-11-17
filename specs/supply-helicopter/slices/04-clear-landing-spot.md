# 04: Nearest clear spot, or NO LANDING ZONE

**Contract.** When it wants to deploy, the heli finds the nearest clear spot within
`air.rotor.landing_reach_m` (40) of where it stopped (Q3, D4, P6–P9).
- If a spot is found, it flies there as ordinary packed flight and starts its
  deploy timer once stationary over it.
- If none is found, it stays packed in the air with `MoveState::NoLandingSpot`.
  Its panel reads NO LANDING ZONE in the warn tone, the same form as ROUTE
  BLOCKED.
- The spot is re-checked every tick until touchdown. If it becomes blocked, the
  heli reverses toward packed and searches again from where it is.

What blocks a spot (P8): any body footprint that blocks a vehicle, forest
ground, ground a wheeled vehicle can't stand on, live ground hulls and soldiers,
other landed aircraft, and the off-map margin. The clearance is a disc of the
type's `rotor_radius_m` about the hull centre. The search reads the true world.

**Seam.**
- Contract: `Mobility::Air` gains `rotor_radius_m: Option<f64>`, required on an
  aircraft that deploys (catalog check); `AirRules.rotor.landing_reach_m`;
  `MoveState::NoLandingSpot`.
- Sim: one deterministic nearest-first ring search, generalised from
  `crash::resting_place` (`crash.rs` ~323) to take a reach, a clearance disc and
  a "blocks" predicate. The crash wreck and the landing both call it (one owner).
  The wreck's placement must stay byte-identical. The chosen spot is held on the
  unit as an internal goal, not a player order: no order line, no queue entry. A
  player order issued during the shift replaces it.
- `test_supply_heli` gains `rotor_radius_m`.
- Web: `panelRows.ts` gets a `no_landing_spot` row modelled on `route_blocked`.
  Reuse an existing warn colour; add no new colour.

**Run / see.**
- `heli-shift`: the heli ordered onto a crate (`add_prop` at tick 0) slides to the nearest clear spot and lands.
- `heli-crowded`: ordered into a ring of props wider than the reach, it hovers and shows NO LANDING ZONE. Ordered out of the ring, it lands.

**Verify (test first).** In `supply_heli.rs`:
- `ordered_onto_an_obstacle_it_lands_on_the_nearest_clear_spot`: the spot's disc is clear, it lies within the reach, and no clear spot is closer (the test samples to check).
- `with_no_clear_spot_in_reach_it_stays_packed_aloft_and_says_why`.
- `deploy_starts_only_once_it_is_stationary_over_the_spot`.
- `a_unit_that_moves_onto_the_spot_mid_drop_sends_it_back_up_to_search_again`: no overlapping footprints at touchdown.
- `the_search_is_deterministic`: replay digest.
- `air_crash.rs` wreck placement stays byte-identical.
- Catalog: an aircraft with `deploy` and no `rotor_radius_m` is refused.

Tests build the prop ring from the rules' reach plus a margin, never a literal
40.

Visual (this slice changes one thing a player sees, the panel row): pin the row
in the panel workbench (`web/scenes/panels.mjs`) as one more state permutation.
Run `compare-screenshots` against the ROUTE BLOCKED row: one look per concept.
The unprimed `screenshot-critique` on the `heli-crowded` frame is the last
check: "can a player tell why this helicopter won't land?" Show the user inline;
their review is non-blocking.

**Digest.** `MoveState` gains a variant (own observation contract), with TS
decoders and parity re-blessed (named). Catalog identity moves with
`rotor_radius_m` on the test row. Existing battle outcomes are unchanged.

**Delegated.** The ring step; the tie order between equally near spots (fixed
and deterministic); the clearance margin beyond the radius (default 0).

**Stays green.** Slices 02–03 tests (their encounters are on open ground, so the
spot is where it stopped), `air_crash.rs`, the panels scene.

**Feedback that would change it.** Too few spots in towns: raise it with the
user. Never silently grow the user's 40 m.
