# Slice 02: simulation per-slot ownership

**Decisions:** D1, D12, D13, D20, D41 (re-baseline at 02), D43, D44, D45, D46, L2, L3, L4.

## Contract it unlocks

Each slot is a full player inside the simulation.

## Seam (`crates/sim`)

- **Per-player state.** `Skirmish` holds `players: Vec<PlayerState { wallet, ready, reservations, next_purchase, totals }>` in place of every `[_; 2]`. This deletes slice 01's side mapping.
  - The cap counts the slot's living units plus its reservations. Today it counts by side (`battle.rs:3483`).
  - Refunds go to the owner (`:976`).
- **Ownership.** Purchases spawn with `owner` set to the buyer (`:874`). `Battle::new` installs controllers from `players` at tick 0. `set_controllers` (`:1480`) shrinks to a play-time `transition_controller(slot, kind)`.
- **Commands.**
  - `next_seq` and `order_bucket` are per slot.
  - The slot is required in a skirmish.
  - Ready, ConfirmPurchase and CancelPending are slot-checked (`:1628`).
  - Purchase IDs are scoped to the slot.
- **AI.** One function picks the policy (D45). Every active-AI skirmish slot runs its own `BalancedDoctrine`, and `ai_side` goes.
  - The doctrine reads through a `PlayerLens { frame, me }` whose `own()` and `economy()` filter the side frame to that slot.
  - Orders carry the slot.
  - A refused order fails a debug assertion and appends to the non-digested `ai_receipts()` log.
  - A slot may take a scripted opening buy list as data, which the tournament needs (D46).
  - Behaviour is otherwise unchanged; grouping comes in slice 12.
- **Random streams.** The aircraft arrival delay and the AI RNG mix `PlayerIndex` in (`:752`, `:1145`).
- **Settlement (D43).** Bonus totals are kept per (victim side, killer slot) and split in slot order. `LethalSource` gains the owner (`damage.rs:363`). The pool stays per side.
- **Digest.** The unit digest hashes `owner` (D44).
- **Re-baseline 1 (D41).** Re-bless `fixtures/parity/skirmish-*.json` and record the before and after digests in the commit.

## What the human can run or see

A 1v1 plays; the HUD's credits and cap are the human slot's own. A headless 2v2 all-AI battle runs to a result.

## Red tests first

- A human and two AI slots on Blue issue orders in the same ticks with no `OutOfSequence` and no `RateLimited`.
- An ally cannot order, cancel or ready for another slot.
- Caps are independent: slot A at 30 doesn't block slot B.
- One slot's Ready does not start the match.
- A kill by Blue 2's unit pays Blue 2 and grows Blue's pool. Two killers splitting a capped bonus get the same split whichever order the deaths are processed in.
- Two AI slots with the same doctrine make different seeded choices.
- A 2v2 replay reproduces its digest, including the tick-0 controllers.
- Each AI slot gets exactly one policy; `controller_turn` never runs for it.

## Delegated

Storage layout. Whether `PlayerLens` is a struct or a trait.

## Stays green

Non-skirmish digests: authored encounters, benchmark stress, menu reel.

## Feedback that would change it

None expected; these are rules the user already set.
