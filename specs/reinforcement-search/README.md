# Reinforcements search

A type-to-filter field in the Reinforcements picker. A player who already knows
a unit's name reaches it without hunting the category tabs; when the search
narrows to one family, Enter buys its first available variant and arms
placement.

Presentation-only: no simulation, protocol, observation or digest change.

## Goal and context

`PurchasePicker` (`web/src/battle/present/purchasePicker.tsx`) is the
Reinforcements button and credits that toggles `PurchaseMenu`: a card grid of
unit *families* under a row of category tabs (REC/INF/VEH/SUP/HEL/AIR), with a
hover flyout stacking that family's *variants*. Clicking a variant arms
physical placement (`PurchasePlacementControl.choose`) and closes the panel.

The tabs are recognition-by-browse. A filter is recognition-by-recall and
complements them. It follows the build/shop-panel convention: while the panel
is open, typing filters, and search spans the whole faction roster rather than
the current tab.

## Contracts

- The search field is a **real `<input type="text">`**. This is what lets the
  existing typing guard, `keyGuard.isTypingTarget`
  (`web/src/battle/input/keyGuard.ts`), exempt it from command keys
  (`C/R/F/G/T/Space/Backspace`) and held camera keys (WASD). A styled `div`
  would leak keystrokes into the game.
- The field sits at the **right end of the category-tab row**, in a new
  `.hud-purchase-filter` footer row. The `role="tablist"` keeps only tabs; the
  field is its sibling.
- Search **spans all categories**; while a query is present the tabs dim and
  stop switching.
- Matching is case-insensitive, whitespace-token AND, substring over
  `roster.family_name` and `roster.variant`. Not internal ids, not
  descriptions.
- **Enter** acts only when the filtered result is exactly one family: it buys
  that family's first currently-purchasable variant, exactly as clicking the
  sole family card would. More than one family, or none purchasable, is a
  no-op. No visible confirm hint is drawn; the behavior lives in the field.
- `onChoose` keeps its contract: arm placement, close the panel.
- The icon is a generated `hud/search` magnifier
  (`packages/scene-assets/src/icons.ts` → `assets/icons/hud/search.svg`), never
  a font glyph.

### Decisions frozen by the user

1. **Enter rule:** first available (the sole filtered family's first purchasable
   variant), not the unique-purchasable-match rule.
2. **Placement:** right end of the tab row.
3. **Match fields:** family name + variant.
4. **Icon:** generated magnifier.
5. **Ladder:** slices 2 and 3 stay separate.

## Slice graph

| # | Slice | Unlocks | Depends |
|---|---|---|---|
| 1 | [Filter core](slices/01-filter-core.md) | Matching + Enter rule as a tested pure seam | - |
| 2 | [Picker filtering](slices/02-picker-filtering.md) | A playable, filterable picker | 1 |
| 3 | [Keyboard contract](slices/03-keyboard-contract.md) | Enter/Escape/focus behavior | 2 |
| 4 | [Visual closeout](slices/04-visual-closeout.md) | Styled, judged, pinned | 2, 3 |

## Invariants

- One owner for "can this card be bought now": `cardPurchasable` in
  `purchaseFilter.ts`, shared by the family card and the variant button. The
  old duplicated condition is deleted.
- One owner for query matching and the Enter choice: `purchaseFilter.ts`.
- The picker still opens fresh: empty query, first category.
- Escape in the field calls `event.preventDefault()` before clearing or
  closing, so `PauseMenu`'s window listener (`e.defaultPrevented`) never opens
  the pause menu. `useBattleSession`'s capture Escape only fires once a
  purchase is armed, after the panel has closed.
- Clearing the field returns to the remembered category; closing and reopening
  resets it.
- No new panel, colour or free-floating surface.

## Review map

- Pure logic: `web/tests/purchaseFilter.test.ts`.
- Panel behavior: `web/tests/purchasePicker.test.tsx`.
- End-to-end: `web/scenes/unit-roster.mjs` (open, type, Enter, confirm ghost).
- Visual: `web/scenes/ui.mjs` captures the search state; approved baselines
  live under `web/scenes/baselines/ui/`. `screenshot-critique` is the last step
  on any visual slice, and `compare-screenshots` judges the new shot against
  the existing `purchase-veh` baseline.

## Next Agent Prompt

**Status (updated as work lands):** all four slices shipped. Unit tests,
typecheck, lint and format are green; `unit-roster` passes in the browser; the
`ui` fixture passes with the new `purchase-search` baseline and the intended
`purchase-*` footer changes blessed. Three unprimed `screenshot-critique`
passes drove two fixes (typed text fell back to a system font; the field read
as a boxed web input); see `slices/04-visual-closeout.md#critique-findings`.

Next pickup: none. Close the spec with `close-spec` once the change is
reviewed and committed. Before ending any pass, update this status block.

Global TODO:

- [x] [1: Filter core](slices/01-filter-core.md)
- [x] [2: Picker filtering](slices/02-picker-filtering.md)
- [x] [3: Keyboard contract](slices/03-keyboard-contract.md)
- [x] [4: Visual closeout](slices/04-visual-closeout.md)
