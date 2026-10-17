# Slice 13: requests for help and shared stance

**Decisions:** D4, D5, D40, D41 (re-baseline at 13 if 1v1 changes), D48. **Needs:** 12. **Research:** [`../research.md`](../research.md).

## Contract it unlocks

Allies cover each other and push together.

## Seam

The `TeamPlan` from slice 12 gains three things.

**Requests for help:**
- **Raised** when a lane's influence trend crosses a threshold, before the objective falls. This applies to AI and human lanes alike.
- **Answered** by the nearest ally whose force exceeds its own lane's need times a margin. It sends a detachment, not its whole army.
- **While engaged**, a detachment is locked to the request and never re-tasked mid-fight.
- **Released** when the request clears and its dwell has passed.

**Stance:** push, hold or fall back, chosen from the score trend, objectives held and the influence balance.
- Falling back takes effect at once.
- A push needs its dwell to have passed, plus every lane at its staging point or a timeout. Then every lane advances in the same decision interval.

**Memory:** request locks, dwell timers and push start are non-digested memory (D40).

## What the human can run or see

Tournament 5v5 reports, with requests for help and coordinated pushes logged per match. A 2v2 browser scene where an AI ally answers the human's lane under pressure.

## Red tests first

- A losing lane draws help from the nearest ally with surplus, and never from an ally without one.
- An engaged detachment is not re-tasked.
- Stance flips to fall back at once, and back to push only after its dwell.
- A push issues every lane's advance within one decision interval.

## Delegated

Thresholds, margins and dwell lengths, tuned on tournament evidence in `fixtures/skirmish-ai.json`.

## Stays green

Non-skirmish digests. Zero refused AI orders.

## Feedback that would change it

If the tournament evidence doesn't read as coordination (help that never arrives, pushes that aren't in sync), iterate here before moving on.
