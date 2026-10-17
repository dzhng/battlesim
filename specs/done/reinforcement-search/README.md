# Reinforcements search

`PurchasePicker` (`web/src/battle/present/purchasePicker.tsx`) carries a
type-to-filter field, so a player who knows a unit's name reaches it without
hunting the category tabs. Presentational only: no observation, rule or digest
changes.

## Why

The category tabs are recognition by browsing. A player who already has a unit
in mind should not have to remember which tab holds it, so the field is
recognition by recall and complements the tabs instead of replacing them.

Three shape decisions follow:

- **Search spans every category.** A filter scoped to the shown tab reads as
  broken when the named unit is elsewhere, so while a query is present the tabs
  dim and stop switching.
- **The field is a real `<input>`.** `keyGuard.isTypingTarget`
  (`web/src/battle/input/keyGuard.ts`) already exempts text inputs from command
  keys and held camera keys, so a real input keeps WASD and `C/R/F/G/T` out of
  the game while the player types. A styled `div` would leak them.
- **Enter buys the sole matching family's first available variant**, exactly as
  clicking that family's card would. With several families still shown it
  no-ops, so a broad query cannot buy by accident. A unique-purchasable-match
  rule was rejected because filtering to one family should behave like clicking
  it.

The user chose the placement (right end of the tab row), family plus variant
matching, and the generated magnifier. A visible `ENTER` hint was tried and
dropped as unnecessary.

## Invariants

- **One owner for buyability.** `cardPurchasable(card, facts)` in
  `web/src/battle/present/purchaseFilter.ts` answers "can this be bought now?"
  for the family card and the variant card alike, from the observed credits,
  slots, disabled reason and battle phase. The duplicate condition that
  predated this change is gone.
- **One owner for matching and the Enter choice.** `filterCards(cards, query)`
  matches every whitespace token, case-insensitively, as a substring of
  `roster.family_name` or `roster.variant`; `enterChoice(matches, facts)` returns
  the sole family's first purchasable variant, or null.
- **Escape never reaches the pause menu.** The field's handler calls
  `event.preventDefault()` before clearing the query or closing and returning
  focus to the Reinforcements button, so `PauseMenu`'s window listener
  (`e.defaultPrevented`) stays out. `useBattleSession`'s capture Escape fires
  only while a purchase is armed, after the panel has closed.
- **The picker still opens fresh:** empty query, first category. Clearing the
  query returns to the remembered category.
- **The field is HUD chrome**, one glowing underline on `.hud-purchase-search
  input` in the `.hud-purchase-filter` row (`web/src/hud.css`), not a boxed
  browser control.

## Pointers

- Matching, buyability, Enter choice: `web/src/battle/present/purchaseFilter.ts`.
- Panel, field, keys, cross-category pool: `PurchasePicker` / `PurchaseMenu` in
  `web/src/battle/present/purchasePicker.tsx`.
- Field styling: `.hud-purchase-filter`, `.hud-purchase-search` in
  `web/src/hud.css`.
- Magnifier: `HUD_ICONS.search` in `packages/scene-assets/src/icons.ts`, written
  to `assets/icons/hud/search.svg`.
- Tests: `web/tests/purchaseFilter.test.ts`, `web/tests/purchasePicker.test.tsx`.
- Scenes: `web/scenes/unit-roster.mjs` (types `abrams`, Enter, ghost, Escape) and
  `web/scenes/ui.mjs` (the `purchase-search` capture).

## Visual provenance

The standard is the picker's own approved look. `assets/purchase-search.png` is
the filtered state and `assets/purchase-veh.png` the browsing state it had to
stay consistent with; the live copies are the `purchase-*` pictures under
`web/scenes/baselines/ui/`. No external reference was used. The before/after
diff is localized to the tab row's right side: the card grid, tabs and flyout do
not move.

Three unprimed `screenshot-critique` passes drove two fixes. Typed text fell
back to a system font because the HUD face is caps-only, fixed with
`text-transform: uppercase`. The field first read as a boxed web input with a
glowing value, fixed by dropping to a single underline and removing the halo.

## Dead ends

- **A header field above the grid.** The variant flyout grows upward from a
  card, so the panel's top edge is space the flyout uses; a header would collide
  with it and re-baseline every purchase shot.
- **A boxed input.** Even in the HUD accent it read as a browser control.
- **A visible `ENTER` hint.** Dropped as unnecessary; the key behavior stays.
- **Unique-purchasable Enter rule.** Rejected in favor of first-available.
