# 03: Landed, it is a ground vehicle

**Contract.** While landed (Q1, D1, P3–P5, P19), the heli:
- has a ground footprint: ground movers route around it, and arrivals keep clear of it;
- is in layer Ground with target class Vehicle: weapons that engage ground vehicles engage it, and weapons that list only air classes stop at touchdown;
- takes ground concealment, including forest and smoke;
- is never shoved and knocks down no trees.

Killed while landed, it leaves a ground wreck with no crash and no crash blast.
Killed during the drop, it crashes. A damaged landed heli keeps smoking. Its
door guns fire while landed (G). Its own side's view is enough here; slice 05
makes enemy tracks follow.

**Seam.** `crates/sim/src/units.rs`:
- `Unit::flies()` (motion is air) and `Unit::airborne()` (flies and not landed) (P3).
- Every current `airborne()` caller is reassigned:
  - **flies:** flight dispatch (`movement/mod.rs` ~130, ~156), arrive facing (~678), drive pivots (`drive.rs` ~502), `move_placement.rs` (~115, ~139), carriage (`carriage.rs` ~146), `smoking`.
  - **airborne:** `ground_footprint`, `layer`, `flight_velocity`, separation, sensing concealment (`sensing.rs` ~198), treads (`battle.rs` ~1868), arrival clearance (`battle/dispatch.rs` ~131).
- `Unit::push_class()` reads the hull's authored push class. `clear_lanes`
  (`battle.rs` ~1809) and `push::pressed` (`movement/push.rs` ~82) read it instead
  of `ground().push` (P4).
- `death_roll` (`movement/drive.rs` ~674) returns early for a landed unit
  before touching `ground()`.
- The death branch (`battle.rs` ~2226) crashes only if `airborne()`; a landed
  unit takes the ground wreck path with no bail-out.
- `Unit::target_class(rules)`: Vehicle while landed, the type's class otherwise.
  It replaces the type reads in `weapons.rs` (~336, ~603, ~651, ~745, ~764) and
  `knowledge.rs` (~492) (P5). The skirmish AI's planning (`capability.rs`) keeps
  the type's class.

After the split, grep `airborne()`, `ground()` and `target_class()` across
`crates/sim`. Every hit must be classified, and the classification goes in a
comment on `flies` / `airborne`.

**Run / see.** Two encounters on the supply map:
- `heli-fire`: the `heli` units plus a red `test_tank` and a red `test_aa`, both holding fire until a scripted fire-at-will after touchdown. The tank shells the landed heli and the AA team stays silent; "Lift off" turns the AA team on and the tank's gun off.
- `heli-ground`: the heli landed beside a lane, a blue `test_jeep` ordered through the spot, and two scripted kills: one while landed, one mid-drop. The jeep drives round it; the landed kill leaves a ground wreck with no blast; the mid-drop kill crashes.

**Verify (test first).** In `supply_heli.rs`:
- `landed_a_ground_gun_engages_it_and_an_air_only_weapon_does_not`, and the reverse once it packs.
- `a_ground_mover_routes_round_a_landed_heli`.
- `a_landed_heli_is_never_shoved_and_fells_no_trees`.
- `landed_it_takes_forest_and_smoke_concealment`.
- `killed_landed_it_leaves_a_ground_wreck_and_no_crash_blast`.
- `killed_in_the_drop_it_crashes`.
- `its_door_guns_fire_while_landed`.
- `a_packed_heli_never_reaches_ground_movement` (the crash guard: drive, shove, order and place around a packed and a landed heli).

`supply.mjs` gains `heliFire` and `heliGround` checks that read the observation:
which unit fired at it, the wreck present, no crash entry.

**Digest.** None for existing battles. Prove with the sim digest pins
(`exact_numbers.rs`, `battle_authority.rs`, `air*.rs`, `deployment.rs`,
`supply.rs`, `hull_guns.rs`, `door_guns.rs`) byte-identical.

**Delegated.** Method names; whether `push_class()` lives on `Unit` or the hull.

**Stays green.** Everything slice 02 named, plus `ground.rs`, `drive.rs`,
`traffic`, `cover.rs` and `air_crash.rs`.

**Feedback that would change it.** None expected; Q1 is the user's call.
