# Slice 00: probe multi-entry admission, timings, throughput

**Decisions:** D36, D49. **Writes to:** `throwaway/` and the "Facts slice 00 measures" section of `choices.md`. This slice changes no production code.

## Contract it unlocks

Before any contract work, evidence that multi-entry maps can generate, pass fairness admission and field a 5v5 opening.

## Seam

A throwaway branch or worktree holding a rough version of the dedicated entry-arm pass from D36. Run it through:
- `mapgen::generate`;
- `sim::map_analysis::admit_skirmish`, with fairness taken as each side's best entry per representative unit and objective, within 15%;
- `PreparedWorld` timings.

Delete the branch once its numbers are recorded.

## What the human can run or see

One table in `throwaway/slot-probe/report.md`, with a row per entry pairing (1v1, 2v3, 3v3, 5v5) over `fixtures/skirmish-corpus.json`. Columns:
- admission rate;
- refusal reasons;
- prepare time per size;
- estimated opening field time at 1 s dispatch per entry;
- whether the left and right arms met spacing and avoided river mouths.

## Verdict

- **Pass:** multi-entry admission rates are in the same range as today's 1v1. Prepare time leaves the preview responsive.
- **Fail:** reslice 04 before anything else. Change arm placement, or change what fairness compares, and record the change as a named decision.

Never loosen the 15% threshold to make the probe pass.

## Delegated

Probe structure, report format, how rough the throwaway arm pass is.

## Stays green

Everything. Nothing in production changes.

## Feedback that would change it

If the user cares about a specific map type (open, mixed or metro) most, weight the corpus toward it.
