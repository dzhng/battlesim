# Slice 07: lobby model

**Decisions:** D17, D18, D21, D26, D29. **Needs:** 01 for the player type; 06 for `toRequest` and links (stub those until 06 lands).

## Contract it unlocks

The lobby's rules as plain serializable state, with no UI.

## Seam

- **Model.** `apps/battle-lab/src/lobby/model.ts` (new) holds the players (the contract's player shape), map choice, map seed and battle seed. Its pure operations:
  - `preset` (1v1, 3v3 or 5v5);
  - `add(side)` and `remove(slot)`: at least 1 per side, at most the lobby's data limit of 5;
  - `setController`;
  - `seatHuman(slot)`, which turns the previous Human row into AI;
  - `setFaction(side)`;
  - `setSize`, which ratchets to the floor (D18);
  - `reroll`, which changes only the map seed;
  - `toRequest`, `toUrl` and `fromUrl`.
- **Map inputs.** The model reports which changes alter map inputs: type, size, region, seed and entries per side. Controller and faction changes don't regenerate the map.
- **Builder.** The scenario builder's `BuilderSlot` (`scenario-builder/model.ts:19`) is deleted. The builder uses this player model (D29).
- **Size labels** are S · M · L · XL, replacing the `small: "XS"` entry in `battleLinks.ts:60` (D17).

## What the human can run or see

Table tests only. The builder still works.

## Red tests first

- 3v4 and 2v3 via add and remove. Remove stops at one per side.
- Human transfer. The single-player lobby always has exactly one Human, on Blue.
- The size ratchets up with players and never down.
- Reroll touches only the map seed.
- A controller change is not a map-input change; adding a third Blue player is.
- URL round trips for 1v1, 3v4 and 5v5.

## Delegated

State representation (reducer or class), naming.

## Stays green

Builder tests and journeys.

## Feedback that would change it

The user changing a lobby rule. Every rule is closed in `choices.md`.
