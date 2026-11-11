# 29: SP howitzers

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 28.

**Models:** M109A7, PzH 2000, 2S19M2, PLZ-05 (`sp_howitzers.py`, one script with `appearances=list(SPECS)`).

## The user's notes (the done list)

- M109A7, PzH 2000, 2S19M2, PLZ-05 (each): "turret shape looks too big, or body too small. front side panel sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Fix the turret-to-hull ratio by the survey's answer to its question 3, for each card: a hull (frame) fix, already done in slice 15, or a turret redraw.
- Skirts (`sp_howitzers.py:101-105`) go through `side_skirt` on each card's outline. Wreck selectors (`:253-262`) keep working.
- Clear the M109's baseline entries.
- If the four `SPECS` outgrow the ratchet, split the turret builder out by concept.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
