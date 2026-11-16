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

## As built (2026-11-15): it was already true

Nothing about the camera changed. The battle's opening camera
(`gameCamera.fromStart`) has always stood on the commanding player's entry
edge and looked across the map, and slice 08 made it read the commanding
player's own road. So a Red commander's side was at the bottom from the
first playable checkpoint. This slice names that function as the one owner
of the turn and adds the proofs.

What follows the camera without knowing a side: everything drawn in the
world (terrain, units, rings, order lines, ghosts, rulers, objective marks,
the lane digits painted on the ground) is projected through it, and the
controls already work in screen directions. No minimap, compass or
off-screen arrow exists to turn.

Checked in the `online-battle` scene, for both commanders: the own entry
projects to the bottom of the opening view; each of up, down, left and
right, by the letter keys, the arrow keys and the screen edges, moves the
view that way on screen; and a unit ordered to a point straight up the
screen advances up the screen and away from its own edge. A fast test
covers all four map edges. Turning the opening by half a circle, and
panning in map directions, each made a check fail.

**An unprimed look at both commanders' views** found them the same kind of
view (same distance, tilt and framing), each with its own way in at the
bottom centre and its unit advancing up the screen, and nothing upside
down, sideways or mirrored, the lane digit included. What it did find is
true for both seats and for single-player: the entry marker is small at the
opening distance, and a unit standing on the lane digit covers it.

**Lane numbers: kept as they are.** Both sides' lanes are numbered from the
same end of the map, so Blue's lane 1 faces Red's lane 1, and teammates on
either side name the same road by the same number. The cost is that lane 1
is on a Blue commander's left and on a Red commander's right. Numbering
from each commander's own left would put every lane's number on the same
side of the screen for everyone, and give one road two names. This is
shown to the user with the lobby's picture; the numbering stays unless
they say otherwise.

Not captured: a battle with more than one lane a side from a Red seat (the
dev battle is a 1v1). The claim for several lanes rests on the lobby's
picture and the camera's geometry.

