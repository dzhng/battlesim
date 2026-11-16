# 00 · Proof and probes

**Unlocks:** the oracle every later slice is judged by. No production code
changes.

## What to build

1. **The relief test, red.** New file `crates/sim/tests/shared_hosts.rs`,
   registered in `crates/sim/tests/main.rs`. One test,
   `a_squad_ordered_into_a_held_house_walks_in_when_its_holder_leaves`, driven
   through `Battle::accept`, stepping and the side's observation:
   - A building whose seat total (read from its published or computed plan,
     never a pinned number) leaves no room for B while A is inside.
   - A is inside. B, some distance away, is sent `OccupyBuilding` and the
     command is accepted.
   - A is sent `ExitBuilding` so that A starts stepping out before B arrives.
   - Assert in order: B starts stepping in only after A is stepping out; A ends
     outside; B ends inside, whole; no refusal for B is published; the seats
     taken never exceed the total at any tick.
   - It fails today: `garrison::validate` refuses B with `BuildingOccupied`.
2. **The digest probe.** An uncommitted scratch program (output in
   `throwaway/shared-hosts/`) that plays each case below for a fixed number of
   ticks and writes its digest every tick, keyed by the commit:
   - single-occupant: building entry and exit, window swaps, reinforcement
     inside a building, collapse, `Board`, `Collect`, helicopter loading;
   - encounters with defender garrisons: `street/*`, `market-town/menu`,
     `paris-corner/corner`, `market-town-test/assault` (use the loader
     `crates/sim/tests/menu_reel.rs` uses);
   - contested, recorded separately: the garrison lab scenario and the
     transport controls fixture.

## Confirm first

- The garrison lab's cast fits the contested demos: two rifle squads filling
  the corner shop and the scouts arriving to no room (one draft measured 8 + 8
  into 16 seats and 4 scouts).

## Verification

- The relief test fails for the stated reason (`BuildingOccupied`), not a
  setup error.
- The probe runs twice on the base commit with identical output.

## Delegated

Probe structure and output format; the test's map and positions.

## Must stay green

Everything; nothing in production changes.
