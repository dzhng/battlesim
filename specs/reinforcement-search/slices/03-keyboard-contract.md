# Slice 3: Keyboard contract

## Contract this unlocks

Typing a query and pressing Enter buys the sole matching family's first
available variant and arms placement; Escape clears then closes; focus stays in
the field through pointer use; the pause menu never opens on those Escapes.

## API seam

Edit `web/src/battle/present/purchasePicker.tsx`:

- The field's `onKeyDown`:
  - **Enter** → if `enterChoice(filterCards(available, query), facts)` is
    non-null, `onChoose(card.id)`; else no-op.
  - **Escape** → `event.preventDefault()`; non-empty query clears it, empty
    query closes the panel and returns focus to the Reinforcements button
    (a ref on it).
- `PurchasePicker` owns the button ref and the close-through-Escape path, so
  focus return is one place.
- No visible confirm affordance: the user judged an `ENTER` hint unnecessary,
  so the field carries the behavior alone.

`facts` is `{ credits: match.credits, occupiedSlots: match.occupiedSlots,
maxUnits: match.maxUnits, finished: match.phase === "finished" }`.

## What the human can run

Open the picker, type a family, press Enter, and place the ghost. Reopen,
press Escape twice, and confirm the pause menu did not open.

## Verification

- `web/tests/purchasePicker.test.tsx`: Enter chooses the sole family's first
  purchasable variant; Enter no-ops with two families; Enter no-ops when the
  sole family has no purchasable variant; a variant-name query buys that exact
  variant, not the family's first available; Escape clears a query and does not
  close; a second Escape closes and returns focus to the Reinforcements button;
  the field keeps focus after a card hover.
- A pause-precedence test: with the picker open and a query, Escape clears and
  the pause menu does not open (assert via the component's own state or the
  absence of the dialog).
- `web/scenes/unit-roster.mjs`: open the picker, type `abrams`, press Enter,
  confirm the ghost, and assert the purchase reserves a slot. This is the real
  proof that Enter selects.

## Must stay green

Slice 2's filtering tests and the existing `unit-roster.mjs` flow. The
`PurchasePlacementControl` contract is unchanged; Enter calls the same
`onChoose` a click does.

## Delegated

- Whether focus return uses `autoFocus` or an explicit `.focus()`.

## Feedback that would change this slice

The user has already dropped the visible `ENTER` hint as unnecessary; the
Enter key itself stays, as the original request asked for it. If Escape should
always close the panel (never clear first), the README's Escape contract
changes first.
