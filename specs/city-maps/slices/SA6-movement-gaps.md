# SA6: movement gaps at bridges, wrecks and unreachable goals

**Depends on:** SA2, C69. **Kind:** slice, rules.

## Question
Do units finish the moves a viewer expects when the way is narrow, blocked or impossible?

## The moments (each is a ruled-out moment today)
1. **A squad at a bridge.** A squad wider than the deck jostles at its edges, and on an angled approach some soldiers stop on the bank for good (6 of 8 cross in the river lab). A tank's hull corner swings off the deck going round. Reproduced before rivers existed, with rect water.
2. **A wreck across the road.** A jeep plans round a wreck lying across an 8 m road, stops with its hull on the wreck's corner and plans the same route again, forever. The same loop was seen with a jeep among trees.
3. **An unreachable goal.** A move to a point that cannot be reached costs the whole search limit before it reports blocked (about 5 s for a 2.8 km leg across an unbridged river), and a reachable goal whose only way round lies outside roughly 400 m of the straight line is reported blocked.
4. **Two columns head-on** can still jam: on a forest track, and on the few roads that lead to a river's bridges. On a generated Open Small river map, four vehicles sat behind an enemy jeep that was still planning, 700 m from the river; 3 of 8 vehicles arrived in 900 s against 7 of 8 on the same seed without the river.
5. **Long routes on river maps.** On 8 km and 10 km generated maps with a river, two of twelve units ordered edge to edge were refused a route and two were still planning at 300 s; planning work rose from 1–5 M to 21–36 M, and three ticks retired about 1.0 G instructions each. None of it happens on the same seeds without a river. The generator's road graph does reach every settlement through the bridges (`crates/mapgen/tests/river_world.rs`), so the fault is in how a long leg finds and commits a journey over a bridge.

## Contract it unlocks
Each moment above has a movement scenario that passes: the whole squad crosses; the jeep passes or reports blocked and stops asking; an unreachable order is refused quickly and a far detour is found or refused for a stated reason; the columns pass or one yields.

## API seam
`crates/sim/src/movement/`, `crates/sim/src/navigation/`, `route_planner.rs`; rules numbers in the fixture's `navigation` block. Read the SA2 Outcome and the entries "Integrating SA2 with C69" and "Navigation grid updates" in `../choices.md` first: the search, the route check and the standing check use different fit rules on purpose.

## What the human can run or see
GIFs from `movement_shots` for each moment, before and after.

