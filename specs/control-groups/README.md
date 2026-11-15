# Control groups, range-select and the fire blink

**Status:** planned, slice 1 next. **Updated:** 2026-11-16.

The army deck (the bottom bar of unit cards) gains three things: Shift-click
selects a range of cards, Ctrl+0–9 stores the selection as a control group that
gathers on the left of the deck under a white bracket, and a card blinks when
its unit is shot at (yellow) or hurt (red).

## Contracts

- **Groups and range-select are client-only.** No command, observation, digest
  or replay changes. Group state is the selection owner's
  (`web/src/battle/input/useUnitControl.ts`).
- **The fire blink adds one observation fact.** Each own unit's record in the
  publication says whether an enemy round aimed at it passed near it this tick.
  Like shouts, it stays out of the battle digest, so outcomes, digests and
  replays do not move. Every round records the unit it was fired at. Damage
  needs nothing new: own health is already published every tick.

## Next Agent Prompt

You are building this plan to completion. Read this file, the
[decisions ledger](decisions.md) (every walk decision there is settled), the
[`game-ui`](../../.agents/skills/game-ui/SKILL.md) skill before any visible
change, and [`tweak-mechanics`](../../.agents/skills/tweak-mechanics/SKILL.md)
before slice 5's simulation work. The approved looks are the mocks in
[`assets/`](assets/): `bar-reference.html` for the deck and brackets,
`blink-reference.html` for the glow, and the player's own reference
screenshot `user-reference-card-bar.png`.

Pickup: slice 1. Slices 1–4 share the deck and run in order on one agent;
slice 5a (simulation) depends on nothing and runs in parallel; 5b needs 3
and 5a.

## Slices

1. **Range-select on cards.** Shift-click from the anchor, Cmd/Ctrl-click
   toggles one card (D2, L6). Test first in `web/tests/armyDeck.test.tsx`; the
   existing Shift-toggle test flips meaning.
2. **Control groups.** Groups in `useUnitControl`: Ctrl+N, Shift+N, N, N twice
   centres (D1, D3–D5, S1–S4, L3–L5, L7–L9). Ctrl+digit gets its own check;
   the shared `isGameKey` guard stays as it is. The page passes the camera move
   in. Restart clears.
3. **The deck's layout and brackets.** One column-major sequence (D13, D15),
   per-card white bracket edges on the cards' outer edge (D6, D8, D14, L2), the
   number tab (D12) and the hover × tab (D7). The deep-blue digit is a new
   `presentation.hud` colour. The deck's row count stays `count / capacity`;
   the partial-last-row offset goes. Screenshot gate: before and after, an
   unprimed critique, compared with `bar-reference.html`.
4. **Tutorial.** "Selecting units" teaches range-select and groups (D9, L10).
5. **The fire blink.**
   - **5a, simulation:** rounds carry their target; the publication's own unit
     gains the "aimed round passed near" flag (B7), outside the digest; the
     reader and `observationView.ts` expose it. Digests and replays unchanged.
   - **5b, client:** per-publication blink detection (a flag for yellow, a
     health drop for red, B2, B8), the per-colour wait and the cut-in (B4,
     B5), and the rising glow (B1, B3, B6) whose timings and mix are
     `presentation.hud` keys. Screenshot gate as in slice 3.
6. **Closeout.** Run everything once, the whole-spec review, consolidate the
   ledger, then `close-spec`.

## Open

- **Windows Chrome and Ctrl+1–9 (D4).** Whether a page's `preventDefault`
  stops Chrome on Windows from switching tabs needs a Windows machine. Until
  someone checks, it is a recorded gap, not a blocker.
