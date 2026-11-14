# 06 · Battle: lane digit behind the chevrons

**Depends on:** 01. **Changes digests:** no.

Load `game-ui` and `renderer` before starting.

**Slice variable:** the painted digit's placement and legibility at the
opening and strategic camera heights. Chevron colour, pulse and glow are out
of scope.

## Contract

Each of the viewer's side's entries already paints three marching chevrons
(`buildDeploymentMarker`, `packages/battle-renderer/src/orderOverlay.ts:693`).
Its lane digit is now painted as ground paint **behind** them: between the map
edge and the first chevron, with the stack slid inward to make room (D9).

- Only digits 1–3 ever occur (at most three lanes), so they are stroke glyphs
  in the mesh. No font, no text renderer.
- Every lane paints its digit, held or not (revised A4).
- The viewer's own lane is emphasised in a way that reads as the lobby's white
  band, without adding text to the ground.
- Only the viewer's side is marked (A1, already true).

## Seam

- `apps/battle-lab/src/battleOverlay.ts`: `BattleOverlayScenario.deployments[]`
  gains `lane: 1 | 2 | 3` and `own: boolean`. `battleOverlayScenario`
  stays pure and takes slice 01's answer plus the viewer's player id;
  `apps/battle-lab/src/useBattleOverlay.ts` makes the Wasm call. While
  touching it, parse the scenario once: `BattleView.tsx` and
  `battleOverlayScenario` both parse it today
  ([launcher follow-ups item 4](../../launcher-followups/README.md)).
- Move `buildDeploymentMarker` out of `orderOverlay.ts` (over the size budget)
  into its own module in `packages/battle-renderer/src/`, and add the digit
  there. Reuse the chevrons' pen, stroke width rule and colours.
- Native needs no code: it replays the recorded overlays. Regenerate any
  overlay captures that parity fixtures hold.

## What a human can see

A generated 3v3 on a Large map, seated as Blue 2: the camera opens on road 2,
whose digit is emphasised; roads 1 and 3 show theirs.

## Verification

- Extend `web/tests/entryChevrons.test.ts`, measured from mesh coverage:
  - every lane paints between the edge and the first chevron;
  - the own lane's paint differs measurably from the others';
  - the other side's entries paint nothing (as today).
  Assert positions and coverage, never a style value.
- Shots at the opening camera height and the strategic height, over road and
  grass, with smoke nearby. Check the digit isn't clipped by the map border or
  hidden under the chevrons, and isn't confused with the objective flags.
- Visual gates: `compare-screenshots` today's chevrons against after;
  `screenshot-critique` last; `preview-shots`, non-blocking.

## Delegated

Digit stroke geometry and size; the spacing between the digit and the stack;
how the own lane is emphasised (within "reads as the lobby's white band, no ground text").

## Out of scope (even if visibly wrong)

Chevron colour, pulse rhythm and glow; the map border; objective flags.

## Must stay green

`entryChevrons` side filtering; the file-size ratchet; digests.
