# 09: The AI flies one in for an emergency

**Contract.** (Q5, P17)

Trigger:
- a unit in one of the AI's combat groups is below the tuning's share of any
  finite weapon kind, or below its share of health (squads: living members
  against squad size; vehicles: hp against max hp);
- and no own supplier's route time to that unit is under the tuning's reach
  time. Set-up time is excluded.

Response:
- If the side owns no flying supplier, it buys one. If it owns one in reserve,
  it sends that one.
- The heli flies to the far side of the unit in need: half the supply radius
  from it, away from the centroid of the enemies the side sees, or toward its
  own entry if none are seen. Slice 04's search picks the spot. A far side
  refused or off the map sends it home.
- It serves. When the group no longer triggers, it returns to the truck's
  support spot, where it deploys as a reserve.
- Engaged on the way in, it carries on.
- The truck stays the standing supplier (slice 01).

**Seam.**
- One module, for example `crates/sim/src/skirmish_ai/airlift.rs`. Its pure
  functions: `emergency(frame, rules, tuning) -> Option<Need>`,
  `far_side(recipient, known_enemies, entry, radius) -> point`, and the mission's
  small state (idle, inbound, serving, returning).
- `purchase.rs` calls it before the role loop. `mod.rs` issues its orders after
  the support loop.
- Tuning values (half, half, 30 s) go in `fixtures/skirmish-ai.json`, the AI
  tuning owner.
- The group is the AI's existing party grouping (`team/`). Route time is
  `route_time`, which already handles aircraft as straight-line time
  (`battle/entry_routes.rs`).

**Run / see.** `/labs/supply-ai`, a builder lab on the `allyHelp.tsx`
precedent:
- test units only, carded through encounter overrides (`test_supply` and
  `test_supply_heli` as `sup`, `test_tank`, `test_rifle`);
- blue and red under `active_ai`, on an empty map;
- blue's truck is parked more than the reach time from blue's damaged tank
  group, which faces red.

Watch blue buy the heli, land it on the side away from red, refill the group,
and fly it back.

**Verify (test first).**
- `skirmish_ai/tests/purchases.rs`, with constructed frames. Thresholds are read
  from the tuning and set on either side of each:
  - `a_crippled_group_out_of_supplier_reach_buys_the_flyer`;
  - `a_crippled_group_a_truck_can_reach_buys_nothing`;
  - `a_healthy_group_buys_nothing`;
  - `unlimited_ammo_never_counts_as_running_low`;
  - `a_reserve_flyer_is_sent_rather_than_a_second_bought`.
- `airlift.rs` unit tests for `far_side` (bearing away from the enemy centroid;
  entry fallback). This is the one place a unit test pays: geometry with edge
  cases.
- One end-to-end battle in `crates/sim/tests/skirmish_ai_orders.rs` on the test
  deck:
  - the heli lands with its bearing from the group pointing away from the seen
    enemy;
  - the group's ammo or hp rises;
  - afterwards it heads for the support spot.
- The `supply-ai` scene samples the purchase, the landing and the return. No
  visual gate.

**Digest.** None until roster rows exist: the test deck only. The game skirmish
corpus moves in slice 10.

**Delegated.** The mission state's storage; the tie-breaks between equally
needy groups (fixed and deterministic).

**Stays green.** Slice 01's tests, every `skirmish_ai` test, and the skirmish
corpus with the game roster (byte-identical: no game card flies and supplies yet).

**Feedback that would change it.** The tuning values, after watching AI battles.
They are data.
