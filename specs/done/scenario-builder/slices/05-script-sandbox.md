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

## Slice 05 result (2026-10-12)

The contract crate now owns `script::CompiledScript`, a small deterministic
interpreter used by native and WASM callers.  Its source language supports
numeric/string/boolean expressions, conditions, bounded `repeat` blocks,
observation queries (`tick`, visible own/enemy counts and indexed positions),
seeded integer draws, ordinary `move`/`stop`/`engagement` orders, named events,
and a script-defined victory result.  The host receives only a borrowed
`ObservationFrame`, the executing side, tick, and an explicit seed.  There is
no ambient clock, DOM, network, filesystem, reflection, or battle handle.

Compilation rejects unsupported syntax with line/column source spans.  Runtime
execution enforces instruction, loop, output, and variable budgets and returns
named deterministic refusals; a failed script never silently falls back.  The
compiled identity includes `scenario-script-1` and the SHA-256 source identity.
Bundle admission compiles authored and victory sources before accepting a
canonical download, so malformed executable code cannot reach play.

Focused native tests prove fresh-run equality, ordinary command output,
observation/seed determinism, hidden-state and ambient-construct refusal,
fuel/output limits, source identity, and replay-safe victory output.  No unsafe
browser evaluation was added.  Battle integration remains slice 06: that slice
must invoke this contract host at tick boundaries and feed accepted commands
through the existing authority/replay log.