## Verification
- Run tweak-mechanics first, for each moment.
- The pending checks in `c69-river-bridge` and `c69-river-around` (`crates/sim/tests/movement_scenarios.rs`) become real checks; new scenarios for moments 2 to 5.
- `city_report` crossings on the generated towns, river maps included (generate them with `river_chance` forced to 1 for a seed, or use seeds the C52 Outcome's Rivers section names), have no tick over 33 ms, no refused route where the road graph connects the ends, and no unit still planning at 300 s; planning work reported.
- Named digest changes; `village_report -- --quick --compare main`.

## Delegated to the implementer
How a squad narrows to a deck; the blocked-goal proof. Record both in `../choices.md` under this slice.

## Must stay green
Route quality against the frozen planner (`navigation_quality` example); every movement scenario.

## Feedback that would change this slice
A fix that needs soldiers to path individually across the map, rather than follow a squad corridor, reopens squad movement as its own spec.

## Outcome

All five movement moments and the integrated generated-river clock corpus pass; the sim lane owns its combined closeout gates. Implemented the five movement moments through shared bridge corridors, local terrain-aware soldier lanes, bounded public-terrain proof and route revalidation, and traffic manoeuvres that make room for reversing vehicles. The ownership boundary remains movement/navigation/route planning; no map-generation, garrison, visibility or structure schema changed. `RouteCheck` is the navigation-to-planner owner for resumable no-shove validation; road arcs also retain whether they cross a physical deck.

The unchanged movement checks now pass: all eight soldiers and the tank cross both C69 approaches; the jeep passes the road wreck; an impossible crossing is refused on tick 1 without replanning; a bridge 5.4 km outside the direct line is found; and all eight opposing vehicles finish. A missing-endpoint regression places the road 1.1 km from the goal without changing the 1 km access rule; a second red/green regression puts a nearer road on the wrong bank and a farther legal approach beyond the radius. Independent review found that discovering one missing endpoint suppressed a later search for the other; a combined red/green regression now pins independent per-endpoint discovery. A cheap direct-Journey regression then found that Shortest ranked bridge roads by time; its corrected graph now uses metres for Shortest and seconds for Fastest across connectors, arcs and heuristic. The 1.2 km test chooses the nearer dirt bridge for Shortest and the farther paved bridge for Fastest. Alternative access discovery and candidate filtering are resumable, and physical connector rejection stays local to its endpoint. Terrain and revision probes have explicit per-step limits, including an irregular 9.8 km diagonal with allowance 1.

The full sim crate passed before the final resumable terrain/access stages; subsequent focused movement, navigation, road-journey and route-planning proofs cover those stages (52 tests, including every movement scenario), and clippy passes with warnings denied. The final policy-unit correction additionally passes 31 road/planner tests and independent review. At final movement source 8aa6c2c6, dense-oracle physical fit and two-sided reachability pass over 12,064 cases; numerical quality changes are disclosed under SA6 in `../choices.md` and pinned in [`../assets/sa6-movement/`](../assets/sa6-movement/README.md). Independent `codex review` could not run with the configured model/account combination; manual shape, diff and documentation review was completed, with root's independent review of traffic and hull escape logic.

Final native GIFs and all six before/after frame sets are preserved in the main checkout under `throwaway/sa6-final/{1373746a/movement-before,8aa6c2c6/movement-after}`, with fixed-camera pixel comparisons, feature crops and independent critique. The first fresh critique found two large-distance cases unreadable and their remote bridge outside the framing; supplementary native overview and tracked/bridge views address that evidence gap, confirmed by the same fresh critic. It additionally observed dry shoulder travel beside the painted road; off-road ground is legal and the continuous deck/water/hull checks remain green. Public-state checks, rather than still positions, prove refusal and absence of repeated planning.

The final integrated `city_report` binary (root HEAD be7255b0, Rust/code-equivalent 15a690ea, SHA256 `470c75a85d8b00ca91589c3debffd022d499f7b6a8b0997c1488d8f4f46339bd`) runs all six frozen generated river maps for 300 s with six units per side and reach 0.92. Every arm has zero ticks over 33 ms, no refused route and no unit still planning at the end. Crossing p99 spans 2.50–5.21 ms; the largest tick is about 11 ms on Open Medium. Planning work totals span 150,356–1,722,493; the busiest ticks use 4,063–4,117 against allowance 4,000 plus one bounded scheduler step. Raw stage instructions, wall/CPU measurements, map hashes, arguments, progress and final digests are frozen in [`../assets/sa6-movement/`](../assets/sa6-movement/README.md), with the original outputs preserved in main `throwaway/sa6-final/integrated-be7255b0/`. The old partial 1373746a corpus lacked the final SA5 reach-local winner and ran under external load; it is historical movement evidence, not this clock gate. Root's integrated closeout additionally passed 485 sim tests and 585 web tests.

The same integrated binary also cross-checks the frozen SA5 dry 6 km and 10 km maps at 100 units per side for 30 s: zero ticks over 33 ms, worst ticks 26.0/21.4 ms wall and 23.05/20.10 ms CPU, with 130.4/129.6 G instructions. These short full-force runs prove recurring cost, not route completion: 111/96 units remain queued or planning at 30 s, none refused. Their named combined-rule digests are `d40a1926b0093c62` and `9e56ce4fa46e43fd`; the manifest retains exact frozen-map hashes and arguments.
