# 09 — Cancellable physical return and condition-based retirement

Status: implementation in progress. Dependencies: 04. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Refund command orders a fastest ordinary route to own road-edge base, leaving unit vulnerable, armed under its existing policy and occupying its slot. A new order cancels withdrawal before arrival; destruction prevents refund. Pay/release slot exactly once at arrival. No bounty removal or instantly disappearing unit.

Use original price and resource denominators, field age from physical spawn including withdrawal, measured at retirement. Fraction 0.15+0.60×((H+A)/2)×max(0.5,1−t/1,200), bounded 15–75%. H: original squad HP with dead members zero or hull max HP. A: mean finite-row remaining/full fractions, lost carriers zero, unlimited rows ignored; no finite rows means one. Include Trophy and truck stock; resupply can improve condition but never resets age.

Observe withdrawal state and implement the Refund button in the existing unit command surface. Do not add separate base stock replenishment or transport rules. Pending pre-spawn cancellation remains full price under slice 04.

## API seam and ownership

Withdrawal intent → ordinary move + retirement condition → wallet/slot result. Existing unit command and observed state feed the same bottom bar/panel.

## Narrow verification

Test fresh/full and aged/half/worst refund values, arrival-only payment, cancellation, blocked return, death en route, lost carrier accounting, empty truck, Trophy rows, resupply-before-return and no bounty clearing. Replay/digest and narrow UI binding test; one return-to-base browser fixture.


For every visual shot produced in this slice: load game-ui and renderer as
applicable; compare candidate against its declared crop/reference and pre-change
baseline with compare-screenshots. Show the real artifact with preview-shots.
Run an **unprimed screenshot-critique as the last visual check before acceptance**.
The human checkpoint is non-blocking: leave a short (~5 minute) opportunity to
correct reversible choices while continuing independent work, decide from evidence
if silent, record rationale and close opened Preview shots. Never infer permission
for a new mechanic from silence. GPU jobs serialize under the existing harness.


## Review surface and verdict

One selected-unit refund command and visible road-edge withdrawal/retirement. Compare the command/panel crop and actual departure; model styling is out of scope. Verdict: the physical lifecycle and computed refund agree and cannot pay twice.

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

Focused evidence: arrival-only physical return and replay, Stop cancellation,
combat destruction without refund, occupied base retaining slot/payment, and
original health/finite-resource condition checks pass. Route-policy upgrade
cancellation exposed a missing branch and is under red/green correction. Trophy
resource fraction is integrated; its native/publication proof waits for protection
completion. Fresh browser departure and exact-once visual/lifecycle gates remain open.
