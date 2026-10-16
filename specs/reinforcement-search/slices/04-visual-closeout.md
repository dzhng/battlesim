# Slice 4: Visual closeout

## Contract this unlocks

The field reads as part of the holo-tactical panel, not as a web input, and the
new state is pinned by an approved picture.

## API seam

- `web/src/hud.css`: `.hud-purchase-search` styled from the existing
  `.hud-panel input[type="text"]` rules: glass fill, thin accent border,
  letter-spaced uppercase placeholder, accent focus glow, magnifier at the
  start. No new colour.
- `packages/scene-assets/src/icons.ts`: add `search` to `HUD_ICONS` (a circle
  and handle on the 24 × 24 grid); run `bun run --cwd web asset -- icons` to
  write `assets/icons/hud/search.svg`. `web/tests/icons.test.ts` fails until
  the file is current and orphan-free.
- `web/scenes/ui.mjs` + `web/scenes/baselines/ui/`: capture and approve
  `purchase-search`.

## What the human can run

`?shot=purchase`, type, and compare against `purchase-veh`.

## Verification

1. `bun run --cwd web asset -- icons`, then `web/tests/icons.test.ts` green.
2. The `ui.mjs` visual run matches the newly approved `purchase-search`
   baseline and every existing `purchase-*` shot still matches (or is
   deliberately re-approved and reviewed in the diff).
3. `screenshot-critique` unprimed on the search shot, last.
4. `compare-screenshots` against `purchase-veh` for the footer row's density
   and the field's legibility.

## Slice variable

The added footer row and field: density, legibility and silhouette against the
panel edge. Out of scope: the grid, cards, flyout and tab styling, which are
not changed.

## Must stay green

Every pinned `purchase-*` picture except the intended footer change; the
`ui.mjs` geometry checks (panels hold their content, cards are the deck card's
size, the flyout stands on its card, one 3 px gap).

## Delegated

- Whether the magnifier sits inside the field or beside it.
- The hint's exact glyph/word.

## Feedback that would change this slice

A reference image for the intended field would turn this into a
compare-screenshots-driven match; none exists today, so the target is the
panel's own approved look.

## Critique findings

Three unprimed `screenshot-critique` passes on `purchase-search.actual.png` and
its crops. Findings and resolutions:

- **Typed text fell back to a system font** (the HUD face is caps). Resolved
  with `text-transform: uppercase` on the field; the final pass measured the
  value's face, size and tracking as consistent with the tabs and `ENTER`.
- **The field read as a boxed native input**, and its value was the only
  glowing text on the panel. Resolved by restyling to a single glowing
  underline with the value carrying no halo; the final crop reads as HUD
  chrome.
- **Tabs dim to 0.32 while a search is active.** By design: the shared HUD
  `button:disabled` rule marks them inert, because a search spans all
  categories and the tabs no longer choose scope. Not changed.
- Minor/not changed: ~1 px vertical centring, the empty width of a short
  query's underline, and the panel's existing diagonal corner brackets.
- The visible `ENTER` hint was removed at the user's request as unnecessary;
  the Enter key behavior stays. The `purchase-*` baselines were re-blessed
  after the removal.

The intended change is localized: the before/after diff shows only the footer
row's right side; the card grid, tabs and flyout do not move. The ten changed
`purchase-*` baselines were re-blessed; nine unrelated baselines that bless
rewrote within tolerance were restored from LFS so the diff holds only the
intended pictures.

