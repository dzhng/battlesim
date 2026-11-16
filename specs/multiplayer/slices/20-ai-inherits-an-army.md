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

## Results (2026-11-15)

The tests are `crates/sim/tests/skirmish_ai_inherits_army.rs`. No simulation
code changed. This is the slice's second outcome: **it copes at 30 and not
at 150.**

**At 30 units, it copes.** A human buys 30 tanks, sends them into a fight
with an active Red AI of the same size, and 90 seconds in the seat is handed
to its AI. Beside it runs a control: the same battle and seed with nobody
commanding the army after that moment. A minute later:

| Seed | Blue alive, handed to AI | Blue alive, nobody commanding | Red alive, AI | Red alive, nobody |
|---|---|---|---|---|
| 31 | 23 | 18 | 23 | 29 |
| 32 | 20 | 17 | 30 | 30 |
| 33 | 15 | 17 | 26 | 30 |
| 34 | 18 | 18 | 29 | 30 |
| 35 | 20 | 17 | 30 | 30 |

- Every unit the AI inherited had an order applied within one planner round
  (five seconds), and none of its orders was refused.
- The objective Blue held at the handover was still Blue's in every run.
  That says less than it sounds: it was still Blue's in every control run
  too, and the scores were identical. An army left alone on an objective
  holds it for a minute. The difference the AI made is in the fight: it kept
  more of its army in four seeds of five and killed more of Red's.
- The AI's 29 orders in its first round fit the order limit of 30 a second
  exactly. A seat's cap of 30 units and the limit are the same number, so a
  seat at its cap is at the edge.

**An army that shrank under a human does make the AI change doctrine, and it
kept its objective anyway.** The AI held the seat with 28 units, a human
took it for a minute while the fight cut the army to 12, and the AI got it
back. The seat's AI does not watch while a human holds the seat, so it saw
the whole loss at once and changed doctrine once, as the walk feared. It did
not give up the objective it held.

**At 150 units, the AI's orders run into the order limit.** On a field with
no enemy, a seat holding 150 units was handed to its AI:

| Planner round | Orders applied | Orders refused for rate |
|---|---|---|
| 1 | 30 | 117 |
| 2 | 30 | 87 |
| 3 | 30 | 57 |
| 4 | 30 | 27 |
| 5 | 28 | 0 |

- The AI gives all its orders in one burst at the start of each planner
  round. The limit holds 30 orders and refills at 30 a second, so one burst
  gets 30 through and the rest are refused. The limit would allow 150 in the
  five seconds between rounds; the burst uses a fifth of that.
- No unit is starved for good: the refused units are ordered in later
  rounds, and all 150 had an order after five rounds. But 117 of them waited
  longer than one round, and the last waited 25 seconds. Any change of plan
  takes that long to reach the whole army.
- This was measured with the seat's cap raised to 150, because an army over
  a seat's cap of 30 cannot be built until slice 22 exists. The AI's buying
  code only compares its unit count to the cap, so at the cap and over it
  the AI behaves the same: it stops buying.
- A 150-unit army in a fight was not measured. With an active Red AI the
  entry roads jammed before 150 units were on the field.

**What this changes.** Rooms are safe: a dropped player's seat holds at most
30 units. For Frontline it is the choice the slice names, and it is the
user's, before slice 22:

- **Make the AI spread its orders** across the planner round instead of
  bursting them. This is the smaller change and removes the cause: 150 units
  would all be ordered within one round.
- **Cap what one seat may receive** in a hand-over at what the limit lets
  through, about 30 units.

Decision 29 already sends a displaced army to the AI seat with the fewest
units, so the bad case needs four humans on one side. The recommendation is
the first option.

## Follow-up (2026-11-15): pacing the AI's orders was tried and set aside

With the user's choice still open, a pass was sent to build the first
option (the AI spreads its orders) so it could be judged. It stopped before
changing anything, on a real conflict: an existing test,
`a_rate_limited_ai_order_is_receipted_and_the_battle_goes_on`
(`crates/sim/tests/skirmish_ai_orders.rs`), pins the opposite as a
principle. Its comment reads: "The world may refuse an AI's well-formed
order: its order bucket is out of a view the AI does not have. The refusal
is receipted, and the battle goes on." Pacing, wherever it lives, means an
AI's order is never refused for rate, so that principle and its test would
have to go. That is a change to an existing rule of the game, not a
contained fix, and it is not the build's to make unasked. Nothing was
changed.

Looked at again, the cost of leaving it is smaller than the 150-unit table
suggests. Frontline's real case is a seat holding its own army and one it
was handed: at most about 60 units. The limit lets 30 orders through a
planner round, so a 60-unit army is fully ordered in two rounds, about ten
seconds, and no unit is left out. Only a seat that receives several armies
in a row gets near the 150-unit case, and decision 29 sends each displaced
army to the AI seat with the fewest units, which spreads them.

**The choice, for the user, with a changed recommendation:**

- **Leave it for the alpha** (now recommended). A doubled army takes about
  ten seconds to be fully commanded after a hand-over or a change of plan.
  The ignored 150-unit test stays as the record.
- **Make the AI pace its orders.** Removes the delay, and retires the
  principle that the world may refuse an AI's order for rate, with its
  test.
- **Cap what one seat may receive** at about 30 units, sending the rest to
  another AI seat.

**Decided by the user on 2026-11-15: leave it for the alpha.** The AI's
order pace is not changed. The ignored 150-unit test stays as the record.

