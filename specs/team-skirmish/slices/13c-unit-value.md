# Slice 13c: value-weighted strength

**Decisions:** D56. **Needs:** 13b.

## Contract it unlocks

The AI judges and buys by what units are worth against what they face. A balance change to a unit's cost or weapons changes the AI's play without code changes.

## Seam

One owner: a unit's **value against a target class**. It is the unit's cost if any of its weapons can engage that class, else nothing. Target classes are the weapons' own (ground vehicles/armour, infantry, low air, high air). It lives with the team planner and is read by:

- **The influence grid** (`team/`): each side's strength at a point is the summed value of its units there against the enemy kinds present, or seen, nearby. With no enemy known, a unit's ground value is used. This replaces 1.0 per unit.
- **Force and surplus** (help and parties): a player's force is the summed value of its units, not a headcount. Party budgets and help margins use the same measure.
- **Purchases** (`BalancedDoctrine`): the role recipe stays. Within a role, candidate cards are ranked by value per credit against the enemy kinds the side has identified. Cost is the tie-break, as now. The existing air-threat pivot becomes a case of the same rule.

Specialization comes from weapon data only. No role name decides a matchup.

## What the human can run or see

Tournament reports unchanged in shape. A balance probe: two tournament runs differing only in one unit's cost (a fixture override in a test, not a shipped change). The AI's purchases and lane judgments shift accordingly.

## Red tests first

- A lane of one tank against three cheap squads that cannot hurt it reads as the tank side's, not the squads'.
- An AT team counts against armour and not against aircraft; an AA unit the reverse.
- Raising a card's cost in a test catalog lowers how often the AI buys it within its role; adding an enemy air unit raises AA purchases.
- Force and surplus use value: a helper with one expensive tank and a lane needing less is chosen over one with three cheap squads that cannot fight the threat.

## Delegated

Normalisation of the influence scale (existing thresholds were tuned for headcount; retune them on tournament evidence in `fixtures/skirmish-ai.json`).

## Stays green

Non-skirmish digests. Zero refused AI orders. Every tournament match finishes. The file-size ratchet.

## Feedback that would change it

If the AI stops fielding a full combined-arms army (D2) because one card dominates value per credit, cap it within the recipe rather than dropping roles.
