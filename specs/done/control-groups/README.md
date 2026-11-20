# Control groups, range-select and the fire blink

**Status:** shipped. **Updated:** 2026-11-16.

The army deck, the bar of unit cards along the bottom of the battle, is where a
player manages a large army without hunting for units on the map. Three things
make that work:

- Shift-click selects a run of cards.
- Ctrl+0–9 stores the selection as a control group, which gathers at the
  deck's left under a white bracket.
- A card blinks when its unit is shot at (yellow) or hurt (red), so the player
  sees where the fighting is from the bar alone.

The [decisions ledger](decisions.md) holds every decision and who made it.

## Contracts

- **Groups and range-select are the client's alone.** No command,
  observation, digest or replay knows them. A group is a stored selection, so
  the selection's owner (`useUnitControl`) keeps it.
- **The blink adds one observation fact.** Each own unit publishes `shot_at`
  (layout column `shotAt`, `OwnUnitView.shotAt`). It is true on a tick when an
  enemy round aimed at the unit passed within the round's suppression radius,
  the same test as the "under fire" bark, or struck it. Like shouts, it stays
  out of the battle digest. It is derived from digested state (the target each
  round carries, `Round.aimed_at`), so replays reproduce it. Damage needs
  nothing new, because own health is published every tick.

## Why it works this way

- **Conventions, not invention.** The player chose RTS and file-list
  conventions throughout: Ctrl+N replaces a group, Shift+N adds to it, N
  selects it and N twice centres on it. On cards, Shift-click is a run and
  Cmd/Ctrl-click a toggle. The Mac's Ctrl-click is its right-click, hence
  Cmd. Ctrl+digit is the browser's tab switch; the battle takes it while it
  takes orders (a live client, input enabled), through a key check of its own
  (`groupKeyPress`), leaving the shared `isGameKey` guard alone. A replay
  leaves it to the browser.
- **Grouping never moves a card.** The bracket is drawn on the cards' outer
  edge and across the gaps, never in added space, so the deck's 68 px pitch
  and its row count stay what they were. The cost, accepted, is that groups
  sit one card gap apart.
- **No gaps in the deck.** It is one sequence filled top to bottom, then
  across: groups 1–9, then 0, then the rest, then waiting reinforcements. A
  group never forces a new column, so a part-filled column's slot takes the
  next card. That is why a bracket is drawn per card, on each side whose
  neighbour is outside the group: an L, or a group across columns, needs no
  case of its own. The player's reference bar shows the same shape.
- **A one-tick fact is read every publication.** React draws at most one
  observation a frame, so every decoded publication reaches the blink through
  the session's `onDecoded` (`BattleView` hands it on, after the session's own
  audio and voices). Its waits are counted in ticks, so
  the blink follows the battle, not the frame rate.
- **The blink is rate-limited.** A card never repeats a colour within a wait,
  a running blink finishes unless red cuts into yellow, and a blink leaves the
  feed once it has run. Sustained fire becomes a steady pulse instead of a
  strobe, and a card the deck moves never replays an old blink.

## Invariants

- Exactly one owner of where a card sits and how its bracket is drawn:
  `deckSlots`. The CSS only places what it returns.
- A unit is in one group at most. Groups keep unit ids and are filtered to the
  units still present. Ids are never reused in a battle, so the dead need no
  pruning. A squad aboard a carrier stays in its group but is never selected.
- Only a round fired at a unit carries `aimed_at`. Area fire at a contact or a
  ground point never blinks yellow.
- `shot_at` never enters the digest. A change that alters any battle digest
  has changed behaviour.
- The blink's timing, wait, colour depth and glow are tuning keys
  (`presentation.hud.fire_blink`). The digit's ink is
  `presentation.hud.group_ink`. Code holds none of their values.

## Where it lives

- Selection and groups: `web/src/battle/input/controlGroups.ts` (keys,
  `storeGroup`, `liveGroups`, `nearestCentre`) and `useUnitControl.ts`
  (`pressGroup`, `groups`, `deleteGroup`). The camera move arrives as
  `UnitControlEvents.onCenter` from `BattleView`'s `useUnitFocus`.
- The deck: `web/src/battle/present/armyDeck.tsx` (range-select, slots, tabs),
  `deckLayout.ts` (`deckSlots`), and the `.hud-army-slot` and
  `.hud-army-blink` rules in `web/src/hud.css`.
- The blink: `web/src/battle/present/fireBlink.ts` (`FireBlinks`,
  `playBlink`).
- The flag: `OwnUnit.shot_at` in `crates/contract/src/observation.rs`, and
  `shouts::NearRounds` in `crates/sim/src/shouts.rs`.
- Tests: `web/tests/armyDeck.test.tsx`, `controlGroups.test.tsx`,
  `deckLayout.test.ts`, `fireBlink.test.ts`, and `crates/sim/tests/shot_at.rs`.
  The deck workbench (`/labs/panels`, "Review battle deck") shows a "Control
  groups" case, which the panels scene pictures, and a "Fire blink" button for
  watching the blink by hand (not pictured).
- The Guide's "Selecting units" section teaches both.

## Dead ends

- **A digit set into the bracket's top edge.** It overlapped the card above
  whenever the deck wrapped. It moved inside the group's first card.
- **Groups starting a new column.** That left holes, which the player ruled
  out.
- **The glow drawn over the card.** That is how the approved mock drew it, but
  it muddied the strength meter just when a unit is hurt, and tinted the
  bracket. It lies behind the card's face now (a needs-user entry in the
  ledger).
- **CSS keyframes for the blink.** Keyframe percentages could honour only the
  sum of `on_s` and `off_s`, so the blink is a Web Animation.
- **A per-round pass sound.** The game has none; the yellow blink is silent.

## Visual provenance

- [`assets/user-reference-card-bar.png`](assets/user-reference-card-bar.png):
  the player's screenshot of another game's grouped card bar. It set white
  group outlines, numbered groups, filling columns top to bottom, and no
  holes.
- [`assets/bar-reference.html`](assets/bar-reference.html): the approved mock
  of the deck, its brackets, tabs and fill order.
- [`assets/blink-reference.html`](assets/blink-reference.html): the approved
  mock of the rising glow, at the player's settings.
- [`assets/walk-map.html`](assets/walk-map.html): the planning walk's map,
  with every question and how it closed.

## Open

Whether Windows Chrome lets a page stop Ctrl+1–9 from switching tabs (D4)
needs a Windows machine. Until someone checks, it is a recorded gap.
