# Slice 05: simulation entry owner and dispatch

**Decisions:** D8′, D15, D37, L1, L9. **Needs:** 02 and 04.

## Contract it unlocks

Every arrival and every return uses one entry rule, and no player can stall another's arrivals.

## Seam

- **`crates/sim/src/entries.rs`** (new):
  - `select(sites, side, kind, point, route_time) -> EntryIndex` picks the shortest drive for that unit type; ties go to the centre.
  - `retire_at(...)` decides whether a returning unit has reached an entry.
  - Jets go through `air::edge` from the centre entry (D15).
- **Callers.** Every caller uses the module:
  - dispatch (`battle.rs:744`);
  - helicopter fuel return (`:692`);
  - retirement (`:946`);
  - the AI's entry (`:2430`);
  - refunds (`:3529`);
  - purchase preview (`:3963`);
  - `air::edge::back_edge` (`edge.rs:15`);
  - the tournament runner (`runner.rs:98`).
- **Pending purchases (D37).** `PendingPurchase.entry` is chosen once at reserve time. Preview and dispatch read it.
- **Dispatch (L9).** Each entry has its own clock, and players take turns at each entry. A player's purchase for one entry never waits behind its purchase for another entry.

## What the human can run or see

A 3v4 all-AI benchmark battle in the browser. Units arrive at the entry nearest where they were ordered.

## Red tests first

- A ground unit ordered far right arrives at the right entry, and so does a helicopter. A jet arrives opposite the centre entry.
- A unit that came in on the left returns and retires at its nearest entry, and its refund is paid.
- A blocked unit at the left entry doesn't delay the centre, and doesn't delay another player.
- No `.find(|e| e.side == side)` lookup over entries remains anywhere in `crates/sim` or the tournament.

## Delegated

The order players take turns in at an entry, as long as it is deterministic. Clock data structure.

## Stays green

Non-skirmish digests. 1v1 skirmish digests, since one entry per side gives the same choices.

## Feedback that would change it

The user watching arrivals in the 3v4 benchmark and finding them unrealistic.
