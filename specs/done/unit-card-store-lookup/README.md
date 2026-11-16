# Unit-card store lookup

**Status:** built 2026-11-16 on `feature/unit-card-store-lookup`.

## Why

The army deck already knows which unit a player owns. Buying another copy meant
opening Reinforcements and finding that unit again by group or name.
Right-clicking a card now opens the store on that exact variant: the filter
holds its name, its info card is already open, and Enter buys it. Presentational
only: no observation, rule or digest changes.

## Why this shape

- **Right-click, not another button.** The card is already the unit's handle in
  the HUD. A right-click uses that handle the same way the pointer's other
  gestures do, without adding a control to every card.
- **Exact by card id.** A variant name can repeat across families, or within one
  (`Base`). The lookup holds the card id, so the filtered family and its info
  card can only be the unit the player asked for, never a cheaper sibling.
- **The filter is the lookup term.** The field holds the family and variant
  names, so the state explains itself. Opening leaves the keyboard to the game,
  as every other shop opening does; the info card is already open, so the next
  click buys it.
- **One shot.** The page clears the request as soon as the store takes it, so a
  remount cannot replay it; the same card can be asked for again because each
  request carries a new serial. Closing the store clears what is left. A plain
  open still starts fresh, first group and empty filter.
- **No second purchase path.** A lookup buys through the shop's existing
  `onChoose`, so placement and every purchase gate stay where they were.

## Invariants

- A right-click does not select, centre, or otherwise change the selection; it
  only asks the store for the card's unit.
- In lookup mode the grid and flyout hold one family and one variant, and both
  the family card and Enter (once the player is in the field) buy that exact
  card.
- Opening a lookup never takes the keyboard; the game's keys keep working.
- The lookup obeys the battle's input gate: it cannot open behind the pause veil
  or a finished match.
- A plain Reinforcements open never inherits a previous lookup.

## Pointers

- Gesture and hit target: `onLookup` in
  `web/src/battle/present/armyDeck.tsx`.
- Request state: `purchaseLookup` in `apps/battle-lab/src/BattleView.tsx`; the UI
  gallery wires the same path in `apps/battle-lab/src/routes/ui.tsx`.
- Store state, exact pool and Enter choice: `PurchaseLookup`, `revealed` and
  `exact` in `web/src/battle/present/purchasePicker.tsx`.
- Tests: `web/tests/armyDeck.test.tsx` and
  `web/tests/purchasePicker.test.tsx`.
- Scene: the lookup checks and `purchase-lookup.png` evidence in
  `web/scenes/ui.mjs`.

## Visual provenance

`assets/store-lookup.png` is the final capture from `web/scenes/ui.mjs` at
1280x800. The card, its info card, the search row and the deck are the existing
approved components; the lookup only changes which family is shown and opens
that family's info card. The approved search state
(`web/scenes/baselines/ui/purchase-search.png`) is the same store without the
lookup, and every existing store baseline stayed at zero pixels different. A
side-by-side of the two states puts the difference on the info card and the
filter text alone.

An unprimed critique of the first lookup shot flagged the auto-selected filter
as a browser text selection. The lookup now opens without taking the keyboard,
so the field keeps the store's ordinary text treatment and the already-open
info card carries the cue. A second pass on that shot found the unfocused
filter's rule too quiet, so the lookup marks the rule active without taking
focus. The final shot was checked against the approved search state; the
difference is the open info card and the exact term.
