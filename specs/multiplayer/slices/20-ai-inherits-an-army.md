# 20 An AI inherits an army

**Stage:** Frontline. **Needs:** nothing; it can run while Rooms is built.
**Burns:** O5.

## Contract

Know whether a seat's AI copes with an army it did not buy, at the size it
is tested at and at the size Frontline can produce, before the hand-over is
built on that assumption.

What the walk found: every seat has an AI waiting from the first tick; the
per-side planner re-reads who holds each seat within five seconds
(`crates/sim/src/skirmish_ai/team/mod.rs:200`); an AI that takes a seat
adopts the army it finds. What nobody has checked: how well it handles units
it didn't buy. An AI returning to a seat reads a changed army as losses and
may fall back to an emergency defence. An AI holding far more than 30 units
stops buying and runs into the limit of 30 orders a second per player.

## Seam

A test in `crates/sim`, using the existing controller change. No new
mechanic: the army is put under the AI by handing it a human-held seat that
already has one. For the 150-unit case, build the army through a test fixture.

## Run or see

A short report: at 30 units and at 150, does the AI keep ordering every unit
it holds, and does it keep the objectives it held?

## Verification

Tests of what the AI does, not of how it is built:

- Handed a fighting army of 30, the AI's side still holds the objectives it
  held, some minutes later.
- The same at 150.
- No unit the AI holds goes unordered for longer than the planner's refresh.
- The AI does not abandon an objective for an emergency defence because its
  army changed under it.

## What the result changes

- **It copes at both sizes:** slice 22 proceeds as planned.
- **It copes at 30 and not at 150:** tell the user. Decision 29 sends a
  displaced army to the AI seat with the fewest units, which keeps armies
  small until every other seat is human; the failing case is four humans on
  one side. The user chooses between fixing the AI at that size and capping
  what one seat may receive.
- **It fails at 30:** fix the AI's takeover here. Rooms depend on it too: a
  dropped player's seat is run by its AI.

## Delegated

How the 150-unit army is assembled for the test.
