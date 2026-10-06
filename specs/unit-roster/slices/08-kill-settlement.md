# 08 — Lethal provenance, credits and net team bounty

Status: focused native implementation complete; integrated publication/commit pending. Dependencies: 04. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Prove lethal-source propagation first, separately from payout arithmetic. Round already has source unit/side; damage Outcome and structural/collapse hits must preserve economic source through blast and building destruction. Emit one full-unit death fact, with original purchase price, victim side and hostile lethal source even if shooter is already dead. A squad pays only on last soldier death; suppression is not a kill. Friendly/source-free deaths pay nothing and do not erase victim bounty.

Economy settles a tick batch: snapshot both pools, ordinary 25% C, bonus min(B,50% C,B×C/S), scale a side's aggregate bonuses proportionally if needed, deduct paid bonus then 50% C losses floored at zero, then add killer growth 50% C. Supply truck adds 25% C once irrespective of stock; collected rewards never recursively fund bounty. No additional pool cap initially, per-death cap remains.

Use deterministic fractional carry and original cost snapshots. Immediate wallet credits must not grant unseen identity/location/destruction notifications or mutate contact memory. Bounty remains internal; do not add a new currency or economy dashboard.

## API seam and ownership

Flight/damage/collapse → stable full-unit death facts → tick-batch economy settlement. Simulation owns one economic death ledger; presentation observes wallet only.

## Narrow verification

Test dead shooter, blast multi-kill, collapse/garrison source, final squad casualty, friendly/source-free death, no duplicate ledger entry, original price, symmetric same-tick trades, aggregate pool cap and truck bonus. Assert wallet-only unseen reward leaves ordinary fog/contact evidence unchanged. Digest/replay/paired codec proof; one quick affected economy combat fixture, no broad retuning.


This is primarily a contract/data slice. No battle or screenshot is required for
pure documentation or metadata. If an actual visual change is produced, it inherits
the README's comparison, Preview and final unprimed screenshot-critique gates.


## Review surface and verdict

Native death/settlement timeline and one ordinary battle wallet readout. Verdict: deaths have causal ownership and exact selected arithmetic without fog leaks or iteration-order bias.

## Delegated freedoms and invariants

Internal type/function/file names and clean decomposition within the named owner
are delegated. Starter numerical tuning is delegated, with rationale and narrow
proof recorded in choices.md; physical/model facts require references. Cosmetic
spacing/icon fit may be adjusted reversibly under game-ui. Do not add mechanics,
change selected economy/victory/visibility rules or expand excluded capabilities.
If this slice exposes an unlisted material choice, record it in the choices ledger
and reslice the focused uncertainty rather than broadening implementation silently.

Keep existing developer fixtures, command sequencing, native/WASM agreement,
side-only knowledge and asset admission green. Verify changed contracts test-first;
non-behavior refactors preserve digest/replay outcomes. Review locally, update the
README pickup and commit the coherent pass, then proceed to the next independent
ready slice. No adversarial review or legacy compatibility scaffolding.

## Handoff evidence

- [ ] Red behavior/admission case observed where applicable.
- [ ] Narrow green proof and any allowed digest change recorded.
- [ ] Artifact/visual gates resolved where applicable.
- [ ] Choices, status and next pickup updated; focused pass committed.

Focused proof: twelve settlement integration tests plus an actual building collapse
confirm causal attribution, immutable entered price, hidden-victim wallet-only payout,
symmetric same-tick trades, finite pool caps, once-only payment, supply bonus and
fractional carry. Source/payout mutations produced expected red results and were
restored. Actual blind-fire purchase battle replays at every tick. Whole-feature
packed native/browser proof remains an integration gate.
