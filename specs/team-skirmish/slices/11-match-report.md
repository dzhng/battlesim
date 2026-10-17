# Slice 11: per-player match report

**Decisions:** D16 (timeline), L6. **Needs:** 03 and 10.

**Visual variable:** how readable the report is with up to 10 players. **Crop:** the result overlay. Out of scope: the battle view behind it.

## Contract it unlocks

The end-of-match report tells every player's story.

## Seam

`apps/battle-lab/src/MatchResultOverlay.tsx`:
- **Totals** are per player, grouped by side. A player's row reads "Blue 1 (you)" or "Red 3 · AI".
- **Timeline** names each player (the user's request of 2026-10-11).
- **VICTORY or DEFEAT** is judged against the viewer's side, not the literal `"blue"` (`:131`).
- **Spectating a zero-human battle** shows a neutral result.

## What the human can run or see

The report after the slice 10 2v2 scene, plus a lab mock at 5v5.

## Red tests first

- The report renders 1v1, 3v2 and 5v5 with the right per-player totals.
- The hard-coded `["blue","red"]` indexing is gone.
- A defeat on Blue reads DEFEAT for a Blue viewer.

## Visual gates

1. Use `game-ui` first.
2. `compare-screenshots` against the current report baseline.
3. `screenshot-critique`, unprimed, as the last check before acceptance.
4. A non-blocking `preview-shots` review: give the user about 5 minutes. If they're silent, decide on the evidence, record why in `choices.md`, close the shots and continue.

## Delegated

Row density; collapsing the timeline at 5v5.

## Stays green

1v1 report scenes; their content changes only in labels.

## Feedback that would change it

The user wanting a team summary above the per-player rows.
