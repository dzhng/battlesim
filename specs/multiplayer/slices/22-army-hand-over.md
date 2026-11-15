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
