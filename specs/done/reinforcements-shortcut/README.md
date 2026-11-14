# Reinforcements shortcut

**Status:** shipped. **Updated:** 2026-11-14.

## Why

The shop is a click target in the HUD, so a player buying mid-fight has to
leave the battlefield with the pointer and find the button. `Tab` opens it
from the keyboard. Presentational only: no observation, rule or digest changes.

## Why Tab

One key runs the whole shop: `Tab` opens it, `Tab` again puts the cursor in
the search so the player types a name at once, and `Tab` a third time leaves
the field with the shop still open, so the game keys work again. Further
presses alternate between the field and the game. A letter cannot do this: in
the field it has to type. `Tab` also costs no command or camera key (the
cluster spends `C R F G T B U H`, the camera `W A S D Q E Z X`).

Owning `Tab` also keeps the browser from walking focus onto HUD buttons, which
a game should never do: a focused button would take `Enter` and `Space`, and
draw a focus ring on the battlefield. The HUD is pointer-driven, its details
show on hover, and the menus keep their own keyboard focus.

## Why the button keeps its name

A standalone HUD control is read by its name alone; the menu button is the
precedent, `aria-label="Menu"` with `title="Menu (Esc)"`. The button therefore
reads `Reinforcements`, while `aria-keyshortcuts="Tab"` and its tooltip carry
the key, and "Buying units" teaches it. An inline `(Tab)` would be the one
button that spells its key, and it would re-render every approved shot that
shows the deck for a hint the tooltip already gives.

## Invariants

- **The field keeps its letters.** No letter belongs to the shop, so every
  roster name (`Viper`, `CV90`, `SEP v2`) types. `Tab` never closes the shop:
  `Esc` clears the query and then closes, and the button always closes.
- **Opening never takes the keyboard.** The shop opens with focus where it
  was, so command and camera keys keep working while it is open. Only the
  second `Tab` takes them, and the third gives them back. `Esc` outside the
  field closes the shop ahead of the pause menu.
- **Tab and Shift+Tab are one key.** Both run the cycle, so neither walks
  focus. Ctrl, Cmd and Alt+Tab stay the browser's and the system's; a `Tab`
  typed into another text control does not open the shop.
- **One owner.** `PurchasePicker` owns `open`, its button and its key. It is
  not in `commandBindings.ts`, which is the table `useUnitControl` dispatches.
  `N` stays in `LabViewport` for the same reason: it drives the viewport's
  `jumpToEvidence`.
- **The key dies with the battle.** `BattleView` passes the same
  `!pause.open && !matchFinished && loadStage === "playable"` expression that
  `LabViewport` receives as `inputEnabled`, so `Tab` cannot open the shop
  behind the pause veil or the result overlay, and the open shop gives up `Tab`
  and `Esc` to a menu over it.
- **The name is the button.** `aria-label="Reinforcements"` and the visible
  text are exactly `Reinforcements`; `aria-keyshortcuts` carries the shortcut
  and `title` names it on hover, as `MenuButton` does. Tests find the button by
  its name, not by a hint.

## Pointers

- Key, toggle, tooltip: `REINFORCEMENTS_KEY`, `enabled` in
  `web/src/battle/present/purchasePicker.tsx`.
- Gate: `battleInput` in `apps/battle-lab/src/BattleView.tsx`.
- Teach: "Buying units" in `apps/battle-lab/src/Tutorial.tsx`.
- Tests: `web/tests/purchasePicker.test.tsx` (the `Tab` cycle, `Escape`
  outside the field, the shop's keys while the battle is covered).

## Dead ends

- **Enter.** It already activates focused buttons and buys in the field;
  binding it globally either steals that from every HUD control or double
  fires on a focused card.
- **P and backtick.** Rejected for the borrowed meanings above, not for a
  code conflict.
- **V.** It shipped first, chosen as the free letter in the command cluster.
  Searching then needed a second key or the pointer, and opening put the
  cursor in the field, which took the game keys away for as long as the shop
  was open. Replaced by `Tab` (2026-11-14).
- **Putting `N` and the shop key in `commandBindings.ts`.** That table's keys are
  `preventDefault`ed by `useUnitControl`; a key owned elsewhere would be
  claimed by a module that cannot act on it.
- **Closing the shop with a letter from the search field.** It would make that
  letter untypeable and units unfindable by name.

## Visual provenance

The feature changes no approved shot: the picker and the deck row render as
they did before it, so every baseline is the upstream one. An earlier inline
`(V)` hint widened the row and re-blessed every shot that shows the deck; it
was removed for that cost and because no other standalone control spells its
key. `assets/reinforcements-shortcut.gif` shows the picker as it first shipped,
on `V`: `V` opens, typing filters to the Tank family, `Esc` clears and closes,
`V` reopens.
