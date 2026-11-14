# 05 · Lobby occupancy

**Depends on:** 01, 04. **Changes digests:** no.

**Slice variable:** which tags show a digit and which is marked as yours.

## Contract

Over the player-free picture, the lobby shows who holds each lane:

- a lane no slot holds shows the tag with no digit (A4);
- the human's own lane shows the white outline and "YOU" (D5);
- both sides follow the same rule (A5).

Occupancy follows every seat change at once, without starting a new preview
(A7). It is applied only while the preview is ready, so a redraw never pairs
the old map's sites with new players.

## Seam

`apps/battle-lab/src/lobby/Lobby.tsx` `MapFrame` (or a small helper beside it)
calls slice 01's reader with the preview's admitted `sites` and
`lobby.players`, and sets a state attribute on each `[data-lane]` group (e.g.
`data-held="none|ally|you"`). CSS in `web/src/menu.css` hides the digit or
reveals the own-lane mark. No lane maths in TypeScript.

## What a human can see

Lobby in a 3v3, moving yourself from Blue 1 to 2 to 3: the YOU mark moves down
the roads. Remove Blue 3 (now 2v3): Blue's bottom tag loses its digit. 5v5:
three numbered tags a side; Blue 4–5 mark none.

## Verification

- `web/tests/lobbyScreen.test.tsx` with a fake worker that returns real admitted
  sites and an SVG with `data-lane`:
  - in a 2v3, Blue's lane 3 is unheld and Red's is held;
  - taking Blue 2 moves the "you" state to Blue's lane 2;
  - a seat change makes no new preview request.
  Assert on the state attribute and request counts, never on CSS class names.
- Visual gates: `game-ui` first; `compare-screenshots` slice 04's shot against
  this one, crop as slice 04; `screenshot-critique` last; `preview-shots`,
  non-blocking.

## Delegated

The attribute's name and values; the CSS that applies them.

## Must stay green

Slice 03's behaviour checks; Deploy opens the previewed map.
