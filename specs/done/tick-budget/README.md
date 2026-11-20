# Tick budget on a long skirmish

A played large mixed skirmish slowed to visible slow motion after about a
quarter of an hour. The simulation's tick cost had passed its real-time budget
(one tick period at the rules' `tick_hz`), and the worker sheds the time it
cannot make up, so the whole battle ran slow. Five changes took the recorded
battle's last minute from 43% of ticks over budget to none, and the mean tick
to a fifth of its cost. Four keep every outcome; one is a
[decision](choices.md).

## What was measured

[replay_report](../../../crates/sim/examples/replay_report.rs) replays a
battle a player exported and reports each game minute's tick times against
the budget, the step's instructions by phase, and a closing digest. It admits
the replay to whatever build runs it, so a change is measured on the same
battle and the digest says whether the outcome held.

The verdict is the share of ticks over budget in the last full minute, native,
run alone. Instructions retired are the comparison between builds: wall time
moves with machine load, and the first numbers, taken beside a compile, put
several times too many of the early minutes' ticks over budget.

The battle itself was scratch: replays are not retained across builds, so it
is no fixture.

| recorded battle | before | after |
|---|---:|---:|
| last minute, ticks over budget | 43% | 0 of 1,800 |
| last minute, tick p50 / p95 ms | 32.1 / 58.7 | 2.2 / 11.0 |
| last minute, M instructions a tick | 596 | 82 |
| whole battle, M instructions a tick | 252 | 54 |
| whole battle, ticks over budget | about 2,200 | 1 |

The two columns are not the same battle after its fifth minute: the first
change moves the digest. Both end near 136 living units.

## Why the cost did not follow the unit count

Between minutes 12 and 14 the step's instructions rose by more than half
(and the median tick doubled) while the units grew by a tenth.
None of the waste was per unit; each was work whose answer was already
certain, multiplied by something that grows with a long battle.

| Change | What was certain | Multiplied by | Effect on the recorded battle (M instructions a tick, whole-battle mean unless said) |
|---|---|---|---|
| Reach before the arc | A target past the weapon's reach cannot be shot | idle mounts × every known enemy and contact | weapons 363 → 17 in the last minute |
| Goal that cannot be entered | No step from a neighbouring cell enters it, so no route does | jammed soldiers replanning at their rejoin interval (`REJOIN_EVERY_S`) | movement 59 → 13.5, together with the two smaller changes below; the 80 ms ticks gone |
| Fog keyed on nearby changes | A body edit out of sight reach changes nothing a sweep reads | every observer of both sides, per edit anywhere | fog 30.3 → 18.6, together with the foliage shortcut and the ray's reach recomputed only as its depth grows |
| Entries that could win | An entry whose straight-line time is slower than a route already found | every entry × every purchase, all lanes on one tick | worst orders tick 710 → 262 M instructions; the 35–42 ms purchase ticks now 11–19 ms |
| A path's box before its chords | A hull clear of the path's box is clear of every chord | every solved shot × every friendly hull | weapons 13.8 → 11.0 |

Two smaller ones ride with them: the vehicle shunt search culls by squared
distance and `Obb2::least_overlap` turns each box's axes once, which is every
box-overlap test in the simulation.

## What must stay true

Each shortcut answers early only what the full work would have answered. The
code that holds each one to that:

- **Reach.** `weapons::engagement::engage` returns out of range before solving
  only when the nearest possible meeting point is past reach: the target's
  distance less its speed over the round's lifetime. It must never reject a
  shot the solve would allow; the check after the solve stays the authority.
  `reach_is_judged_where_the_round_meets_its_target_before_the_line_of_fire`
  pins it, including a target that closes fast enough to be met inside reach.
- **Own routes.** `movement::final_leg::search` tests the goal's neighbours
  with the same step test its loop uses (`step_to`). A second test for the
  early answer would let the two disagree.
- **Fog.** A reused observer field must equal a fresh sweep.
  `OcclusionGrid::sweep_observer` re-sweeps for a body change or a clearing
  within sight range, two fog cells and `WorldGeometry::sight_change_margin`
  of an eye; that margin must cover everything an edit can change in what a
  ray reads. A checking build that re-swept every reused field across the
  recorded battle found each equal.
- **Foliage.** `ForestState::canopy_cell` answers open ground from the forest
  span buckets without searching a page. It holds only while every foliage
  cell lies in a bucket of its forest's span; a debug assertion checks the
  page whenever the shortcut is taken.
- **Entries.** `Mobility::least_route_time` must be a time no planned route
  can beat: no speed factor exceeds the mover's best speed, and a route ends
  within `GOAL_REACH_M` and half a navigation cell's diagonal of its goal.
  `Battle::entry_route_time` asserts it in debug builds;
  `select_skips_entries_that_cannot_beat_the_best_route` pins the skip and
  the tie going to the entry listed first.
- **Friendly lines.** `HullFrame::box_may_come_within` uses the sphere
  `may_come_within` uses per chord, so it skips only hulls every chord would
  have skipped.

## Dead ends

- **A failed shunt stands while its route is unchanged.** One stuck wheeled
  vehicle repeats an identical failing search on every stall-timer detour.
  Keeping the failed shunt when the arriving route equals the current one
  matched 71 of the 111 detours that met a failed shunt: the rest arrive with
  an extra first waypoint that is pruned only at the next steer. It also moved
  the stuck vehicles onto the shunt's own retry interval (`RETRY_S`), which at
  the current tuning coincides with the skirmish AI's purchase interval, so
  their searches landed on the purchase tick and the replay went from one tick
  over budget to five. Catching the rest needs the shunt to record the route it
  searched. Dropped.
- **Sorting publication anchors as packed integers.** Byte-identical
  publications and a fifth less packing work, about a hundredth of the tick.
  Not worth the bit-packing.
- **Borrowing the live planning grid for an entry's route estimate** instead
  of copying the side's geometry. A search pays for each clearance tile once
  per knowledge revision and the live planner's per-tick allowance is counted
  in the same payment, so an estimate on the live grid changes later routes;
  the purchase preview also runs it from the browser and must not touch live
  state.
- **Sweeping fog from the eye's fog cell.** Most re-swept eyes stay in their
  cell, so it would halve the fog phase again, but fog and spotting would stop
  agreeing cell for cell
  (`a_rays_last_visible_cell_is_the_last_one_sensing_sees`).
- **Reach before a guided weapon's own sight line.** Most guided sight tests
  are for targets past reach, but the exact form must judge reach at the
  squad's middle and every seen soldier, from every origin a garrisoned unit
  may fire from.
- **A larger kept-answer store for the firing solver.** Half the requests
  missed it; judging reach first removed 98% of the requests.
- **Sharing the garrison's second lock choice.** Seats move between the two
  choices, so it is not the same work twice.

## What is left

- The remaining cost is fog for moving single-eye units, vehicle steering and
  shunt searches, and target ranking: no single owner, and any further cut is
  a rule change or tracking what a result depended on. The one tick still over
  budget in the recorded battle is a purchase tick that coincides with a
  failing shunt search.
- A mount with nothing to shoot still ranks every known enemy every tick. It
  is cheap now; making it look again later, not every tick, is a mechanic
  decision that was not needed.
- Not measured here: the same battle as WebAssembly in the browser, which runs
  slower than native.
- Outside the simulation, from the same player report: the page's per-frame
  render preparation and React re-renders hold the main thread, and the worker
  stops ticking while the page still holds every publication buffer
  (`PUBLICATION_POOL`), so a run of slow page frames also slows the battle;
  and GPU buffers grow with no art load to explain them. Neither was touched.
