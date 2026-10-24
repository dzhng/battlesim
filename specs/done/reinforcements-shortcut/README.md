# Reinforcements shortcut

**Status:** shipped. **Updated:** 2026-10-22.

## Why

The shop is a click target in the HUD, so a player buying mid-fight has to
leave the battlefield with the pointer and find the button. `V` opens it from
the keyboard. Presentational only: no observation, rule or digest changes.

## Why V

The command cluster already spends `C R F G T B`; the camera reads
`W A S D Q E Z X`, and `Shift` is the camera boost and the order-queue
modifier. Every remaining mnemonic carried a borrowed meaning: `P` is patrol
in this genre (and pause to some players), `Enter` is the browser's activation
key for every focused control, and `` ` `` opens a developer console in the
games players come from. `V` is free, sits in the cluster, and names one thing
here.

## Why the button keeps its name

A standalone HUD control is read by its name alone; the menu button is the
precedent, `aria-label="Menu"` with `title="Menu (Esc)"`. The button therefore
reads `Reinforcements`, while `aria-keyshortcuts="V"` and its tooltip carry
the key, and "Buying units" teaches it. An inline `(V)` would be the one
button that spells its key, and it would re-render every approved shot that
shows the deck for a hint the tooltip already gives.

## Invariants

- **The field keeps its letters.** `isGameKey` rejects a press whose target is
  a text control, so while the search field has focus `v` types. That is not
  optional: `Viper`, `Venom`, `CV90`, `VBCI`, `VBL` and `SEP v2` are roster
  names. `V` therefore opens only; `Esc` clears the query and then closes, and
  the button always closes.
- **One owner.** `PurchasePicker` owns `open`, its button and its key. It is
  not in `commandBindings.ts`, which is the table `useUnitControl` dispatches.
  `N` stays in `LabViewport` for the same reason: it drives the viewport's
  `jumpToEvidence`.
- **The key dies with the battle.** `BattleView` passes the same
  `!pause.open && !matchFinished && loadStage === "playable"` expression that
  `LabViewport` receives as `inputEnabled`, so `V` cannot open the shop behind
  the pause veil or the result overlay.
- **The name is the button.** `aria-label="Reinforcements"` and the visible
  text are exactly `Reinforcements`; `aria-keyshortcuts` carries the shortcut
  and `title` names it on hover, as `MenuButton` does. Tests find the button by
  its name, not by a hint.

## Pointers

- Key, toggle, tooltip: `REINFORCEMENTS_KEY`, `enabled` in
  `web/src/battle/present/purchasePicker.tsx`.
- Gate: `battleInput` in `apps/battle-lab/src/BattleView.tsx`.
- Teach: "Buying units" in `apps/battle-lab/src/Tutorial.tsx`.
- Tests: `web/tests/purchasePicker.test.tsx` (`V opens the shop and stays a
  search character in the field`, `V does nothing while the battle is covered`).

## Dead ends

- **Enter.** It already activates focused buttons and buys in the field;
  binding it globally either steals that from every HUD control or double
  fires on a focused card.
- **P and backtick.** Rejected for the borrowed meanings above, not for a
  code conflict.
- **Putting `N` and `V` in `commandBindings.ts`.** That table's keys are
  `preventDefault`ed by `useUnitControl`; a key owned elsewhere would be
  claimed by a module that cannot act on it.
- **Closing the shop from the search field with `V`.** It would make `v`
  untypeable and those units unfindable by name.

## Visual provenance

The feature changes no approved shot: the picker and the deck row render as
they did before it, so every baseline is the upstream one. An earlier inline
`(V)` hint widened the row and re-blessed every shot that shows the deck; it
was removed for that cost and because no other standalone control spells its
key. `assets/reinforcements-shortcut.gif` shows the production picker in the
UI gallery: `V` opens, typing filters to the Tank family, `Esc` clears and
closes, `V` reopens.
