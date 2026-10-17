# Slice 08: lobby map picture

**Decisions:** D14, D23, D39. **Needs:** 04 and 07.

**Visual variable:** how legible the map is at lobby size. This means terrain, roads, water, forest and buildings, objectives, and entries, with entries and objectives easier to see than the terrain. **Crop:** the map frame only. Out of scope: the banners, the bottom strip, and the composition of the page.

## Contract it unlocks

A real admitted map is drawn as a top-down picture, and a preview lifecycle sits around it.

## Seam

- **Picture.** `mapgen::inspect` gains a picture built from a `MapDefinition` plus `SkirmishSites`, in the player palette. It is exposed through wasm. The preview runs in the worker and returns the SVG with the request, map and sites (D39). It is oriented so Blue's edge faces left (D23).
- **Preview controller.** It wraps `admitBattle` (`web/src/battle/prepare/admission.ts`):
  - a newer request cancels an older one;
  - states are loading, retrying, ready and failed;
  - a finished preview closes its worker (D14).
- **Lab route.** A component lab route renders the picture alone for 1v1, 3v4 and 5v5 at S, M, L and XL.

## What the human can run or see

The picture lab route.

## Red tests first

- The picture contains every entry and objective of the admitted sites.
- A stale preview result, arriving after a newer request, is dropped.
- A seed-dependent refusal goes to retrying, then ready.
- A deadline gives failed.

## Visual gates

1. Use `game-ui` first.
2. `compare-screenshots` against the map treatment in [`../visualizations/lobby-directions.html`](../visualizations/lobby-directions.html), Direction 3.
3. `screenshot-critique`, unprimed, as the last check before acceptance.
4. A non-blocking `preview-shots` review: give the user about 5 minutes. If they're silent, decide on the evidence, record why in `choices.md`, close the shots and continue.

## Delegated

SVG versus canvas in the page; simplification level per size; marker glyphs, within `game-ui`.

## Stays green

Mapgen hashes: the picture is read-only.

## Feedback that would change it

The user finding entries or objectives hard to read.
