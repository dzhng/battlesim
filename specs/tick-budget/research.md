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
| 4 | Soldiers' own-route planning | `final_leg::search` under `soldier::plan_own` is a third of the trial-1 step and all of its 80 ms ticks | The 80 ms ticks go; movement phase falls by half | next |
| 4b | Vehicle shunt search | `movement::shunt::search` is most of the vehicles' movement | Movement phase falls further | queued |
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
