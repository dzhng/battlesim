# Slice 03: publication and the TypeScript mirror

**Decisions:** D31, D32, L5.

## Contract it unlocks

The per-side observation carries a list of players and owned units, end to end, into the browser.

## Seam

- **`crates/sim/src/publication.rs`** replaces `readyBlue`/`readyRed`, `credits` and `occupiedSlots` with a players table: index, id, controller, ready, credits, unit count, cap, and pending purchases with their owner.
  - `OWN_FIELDS` gains `owner`, as a `PlayerIndex`.
  - Report totals and events are per player; `MatchUnit` gains `owner`.
  - Scores and the result stay per side.
- **Readers** change to match:
  - `publication/reader.rs`;
  - `web/src/battle/sim/observationView.ts`, where `ready` and `scores` stop being pairs;
  - the presentation crate;
  - the lab mocks in `routes/ui.tsx` and `panels.tsx`.
- **Fixtures.** Re-bless `fixtures/parity/publication/codec-vectors.json` and add one 2v2 golden frame.
- **Web HUD.** It reads the viewing player's row from `players`; no other HUD change in this slice.

## What the human can run or see

A 1v1 plays. A 2v2 benchmark battle publishes owners, which can be checked in the feed inspector.

## Red tests first

- A 2v2 frame round-trips through writer and reader, with owners and player rows intact.
- The reader refuses a layout mismatch.
- The HUD picks its own row by player.

## Delegated

Field packing order and limb splitting, as long as the reader's layout check covers them.

## Stays green

Non-skirmish publication bytes, apart from the layout header. Every simulation digest. Publication doesn't affect digests.

## Feedback that would change it

None expected.
