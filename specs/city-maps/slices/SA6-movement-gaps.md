# SA6: movement gaps at bridges, wrecks and unreachable goals

**Depends on:** SA2, C69. **Kind:** slice, rules.

## Question
Do units finish the moves a viewer expects when the way is narrow, blocked or impossible?

## The moments (each is a ruled-out moment today)
1. **A squad at a bridge.** A squad wider than the deck jostles at its edges, and on an angled approach some soldiers stop on the bank for good (6 of 8 cross in the river lab). A tank's hull corner swings off the deck going round. Reproduced before rivers existed, with rect water.
2. **A wreck across the road.** A jeep plans round a wreck lying across an 8 m road, stops with its hull on the wreck's corner and plans the same route again, forever. The same loop was seen with a jeep among trees.
3. **An unreachable goal.** A move to a point that cannot be reached costs the whole search limit before it reports blocked (about 5 s for a 2.8 km leg across an unbridged river), and a reachable goal whose only way round lies outside roughly 400 m of the straight line is reported blocked.
4. **Two columns head-on on a forest track** can still jam.

## Contract it unlocks
Each moment above has a movement scenario that passes: the whole squad crosses; the jeep passes or reports blocked and stops asking; an unreachable order is refused quickly and a far detour is found or refused for a stated reason; the columns pass or one yields.

## API seam
`crates/sim/src/movement/`, `crates/sim/src/navigation/`, `route_planner.rs`; rules numbers in the fixture's `navigation` block. Read the SA2 Outcome and the entries "Integrating SA2 with C69" and "Navigation grid updates" in `../choices.md` first: the search, the route check and the standing check use different fit rules on purpose.

## What the human can run or see
GIFs from `movement_shots` for each moment, before and after.

## Verification
- Run tweak-mechanics first, for each moment.
- The pending checks in `c69-river-bridge` and `c69-river-around` (`crates/sim/tests/movement_scenarios.rs`) become real checks; new scenarios for moments 2 to 4.
- `city_report` crossings on the generated towns still have no tick over 33 ms; planning work reported.
- Named digest changes; `village_report -- --quick --compare main`.

## Delegated to the implementer
How a squad narrows to a deck; the blocked-goal proof. Record both in `../choices.md` under this slice.

## Must stay green
Route quality against the frozen planner (`navigation_quality` example); every movement scenario.

## Feedback that would change this slice
A fix that needs soldiers to path individually across the map, rather than follow a squad corridor, reopens squad movement as its own spec.
