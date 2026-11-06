# 20 — Integration and closeout

**Status:** pending. **Dependencies:** 01–19.

## Contract and review question

Do the selected owners now explain the complete player/tool journeys with fewer
coordinated writes, while public behavior and retained systems remain unchanged?
This slice introduces no new API. It owns integration evidence, scope reconciliation
and removal of residue, not a last opportunity to broaden the refactor.

## Integrated journeys

- Menu/lobby → preview → Deploy → loading → first playable frame → pause →
  Restart → leave/return. Include cancellation after a response is queued.
- Selection → pointer preview → facing drag → release → delayed acknowledgement
  → observed order. Include purchase confirmation followed by a new choice.
- Session/world/quality replacement → fresh submitted frame → matching picks;
  departure while preparation is pending leaves the page GPU correctly owned.
- Page audio permission → loading bed → live battle → quick mute/unmute → menu;
  repeated visits return connections to the existing baseline without stale combat.
- Mechanics/map/sound draft → invalid input/refusal → review → save/conflict →
  explicit discard. Map retained geometry and cancelled sample remain correct;
  audition replacement cannot resurrect old playback.

Use existing production routes and consumer tests. Do not introduce a diagnostic
FSM route, generic trace system or a second set of fixture definitions.

## Proof and deletion

Reconcile every row in [scope](../scope.md): selected consumers use one owner;
retained families have no incidental conversions. Search old symbols, reset
bundles, aliases, transitional flags and test-only hooks. Inspect production-code
cost separately from tests/docs; reject extra indirection without a concrete
consumer/contract. Remove unused helpers only after tracing callers.

Run the once-only full repository closeout in [verification](../verification.md).
Reuse valid focused results. No balance report or native GPU parity tour is needed
without a new concrete concern: this plan changes no gameplay or shared Rust
renderer/presentation logic. Preserve the authority's existing digest/raw-bit
checks and every owning test suite.

For visual evidence, use fixed before/after inputs and `compare-screenshots` on
the whole interacted surface. Run unprimed `screenshot-critique` as the **last
visual acceptance check** and show useful shots with `preview-shots`, following
the non-blocking review window. No baseline reapproval or tolerance loosening.

Update only subsystem documentation whose ownership principles changed; do not
copy the implementation roster into READMEs. Keep evidence in `throwaway/` and
record concise conclusions. Apply the repository's substantive-change review.

## Decision budget and finish

Delegated: smallest existing integrated scenarios and organization of evidence.
Not delegated: relaxing a gate, changing retained behavior, or adding more selected
families. New scope needs its own explicit decision and slice.

Mark this slice complete only after every required gate passes and no selected
work remains. Update the README handoff, then use `close-spec` once implementation
has shipped to archive the plan as durable rationale. Planning completion alone
does not satisfy this gate.
