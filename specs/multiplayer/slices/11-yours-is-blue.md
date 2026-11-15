# 11 Yours is blue

**Stage:** Foundation. **Needs:** 08. **Carries:** decision 64; landmine 5.

## Contract

In battle your side is the accent blue and the enemy is red, whichever side
you command. Colour means "mine" and "theirs", never "Blue" and "Red". The
lobby still names the sides Blue and Red and is not touched.

## Seam

Presentation only. Most of the HUD already colours by own and enemy. The
places that colour by side name are the ones to move, among them the scores
and objective rings (`web/src/hud.css:1593`), the battle overlay
(`apps/battle-lab/src/battleOverlay.ts:421`) and the result screen. Find the
rest by putting a Red commander on screen. Use
[game-ui](../../../.agents/skills/game-ui/SKILL.md) and
[renderer](../../../.agents/skills/renderer/SKILL.md) first, and reuse the
existing colours exactly.

**Slice variable:** which colour each thing wears. Out of scope and expected
to look wrong until slice 12: which way the map faces.

## Run or see

The dev battle, one browser per side. Take the same moment from both seats.

## Verification

1. Capture a Blue commander's and a Red commander's screen of one battle at
   one tick: HUD, objective rings, units, order marks, and the result screen.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md)
   to judge the Red commander's shot against the Blue commander's. The target
   is the same role wearing the same colour, not identical pixels.
3. Use [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md)
   on the Red commander's shots as the last check, unprimed.
4. Show the user with
   [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). Wait about
   five minutes; if they are silent, decide on the evidence, record it in
   decisions.md, close the shots and continue.

No test names a colour or a class.

## Must stay green

A Blue commander's screen is unchanged: compare before and after.

## Delegated

How "own side" reaches each component.
