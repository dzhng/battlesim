# 05 — Deterministic authored-code sandbox

## Contract unlocked

Advanced hand-authored code can produce bounded orders/events/victory signals portably and deterministically, or the feature is explicitly removed from the shipped runtime.

## Seam

Run a focused runtime spike before exposing script fields in the builder. The host receives only `(tick, slot/side-scoped observation, deterministic seeded RNG)` and returns bounded ordinary commands/events. It has no wall clock, browser/DOM, filesystem, network, ambient randomness, hidden `Battle` access, or direct rule mutation.

Freeze the proven source/compiled representation, runtime version, instruction/fuel/tick budget, allocation limits, error semantics, and scenario identity participation. Script failures are deterministic refusals or recorded state transitions, never silent fallback.

## Runnable artifact

A native/WASM probe runs one victory/event script twice from fresh state and compares outputs, then replays the accepted command log with scripts disabled. It includes a loop/fuel exhaustion refusal and an attempt to read hidden state.

## Verdict

Keep the sandbox only if native/WASM determinism, bounded work, and observation isolation pass. If not, remove authored executable code from the shipped runtime and keep only the existing typed event/order seam; do not add unsafe browser evaluation.

## Verification

Use deterministic digest/replay comparisons, host API refusal tests, source-span diagnostics, and instruction-budget tests. No visual gate is required unless the script editor is introduced early.

