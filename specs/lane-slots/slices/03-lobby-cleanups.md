# 03 · Lobby cleanups

**Depends on:** nothing (lane-independent). **Changes digests:** no.

## Contract

- The status line says what the preview is doing with no seed: "Ready",
  "Generating map", "Redrawing map · try N", "No map" (D10). The seed stays in
  the Deploy link, so the battle is still the map the player saw.
- The size reason line is gone; sizes below the floor stay disabled (D11).
- Region is a row of tabs (Random · Taiwan · New York · Paris) using the
  existing `Choice` control, like Match, Map and Size (D12).

## Seam

`apps/battle-lab/src/lobby/Lobby.tsx`:

- `PreviewStatus`: drop the seed span.
- `SizeChoice`: drop the reason child and its `largest` maths.
- The region block: replace the `Pick` with `Choice` (`RANDOM` plus `REGIONS`,
  labels via `spoken`). `Pick` stays; faction and controller use it.
- Delete the now-unused `.lobby-seed`, `.lobby-reason` and `.lobby-region` CSS
  in `web/src/menu.css`.

## What a human can see

The lobby strip from the [before shots](../assets/): no seed, no reason line,
Region as four tabs.

## Verification

- Rewrite `web/tests/lobbyScreen.test.tsx` as behaviour checks: status
  `data-status` is `ready` once the map is ready; the Deploy link names the
  admitted seed; sizes below the floor are disabled and can't be chosen;
  choosing a region tab rebuilds the map in that region (from `aria-checked`
  and the next preview request, not label text). Lines that pinned removed
  text: `:77`, `:93`, `:155-157`.
- Scenes: `web/scenes/generated.mjs:83` reads the seed from the Deploy href;
  `:90` reads the region from the checked tab; `web/scenes/lobby.mjs:171`
  checks that closed sizes are disabled, not that a reason shows;
  `web/scenes/ui.mjs:19` drops its `.lobby-seed` entry.
- Re-bless `web/scenes/baselines/ui/menu-skirmish.png` after judging it.
- Visual gates: `game-ui` first; `compare-screenshots` before against after;
  `screenshot-critique` last; `preview-shots`, non-blocking (README
  [Verification](../README.md#verification)).

## Delegated

Tab widths and spacing within the existing `Choice` look.

## Must stay green

Lobby model tests; Deploy opens the previewed map; digests.
