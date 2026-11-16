# Tick budget on a long skirmish

A played large mixed skirmish slowed to visible slow motion after about a
quarter of an hour: the simulation's tick cost passed its real-time budget, and
the worker sheds the time it cannot make up.

## Contract

- **Target:** on the recorded battle, in its last full game minute (the
  137-unit end state), no more than 15% of ticks cost more than the tick budget
  (the reciprocal of the rules' `tick_hz`), step plus both sides' publications,
  native, wall time.
- **Evaluator:** `cargo run -p sim --release --example replay_report -- <battle.json>`
  ([replay_report](../../crates/sim/examples/replay_report.rs)), run alone on
  the machine. The "ticks over budget" column of the last full minute is the
  verdict; the CPU-time column beside it guards against scheduling noise; the
  phase table (instructions retired) is the load-independent comparison between
  builds.
- **The battle:** the player's exported replay, kept as scratch
  (`throwaway/replay-slow/battle.json`). Replays are not retained across
  builds, so it is a development workload and not a fixture.
- **Outcome rule:** a change meant to keep outcomes must end the replay on the
  baseline digest. A change that moves the digest is a named decision in
  [choices](choices.md), and its rows are compared as a different battle.
- **Not in scope:** the browser's own costs (main-thread render preparation,
  React re-renders, GPU buffer growth) and WebAssembly's slowdown over native.
  They are real and listed under [what is left](#what-is-left).

## Baseline

Unchanged build, run alone. The budget was 33.3 ms when measured.

| minute | living units | tick p50 ms | p95 | ticks over budget |
|---:|---:|---:|---:|---:|
| 6 | 70 | 11.3 | 27.8 | 1% |
| 9 | 97 | 11.2 | 33.7 | 5% |
| 12 | 123 | 14.2 | 39.1 | 7% |
| 13 | 131 | 20.2 | 44.7 | 8% |
| 14 | 136 | 30.0 | 56.8 | 35% (sampled; 27% by CPU time) |
| 15 | 136 | 32.1 | 58.7 | **43%** |

Last minute, instructions a tick by phase: weapons 61%, movement 21%, fog 14%.
A call-tree sample of minutes 13–14 puts three quarters of the step in the
weapons' target choice, nearly all of it in the firing-solution solver
(`flight::solve::solve_launch_past`): the arc's intercept maths and its
raycasts against terrain and bodies.

The cost does not follow the unit count: it doubles between minutes 12 and 14
while the units grow by a tenth.

## Hypotheses

See [research](research.md) for the queue, each trial and what it showed.

## What is left

Filled in at closeout.
