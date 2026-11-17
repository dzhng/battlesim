# Tick budget: research record

The contract and baseline are in the [README](README.md). Raw output of every
run is scratch under `throwaway/replay-slow/`.

## What the baseline's last minute is made of

Counted in the firing-solution path during minute 15 (a diagnostic build,
since removed), per tick on average:

- about 210 mounts choose a target afresh (no lock, or a lock they cannot
  shoot);
- they ask for about 13,500 firing solutions;
- **98% of those are for a target farther away than the weapon reaches**: the
  reach is judged on the solved intercept, after the arc is solved and
  raycast. Of them 78% come back "blocked", 18% "out of range" after a clear
  solve, 4% "out of reach" before any raycast;
- the solver's kept answers miss on about half of the requests: a moving
  shooter or target never repeats an aim to the bit.

## Hypothesis queue

| # | Change | Mechanism | Prediction | Status |
|---|---|---|---|---|
| 1 | Judge reach before solving the arc | A target whose every possible meeting point lies beyond the weapon's reach can never be shot; its solve and raycasts are waste | Weapons phase falls by more than two thirds in minute 15; over-budget ticks fall below the target; the battle's loads stay the same | **kept** (trial 1) |
| 2 | One lock choice per garrisoned mount a tick | The garrison's window requests run the whole lock choice a second time (a fifth of the weapons' time) | Weapons phase falls by up to a fifth of what is left after 1 | queued |
| 3 | A mount that found nothing looks again later, not every tick | About 210 mounts redo a full search thirty times a second | Large, but changes when crews reacquire: a mechanic decision | queued behind 1 |
| 4 | Soldiers' own-route planning and the vehicle shunt search | `final_leg::search` is a third of the trial-1 step; the 80 ms ticks recur every 61 ticks | The 80 ms ticks go; movement phase falls by half | **kept** (trial 2) |
| 4b | A vehicle whose detour comes back the same keeps its failed shunt | One stuck vehicle repeats an identical failing search every 61 ticks | The remaining 20–30 ms movement ticks go; a rule change | queued |
| 5 | Fog sweep | Flat share, bursty per sweep tick | Tail only | queued |
| 6 | Larger kept-answer store for the solver | Half the requests miss | Moot if 1 removes 98% of requests | parked |

## Trials

### 1. Reach before the arc: kept

`weapons::engage` returns out of range before solving when the nearest
possible meeting point (the target's distance less its speed over the round's
lifetime) is past reach. [Decision](choices.md).

| minute 15 | baseline | trial 1 |
|---|---:|---:|
| ticks over budget | 43% | **2%** |
| tick p50 / p95 ms | 32.1 / 58.7 | 5.9 / 20.9 |
| weapons, M instructions a tick | 363 | 17 |
| all phases, M instructions a tick | 596 | 197 |
| whole replay, wall seconds | about 400 | 130 |

Prediction held for cost; the prediction that loads would stay the same did
not. The digest moved and the battle diverges from minute 5 (it ends with 135
living units to the baseline's 136, fewer fallen, twice the path searches).
A diagnostic count on the baseline battle found why: 895 of about 340,000
shootable assessments came from the per-soldier retry on a squad whose middle
was past reach, and 213 from the per-muzzle fallback, most of them in one
engagement in minute 5. Accepted as a decision.

What is left in the trial-1 battle, minute 15: movement 65% of instructions,
fog 19%, weapons 9%. Ticks over budget are nine tenths movement: single ticks
of about 1.8 G instructions (80 ms) that recur about every 61 ticks. A sample
puts them in the soldiers' own-route planning
(`movement::soldier::plan_own` → `final_leg::search`), which the baseline
battle hardly used.

### 2. Searches that cannot succeed, and cheaper box tests: kept

Outcome-preserving: the replay ends on trial 1's digest.

The prediction named the wrong owner for the 80 ms ticks. They were one
stuck wheeled vehicle: its stall timer asks for a detour every two seconds,
the detour comes back the same, the arriving route drops the shunt that found
no way, and the same search fails again through all the poses its limit
allows. Each arc of that search spent most of its time in a square root for a
distance cull and in a box-overlap test that turned its axes twenty times.

The soldiers' own routes were a separate waste: 2,600 of 6,348 searches
failed, and the failures were 99% of the cost. A failed search walks its
whole window, twice when soldiers stand near. Most aimed at a point inside a
known body or ringed by standing squadmates.

- `final_leg::search` answers "no route" before searching when no step from a
  neighbouring cell can enter the goal: the same answer the search reaches.
- `shunt::Room::free` culls by squared distance.
- `math::Obb2::least_overlap` turns each box's axes once.

| | trial 1 | trial 2 |
|---|---:|---:|
| minute 15, ticks over budget | 2% | **1 of 1,800** |
| minute 15, tick p50 / p95 ms | 5.9 / 20.9 | 2.6 / 14.4 |
| minute 15, movement M instructions a tick | 127.6 | 29.3 |
| minute 15, all phases M instructions a tick | 197.1 | 97.9 |
| whole replay, ticks over budget | 239 | 16 |
| worst tick after setup, ms | 82 | 50 |
