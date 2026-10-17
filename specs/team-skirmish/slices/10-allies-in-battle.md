# Slice 10: allies in battle

**Decisions:** D16, D24, D47. **Needs:** 03 and 06.

**Visual variable:** telling allies from your own units at default and far zoom, over grass, road, fog and smoke. **Crop:** unit marks and info panels. Out of scope: the match report (slice 11) and the HUD layout.

## Contract it unlocks

A human can fight alongside AI teammates without confusion or silently refused orders.

## Seam

- **Selection (D47).** `controlled(observation, player)` in web is the only "which units can I order" rule. It is used by:
  - `useUnitControl.ts:80, 127, 207, 283`;
  - `pointerIntent.ts:46`.

  Box-select and double-click pick only your own units.
- **Colour.** `presentation.hud` and `overlay.xray.own` (`fixtures/game.json:1723`) gain an allied role, pale ice `#b9f0ff`. Allies also get a shape cue: a hollow selection mark.
- **Ally panel.** Selecting an ally opens its normal info panel read-only, with an owner line ("Blue 2 · AI") and no order controls.
- **Factions.** `useBattleSession.ts` reads factions from the players, removing `factions[0|1]` at `:194, 242, 267, 280, 1166`. The presentation crate takes its ally relation from `owner`.

## What the human can run or see

A 2v2 browser scene with you and one AI ally against two AI.

## Red tests first

- Box-select over mixed own and allied units selects only your own.
- No command ever leaves the client with an ally's unit.
- The ally panel shows its owner and has no order controls.

## Visual gates

1. Use `game-ui` first.
2. `compare-screenshots` before and after on the same 2v2 frame.
3. `screenshot-critique`, unprimed, as the last check before acceptance.
4. A non-blocking `preview-shots` review: give the user about 5 minutes. If they're silent, decide on the evidence, record why in `choices.md`, close the shots and continue.

## Delegated

The exact hollow-mark geometry; the panel's owner-line wording.

## Stays green

1v1 battle scenes. Native compiles.

## Feedback that would change it

The user finding pale ice too close to own Blue. In that case the shape cue carries more of the difference, or the hue changes.
