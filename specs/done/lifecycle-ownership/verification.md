# Verification and review

The [repository check ladder](../../development.md#check-ladder) governs cost.
This document records the shared verification policy. Focused tests and consumer
journeys remain in the repository's test suites and git history for each owner.

## Before each change

Read the existing test for the seam and identify which contract it proves. Run the
narrow relevant case before editing. Add a missing important preservation case
before replacing state, one at a time using `write-tests`; characterize current
required behavior rather than private field arrangement. A separate defect fix
needs a named decision and a test that first fails for the actual user-visible or
API failure. Do not add enum-name, reducer-action, class-name or copied-config tests.

Mock only natural edges: worker channel, clock, file/network completion, browser
GPU admission and audio output. Preserve real owners inside the test. Use the
existing mounted components or exported client APIs, not new test-only production
hooks. Existing pure unit tests stay where they earn fast feedback; prefer the
outer practical consumer for new regression coverage.

Acceptance evidence records focused commands/results, controlled response ordering,
changed ownership, and deleted state. Keep scratch evidence under
`throwaway/lifecycle-ownership/`; durable docs record conclusions and pointers,
not copied logs or mutable test inventories.

## Gates by boundary

| Question                                        | Narrow evidence                                                                                                                          |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Does stale work alter a newer owner?            | Controllable delayed response through the existing client/component, then actual admitted result/output                                  |
| Does cancellation release the correct resource? | Owned worker/graph/scene lifetime and late completion, including repeat cleanup; observe outcomes, not invented internal phases          |
| Are command results and credits identical?      | Existing client/authority tests, accepted commands, release ordering and raw publication bits                                            |
| Does an editor retain the right draft/map?      | Mounted editor with real draft logic and controlled HTTP boundary; inspect values, review identity and displayed artifact                |
| Does file publication remain safe?              | Existing server/publication tests; no new journal format or transaction implementation                                                   |
| Does picking refer to the drawn image?          | Viewport tests for skipped submission/replacement, then existing camera/input scene when browser behavior is in question                 |
| Does audio remain identical?                    | Existing command/graph tests, admission/drop output and lifecycle cleanup; browser/offline audio only for a question mocks cannot answer |

The [web manifest](../../web/package.json) owns test/typecheck commands; a focused
invocation is `bun run --cwd web test -- tests/client.test.ts`. Build required WASM
and native test helpers only when the test's subsystem needs them. Do not change
Cargo build configuration, artifact identities, or dependencies to make a planning
or cleanup check cheaper.

## Visual and interactive preservation

These slices do not redesign the UI. Still, controls and event timing are visible
behavior. Use the existing [fixture registry](../../apps/battle-lab/src/fixtures.json)
and [browser scene runner](../../web/README.md#checks-and-evidence) rather than
creating an FSM lab. Choose the smallest existing scene that proves the changed
interaction. Capture before/after at identical content, seed, tick, camera, quality,
viewport and input state. The variable is the changed interaction/lifetime; color,
layout, models, animation tuning and scene density are frozen.

For every slice that produces a shot:

1. Use `game-ui` for the player-facing preservation constraints and `renderer`
   before renderer-boundary work. Verify the actual production entry path.
2. Use `compare-screenshots` against the before/approved target and the crop named
   in the slice. Whole-frame checks protect surrounding UI after the local verdict.
3. Run unprimed `screenshot-critique` as the **last visual acceptance check**. A
   passing scene or the implementer's own eyes are insufficient.
4. Show useful shots with `preview-shots`; use the README's non-blocking review
   window. Do not reapprove a baseline or relax tolerances to pass a refactor.

No paid generation, asset rebake or recording of new content is needed. Fetch only
the relevant LFS assets. Serialize expensive GPU work using the repo's existing
lock. Tests do not play unsolicited sound aloud.

## Determinism and performance

No Rust simulation or numeric algorithm change is selected. Worker/client cleanup
must preserve command acceptance, tick scheduling, per-tick digests and packed
publications; reuse `authority.test.ts` and the authority browser fixture. A UI,
editor or audio lifecycle slice does not require a played balance battle.

Do not introduce additional per-frame allocations, recurring timers, eager asset
downloads, concurrent GPU candidates, or queue growth. Existing bounded-work tests
and source inspection are the first evidence. An expensive performance run must
answer a concrete regression concern a cheaper check cannot settle; do not add a
benchmark project to this refactor.

If a later approved scope change touches digested state, write a new precise slice
before implementation. Preserve the original scalar digest encoding and compare
matched accepted-command inputs and full per-tick outputs. Source changes can
legitimately change the engine-build identity; compare within each build and never
disable cross-build replay admission. No migration is a substitute for parity.

## Closeout once

For each coherent change, run its owning focused checks, necessary type/lint checks,
and the repository's `review` procedure. Count production code separately from
tests/docs and inspect any increased indirection. A new owner must delete the old
coordination, not merely wrap it. Follow `codex` for substantive implementation
review where required by the repository skills.

After all slices, run the root manifest's full `check` and `verify` once, plus the
production web build if the scene runner has not already established that artifact.
Reuse a still-valid result; rerun only what a subsequent edit invalidated. There
is no balance-report requirement for this plan because it selects no rule changes.
All encountered check failures must be handled under repository policy, with
unrelated corrections isolated rather than hidden in a cleanup diff.

The final closeout reconciles the scope ledger, searches for obsolete
state/aliases/compatibility residue, and updates only the subsystem docs whose
ownership contracts changed. The closed rationale is archived under
`specs/done/lifecycle-ownership/`.
