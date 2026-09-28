# G0: verdicts

**Depends on:** S1–S5. **Kind:** gate.

## Question
Do the plan's assumptions survive, and what numbers does Phase 1 build to?

## Contract it unlocks
A gate, not code. Its outputs are the decisions below, written into `../decisions.md` as `G0-*` rows, the Budgets table updated, and every Phase 1 slice file whose inputs changed updated.

## API seam
The five `spikes/S*.md` verdicts.

## What the human can run or see
A one-page G0 summary for the user: kills fired, numbers, and the decisions taken.

## Verification
Decide, on the evidence:
- the fog cell (S1);
- the sim step target (S1);
- the residency radius, triangle budget per tier, interior tiers and kit texture edge (S3);
- the placement encoding (S2);
- the fog technique (S4);
- the seam strategy and exposed-edge data (S5).

If a kill fired, reslice the affected lanes before any Phase 1 slice starts.

Non-blocking: open the summary for the user with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If they're silent, decide on the evidence, record each decision with its rationale, and proceed.

## Delegated to the implementer
None; every output is a named decision. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
n/a

## Feedback that would change this slice
Any user pushback on a G0 decision reslices the affected lane.
