# 12 Your side at the bottom

**Stage:** Foundation. **Needs:** 08. **Carries:** decisions 65, 66.

## Contract

In battle, the edge your side enters from is at the bottom of the screen. The
lobby does not turn: its sides stay left and right and its map picture keeps
one orientation for everyone.

## Seam

The battle camera's opening orientation, and anything in the battle view that
is drawn in map orientation and must follow it. Use
[renderer](../../../.agents/skills/renderer/SKILL.md) and
[game-ui](../../../.agents/skills/game-ui/SKILL.md) first.

**Slice variable:** orientation of the battle view. Out of scope: colour
(slice 11), the lobby.

One thing to judge on screen, not to assume: lane numbers run from one fixed
end of the map. Look at how they read for a Red commander once the view has
turned, and keep the numbering that lets teammates on both sides name the
same lane by the same number.

## Run or see

The dev battle from both seats: each commander's own units drive in from the
bottom.

## Verification

1. Capture both commanders' opening view and a mid-battle view.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md):
   Red's view against Blue's, judged on where each commander's own entry edge
   sits.
3. Use [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
   as the last check, unprimed.
4. Show the user with
   [preview-shots](../../../.agents/skills/preview-shots/SKILL.md), the lane
   numbers in particular. Wait about five minutes; if they are silent, decide
   on the evidence, record it in decisions.md, close the shots and continue.
5. A test that a Red commander's camera controls move the view the way the
   keys and screen edges say.

## Must stay green

A Blue commander's view; the skirmish lobby's map picture.

## Delegated

Where the turn is applied.
