# 06 · One mark per house

**Unlocks:** a shared house reads as one held position on the ground.

**Slice variable:** how many rings and outlines a held house draws. Ring
size, colour, width and the outline's shape are out of scope.

## Seam

- `apps/battle-lab/src/battleOverlay.ts`: the order layer draws one building
  ring per house for all its selected, revealed or leaving occupants, keyed by
  the building's structure owner. Each squad keeps its own routes.
- `heldBuildingsLayer` draws one outline per building, resolving replacement
  parts and identified enemy occupants to the same owner.
- Picking is unchanged (`sideInstances.ts` keeps the nearest-centre rule);
  soldiers and deck cards still select single squads.

## What the human can see

Shots of one selected occupant, two selected occupants, the Space reveal, a
squad leaving on an order, and an identified enemy-held house.

## Verification

- Judge two crops separately: the ring, then the outline.
- `compare-screenshots` against today's single-occupant shots of the same
  framing, then an unprimed `screenshot-critique` as the last check.
- Show with `preview-shots`; non-blocking, about five minutes, then decide on
  the evidence, record it in the ledger, close the shots, and go on.
- No battle digest moves.

## Delegated

How the overlay groups occupants; specimen framing.

## What would change this slice

The user wanting the ring to pick every occupant (the ledger's rejected
alternative): that would be a new slice for selection.
