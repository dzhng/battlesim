# 02: A supply heli lands and serves (first playable)

**Contract.** An aircraft with `capabilities.deploy` sets up like a truck (Q2, Q4, D2, D3, D5, D6):
- Stopped, it brakes to a hover. Once its horizontal speed is zero, its deploy timer runs.
- While landing, its height follows the timer down to the ground by `drop_seconds`, with an ease-out flare at the bottom (T1).
- At touchdown the landed latch is set (P1). At full progress it serves through the unchanged supply rule.
- Packing holds it on the ground; at packed the latch clears and it climbs at its normal rate. It moves sideways only once it is at the low hover (P10).
- Packed mid-drop, it resumes flight from its current height.
- A landed unit, or one whose deploy timer is running, takes no part in air separation (D6, P22).
- The low hover clears every type that can land (D5).
- An explicit Deploy order in flight acts as Stop (P11).

In this slice the landed heli is **still an aircraft to every other rule**
(footprint, layer, targeting); slice 03 changes that. It still lands where it
stops; slice 04 adds the clear-spot search.

**Seam.**
- Contract `crates/contract/src/catalog/capabilities.rs` `Deploy` gains
  `drop_seconds: Option<f64>`. Catalog check (`catalog.rs`, beside "supply needs
  deploy"): required on an air-mobile type with `deploy` and less than
  `seconds`; refused on a ground type (P2).
- `crates/sim/src/deployment.rs` owns the landed latch on `Deployment` (P1). It
  exposes the drop share for the height law (for example
  `deployment::drop_progress(unit) -> Option<f64>`) and `deployment::landed(unit)`.
  Nothing else re-derives touchdown from `current`. The digest
  (`units/digest.rs`, ~268) folds the latch only while set.
- `deployment::target`: an aircraft's target stays `Packed` until it is
  stationary (zero XY air velocity, no movement goal) (D2).
- `crates/sim/src/units.rs`: `Unit::landed()` reads the latch. The `flies()` /
  `airborne()` split (P3) lands in slice 03; here only flight code reads `landed()`.
- `crates/sim/src/movement/air.rs`:
  - one height law while landing that bypasses the climb clamp (~120) and sets
    `air.velocity.z` to the forced sink;
  - landed: hold XY and z at ground, velocity zero;
  - packed: normal flight, with `may_advance` held until it reaches the low hover;
  - `separation` (~228-252) skips a landed unit both ways;
  - `low_hover` (~191) takes the tallest top of ground-layer types and of air
    types that deploy.
  - If `air.rs` would outgrow the file-size budget, put the landing law in a
    `movement/landing.rs` beside it. Slice 04's search joins it there.
- Test row `test_supply_heli` in `fixtures/units/test/aircraft.json` (P18):
  extends `test_heli`; `deploy {seconds 3, pack_seconds 1, drop_seconds 2}`;
  `supply` at half `test_supply`'s stock; `roles: ["logistics"]`; no roster.
  Hull no taller than `test_supply`'s.
- Lab: the supply route becomes a `?variant=` page on the `airHover.tsx` /
  `transport.tsx` pattern. The default variant is today's truck encounter,
  unchanged. A new encounter `heli` goes in `fixtures/maps/supply/encounters/`,
  listed in `meta.json`:
  - `test_supply_heli` flies in to a point beside the truck lab's recipients at
    the same offsets: a damaged `test_tank`, a spent `test_at` and a
    short-handed `test_rifle`;
  - demo buttons "Heli: lift off" (`set_deployment` packed) and "Heli: fly on" (move);
  - a per-variant camera at 60–90 m.

**Run / see.** `/labs/supply?variant=heli`:
1. The heli flies in, brakes, drops with a flare, and sits on the ground.
2. The panel reads DEPLOYING, then SUPPLYING; the recipients' rows read RESUPPLYING.
3. Hp, missiles and soldiers return, and the heli's stock falls.
4. "Lift off": PACKING on the ground, then it climbs.

Rotors still spin and the drop line still shows; the lab legend says those are
slices 06 and 07.

**Verify (test first).** A new `crates/sim/tests/supply_heli.rs`, reusing
`air::{battle_with, open_map, order, fixture}`. Helpers are game facts:
`on_the_ground(b, id)` (height equals terrain within 1 cm), and `deploy_s` /
`pack_s` / `drop_s` read from the resolved row.
- `it_brakes_to_a_hover_before_it_starts_to_land`: deploy progress stays 0 while its XY speed is above zero.
- `its_height_falls_with_setup_and_it_touches_down_before_it_serves`: z never rises during the drop, reaches ground at the drop time, never dips below; service starts only at full progress.
- `a_set_up_supply_heli_serves_its_reach_from_its_stock`.
- `it_packs_on_the_ground_and_climbs_only_once_packed`: z stays at ground while packing, then rises no faster than `climb_mps`; XY holds until it reaches the low hover.
- `packed_mid_drop_it_flies_on_from_where_it_was`: never landed.
- `its_vertical_velocity_matches_its_sink`: shots lead with it.
- `a_landed_heli_neither_drifts_nor_pushes_other_aircraft`: two landed 5 m apart both keep serving; a hovering heli beside one doesn't move it.
- `the_low_hover_clears_a_landed_heli`: a hovering recipient's hull bottom stays above the landed heli's top.
- `a_landing_replays_exactly`: record, replay, equal digest.
- Catalog test: an aircraft with `deploy` but no `drop_seconds` is refused; a ground type with `drop_seconds` is refused.

`web/scenes/supply.mjs` gains a `heli` section: it samples (tick, z, deployment,
service) every 15 ticks and asserts hover, then drop, then ground, then serving,
in order. Frames: `heli-drop`, `heli-landed`, `heli-serving`, `heli-packing`.
There is no visual gate here (unchanged art at a new height); show the frames
to the user inline.

**Digest.** None for existing battles: no existing unit deploys and flies, and
the test catalog's low hover is unchanged by P18. The test document set gains a
row, so a fixture pinning test-set identity re-blesses (named). Prove that the
outcomes of existing encounters don't move.

**Delegated.** The flare curve (monotone, ease-out, exactly ground at
touchdown); the brake law; the stock list's per-supplier wording.

**Stays green.** `supply.rs`, `deployment.rs`, `air_supply.rs`, `air*.rs`,
`transport*.rs`, the `supply` and `transport` scenes.

**Feedback that would change it.** The feel of the drop. The curve and
`drop_seconds` are the knobs.
