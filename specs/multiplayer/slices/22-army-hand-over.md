# 22 Army hand-over

**Stage:** Frontline. **Needs:** 20. **Carries:** decisions 29, 31, 34, P16;
landmine 8.

Use [tweak-mechanics](../../../.agents/skills/tweak-mechanics/SKILL.md)
before proposing anything here.

## Contract

A recorded event moves everything a seat holds to another seat on its side:
its units, its credits, and its purchases still on their way. The receiver is
the seat on that side an AI holds with the fewest units, ties broken by seat
order (P16). The simulation picks it; the event names only the seat being
vacated.

Nothing moves a unit, a wallet or a pending purchase between seats today, and
a unit never changes owner.

## Seam

New `crates/sim/src/battle/seats.rs`, modelled on the controller change
(`crates/sim/src/battle/controllers.rs:50`):

```rust
pub enum SeatEvent {
    HandOver { tick: Tick, player: String },
    // slice 23 adds Reset
}
impl Battle { pub fn hand_over(&mut self, player: &str) -> Result<(), SeatError>; }
// Replay gains seat_events, absent from the record when empty.
```

It applies at a tick boundary. A battle that records no seat event keeps its
digest and its replay byte for byte, so rooms and skirmish are untouched.
(The replay's `engine_build` stamp is the one exception: it is a fingerprint
of the simulation's source and changes with every source change, as it has
for every slice. A first pass at this slice stopped on that; it is not what
"byte for byte" is about.)

## Run or see

A native example that plays a 5v5, hands one seat's army over mid-fight, and
prints the digest live and from the replay.

## Verification

Rust tests of the moments the rule creates and rules out:

- After a hand-over, the old seat cannot order those units, and the receiver
  can.
- A refund for a handed-over unit pays the receiver. Today a refund pays the
  original buyer (`crates/sim/src/battle.rs:595`).
- A purchase on its way arrives as the receiver's. Purchase numbers are per
  player today (`crates/sim/src/battle.rs:1306`); two seats' numbers must not
  clash after a hand-over.
- A receiver over its unit cap keeps every unit and cannot buy until losses
  bring it under.
- The side's total credits are the same before and after.
- With no AI-held seat to receive, the hand-over is refused.
- The live digest and the replay's digest agree.
- A battle with no seat event has the digest it had before this slice.

Add the hand-over to the deliberate rules listed in
[development.md](../../../development.md#game-rules) if tweak-mechanics
judges it one.

## Must stay green

Every existing digest and replay. No room path calls this.

## Delegated

How ownership is represented, after reading the battle's data layout; the
order of seat events within a tick, fixed once and documented.

## As built (2026-11-15)

`crates/sim/src/battle/seats.rs` holds both seat events.
`Battle::hand_over(player)` records `SeatEvent::HandOver` for the next tick.

- **The receiver** is another seat on the same side that an AI holds
  (active or passive), with the fewest living units, ties broken by the
  order of players. It is chosen again when the event applies, from the
  battle's state, so a replay chooses the same one.
- **What moves:** every living unit (keeping its place, orders, routes and
  condition), the whole wallet, and every purchase on its way (keeping its
  price, kind, destination, road and timing).
- **What does not move:** history. A unit's spawn and its price stay with
  the seat that bought it; a kill or a loss counts for whoever held the
  unit when it happened; dead units keep the owner they died under.
- **A refund** for a handed-over unit pays the receiver, at the price and
  age the unit always had.
- **Purchase numbers:** handed-over purchases are given fresh numbers from
  the receiver's own counter, so none clashes with one the receiver made.
- **Over the cap:** the receiver keeps every unit and cannot buy until
  losses bring it under.
- **With no AI seat to receive,** the hand-over is refused and nothing
  changes.

Order within a tick, written where the events apply
(`apply_seat_events`): controller changes at the boundary before; then
orders are admitted and executed and that tick's income is paid; then seat
events in the order they were recorded; then arrivals and the tick. An
event that is no longer valid when it applies (or one a tampered replay
supplies) does nothing, live and replayed alike.

A replay carries `seat_events`, absent when there are none. Three existing
battles recorded before the change (seeds 1, 7 and 19, 600 ticks each) give
the same 1,803 digests, the same scenario and configuration digests, and
the same replay apart from the engine stamp; no fixture changed.

Both events are listed among the deliberate rules in `development.md`:
choosing an AI seat and moving ownership and money follow from no physical
property.

The native example `crates/sim/examples/seat_events.rs` plays a 5v5, hands
a seat's six units and its credits over mid-fight, and matches its replay
at every one of 1,800 ticks.
