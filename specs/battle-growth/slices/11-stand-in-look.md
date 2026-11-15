# 11 Stand-in look

**Question:** does the block read as a unit whose art is still loading, in the
game's own style?

**Depends on:** 10.

## Slice variable

The block's form and colour role, and nothing else.

**Judged on:** crops of the blocks in slice 10's fixture: a vehicle and a
squad, each near and far, in the open and at a fog edge.

**Out of scope, even if it looks wrong:** the moment of the swap (a hard swap
is the player's choice), how dense a squad's blocks are, HUD readouts,
shadows on anything but the block.

## Target

Option D in [the mock](../visualizations/first-contact-mock.html): a plain
grey block that the model replaces. The mock is a diagram, not a style
reference; the style comes from
[`game-ui`](../../../.agents/skills/game-ui/SKILL.md).

## What slice 10 left (2026-11-15)

[Four shots](../assets/stand-in/) of the default block and an
[unprimed reading](../assets/stand-in/unprimed-critique.md) of them. Two
things for this slice to settle:

- The default block reads as a model that failed to load. Nothing says it is
  a unit.
- A wreck whose art has not arrived is drawn as a box by the map props' own
  fallback, in a different colour from the unit's block. A player sees two
  kinds of box. Give them one look, or say in the ledger why they differ.

## Fixed

- Use `game-ui` before choosing anything.
- The block's colour is an exact colour the UI already has. No new hue and no
  near-copy.
- The block must not be mistakable for a roster unit or for scenery.

## Steps

1. Capture the fixture's shots with the default block from slice 10.
2. Choose the form and colour role with `game-ui`; capture again.
3. Use
   [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
   to judge each candidate against the same frame with the art loaded (does
   the block sit where the model does, at its size) and against the first
   capture.
4. Run [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md),
   unprimed, as the last check before accepting.
5. Keep the candidates, the comparison and the critique in
   [`../assets/`](../assets/).

## Review checkpoint (non-blocking)

Open the accepted shots with
[`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md) and allow
about five minutes. If the player says nothing, decide on the evidence, record
the decision and why in the [ledger](../decisions.md), close the shots and
proceed. A person who has looked blesses the fixture's baseline picture; until
then slice 10's mechanics tests are the gate.

## Delegated

The block's exact shading within the chosen colour role, and whether it casts
a shadow.

## Stays green

Slice 10's tests, the visual suite.

## Feedback that would change this slice

The player preferring a contact marker or nothing at all: both were options in
the mock, and either deletes this slice's look work while keeping slice 10's
picks.
