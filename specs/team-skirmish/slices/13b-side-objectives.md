# Slice 13b: side objectives

**Decisions:** D55 (amends D3), D4, D40. **Needs:** 13.

## Contract it unlocks

An AI player takes open objectives quickly and keeps what it took, while its main group still fights as one.

## Seam

The `TeamPlan` from slices 12 and 13.

**Detached parties: one concept.** Slice 13's help detachments become one kind of a general party: units detached from a player's main group to an objective, with one lock and one lifecycle. There are three kinds:
- **help**: answers another lane's request (slice 13's rules, unchanged);
- **hold**: stays on an objective the player took;
- **claim**: goes to take an open objective.

Every party rule from slice 13 applies to all three kinds:
- its units can fight the threat there, by weapon target class;
- it includes at least one unit that can hold the objective, so aircraft never go alone;
- it is never re-tasked while engaged;
- a party lost on the way is replaced while still needed;
- a unit is in at most one party.

**Main objective moves on.** When a player's main objective is owned by its side and uncontested, the player's next plan picks a new main objective. The taken one becomes a side objective with a hold party.

**Open objectives first.** An objective is open when it is neutral, no enemy is visible within its radius, and no teammate's main group or party is going there.
- Each player sends claim parties to the open objectives nearest its lane.
- The main objective's score prefers open objectives over defended ones.

**Budget.** Parties of every kind draw from one budget, a share of the player's army, from `fixtures/skirmish-ai.json`. Claim parties come before hold parties, which come before new help, while the budget lasts. Help that is already engaged keeps its units.

**Pressure.** A side objective under pressure raises help through slice 13's mechanism, exactly as a main lane does.

**Memory:** party locks are non-digested memory (D40).

## What the human can run or see

Tournament reports that log, per match:
- when each objective is first taken;
- side objectives held;
- parties by kind and size.

The `ally-help` scene stays green.

## Red tests first

- A player whose main objective is taken and uncontested picks a new main objective and leaves a hold party on the old one.
- An open objective near a player's lane gets a claim party, and the main group does not go there.
- A claim or hold party is units that can hold ground, never aircraft alone, and never more than the budget.
- A unit is in at most one party; an engaged party unit is not re-tasked.
- A side objective under pressure raises help.

## Delegated

Party sizes, the budget share, the open-objective scoring bonus, tuned on tournament evidence in `fixtures/skirmish-ai.json`.

## Stays green

Non-skirmish digests. Zero refused AI orders. Every tournament match finishes.

## Feedback that would change it

If open objectives still sit unclaimed for minutes, or the main group thins out until it no longer fights as one, iterate here.
