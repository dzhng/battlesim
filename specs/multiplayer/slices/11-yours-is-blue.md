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

## As built (2026-11-15)

Most of the battle already coloured by role: scores, objective rings and
labels, callouts, contacts and order marks read the same from either seat
before this slice. What still coloured by side name was the result screen's
two final scores and its border. They now take their colour from the
viewer's role, through the role mapper the scoreboard and timeline already
used. The overlay takes its side from the session's commanding viewer
instead of a separate argument. No colour was added.

Added at integration: **on the result screen the viewer's own side comes
first,** in the score and in the two scoreboards, as it does in the
battle's score strip. Before, a Red commander's own score was first during
the battle and second on the result. A watcher keeps seat order.

Checked in the `online-battle` scene, by the colours the page computes and
never by a colour's value or a class name: both commanders' own things wear
one colour and their enemy's another, at five moments of one battle taken
from both seats at the same tick (preparation, moving, capturing, holding,
contact), and on the result screen. A Blue commander's and a watcher's
result screens are unchanged, and all 40 approved pictures of the UI scene
match.

The result screen was judged on the game's own finished-match example, shown
to each seat; no online battle was played to its end.

**An unprimed look at the Red commander's screens** agreed that the battle
reads correctly (cyan is mine, red is the enemy's, grey is nobody's) and
tripped on the result screen:

- **A side's name is drawn in the other side's colour.** The Red
  commander's own column is headed "RED" in blue, and the enemy's "BLUE" in
  red, and so are their rows. Decision 64 makes colour mean mine and theirs,
  and the lobby names the sides Blue and Red, so after a battle the two
  meet. **Open, for the user:** keep the names; or head the two columns by
  role ("YOUR SIDE", "ENEMY") in battle and on the result. The
  recommendation is the second, taken when slice 18 puts callsigns on the
  result screen. Nothing was changed here.
- The score order flipped between battle and result: fixed, above.
- The amber ring and leader line on the player's own selected unit sit
  close to the enemy's red. It is the same amber a Blue commander sees and
  single-player has; noted for the battle screen's owner.

Everything else it listed is the battle screen as single-player has it:
captions running into a callout, an objective label over a unit standing
on it, a unit hidden behind the reinforcements panel at one camera
position.

**Decided by the user on 2026-11-15: head the columns by role.** To a
commander the result screen's two sides are headed "YOUR SIDE" and "ENEMY";
to a watcher they keep their names, "BLUE" and "RED". The three approved
result pictures taken from a commander's seat were re-approved with the new
headings. Players' rows still carry their seat names until slice 18 gives
an online match callsigns.

