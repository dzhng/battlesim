# 03 · Admission on arrival

**Unlocks:** the rule itself, for both hosts. The relief test goes green.

## Seam

- **`crate::hold` owns host seats:** a `Seats { total, staying, leaving }`
  value per host, where `staying` counts living soldiers of squads stepping in
  or inside and `leaving` those stepping out. A carrier's total is
  `carriage::seats(rules, kind)`; a building's is its plan total (or
  `seats_for_state` before any plan exists).
- **One function is the only constructor of `Phase::Entering`**, for both
  hosts:
  1. Building only: another side's squad holding or stepping in → give up
     silently, as today.
  2. `staying + need > total` → pop the order and publish `CapacityFull`.
  3. Otherwise create the hold.
  Called from the building path in `hold::advance` and from every
  `carriage::start_boarding` site, Collect's already-beside-the-carrier path
  included.
- **The end of the entry timer**, for both hosts: `staying + leaving <= total`
  counting everyone, else drop the hold, pop the order, publish
  `CapacityFull`.
- **Seating:** a squad admitted into a building takes windows nobody holds,
  in today's even spread. Existing occupants never move on entry.
- **At the command:** one helper refuses, per squad in `ack.refused`, only a
  squad bigger than the whole host. Used by `Board`, `Collect` (bigger than
  every listed carrier) and `OccupyBuilding`. Board's closest-first trimming
  goes.
- **Delete:** `carriage::seats_used`, `carriage::fits`,
  `garrison::held_by_another`, the silent slot-count drop in `hold::advance`.
- **Cadence:** keep it. A carrier hold starts and advances in the same tick
  (`carriage::advance` runs before `hold::advance`); a building hold starts at
  `Entering(1)`. Flattening that moves single-host digests.

## Confirm first

- Tick order in `battle.rs`: commands, then `carriage::advance`, then
  `hold::advance`, then movement.
- Both directions of a host share one timer (`Host::timing`).
- Whether a squad ordered back into the building it is leaving cancels its
  exit through the existing "wants to stay" transition.

## Tests (`crates/sim/tests/shared_hosts.rs`)

- The relief test, now green: remove its `#[ignore]`.
- `squads_share_a_host_while_seats_remain_and_a_latecomer_finds_no_room`, one
  test covering both hosts: two squads share; a third arrives, gets
  `CapacityFull`, ends with no hold and an empty queue, and stays outside when
  a seat later frees; a squad bigger than the host is refused at the command;
  an overflowing `Collect` refuses the squad that arrives last, not the first
  to finish its timer; a leaver doesn't count at the start check; a leaver
  that turns back makes the entrant fail at its end check.
- Rewrite in place: `squads_beyond_the_seats_are_refused_closest_first` and
  `capacity_lost_during_boarding_is_published_as_refusal`
  (`tests/transport.rs`), `a_building_takes_one_squad_and_a_second_is_refused_until_it_leaves`
  (delete; the relief test replaces it) and
  `a_squad_that_finds_an_enemy_squad_inside_gives_up_its_order`
  (`tests/garrison.rs`; it now gives up when stepping in starts).

## What the human can run or see

In the garrison lab, order two squads into the corner shop one at a time:
both get in. Send the scouts: they walk over and get "no room" with its voice
line.

## Verification

- `UNCHANGED_SINGLE_HOSTS` and `UNCHANGED_OTHER_SCENES` identical.
- Named moving check `ARRIVAL_ADMISSION_BEFORE_AFTER`: only the contested
  garrison lab and transport controls runs may move.
- `LIVE_SCRIPT_REPLAY` green.

## Delegated

The admission function's name and signature; how refusals are collected.

## Must stay green

`transport::`, `transport_rolling::`, `transport_stationing::`, `garrison::`,
`buildings::`, `entry_reach::`, the skirmish AI transport tests, `menu_reel::`.
