# 23 Seat reset

**Stage:** Frontline. **Needs:** 22. **Carries:** decision 6, the once-only
opening credits, P17; landmine 8.

Use [tweak-mechanics](../../../.agents/skills/tweak-mechanics/SKILL.md)
before proposing anything here.

## Contract

A recorded event gives a seat a fresh start: no units, no purchases on their
way, and a wallet holding the opening credits, or nothing. The event says
which. It applies only to a seat that holds nothing, which is what a
hand-over leaves.

The server knows which players have had their opening credits in a game; the
simulation does not know who a human is. So the server decides, and the
event records the decision (P17).

## Seam

`crates/sim/src/battle/seats.rs` from slice 22:

```rust
pub enum SeatEvent {
    HandOver { tick: Tick, player: String },
    Reset { tick: Tick, player: String, opening: bool },
}
impl Battle { pub fn reset_seat(&mut self, player: &str, opening: bool) -> Result<(), SeatError>; }
```

## Run or see

The native example from slice 22, extended: hand over, reset, then the seat
buys its first unit and it drives in by that side's road.

## Verification

- A reset seat holds no units and exactly the opening credits when `opening`
  is true, and no credits when it is false.
- A reset seat earns income from then on like any seat.
- Resetting a seat that still holds units is refused.
- The seat's order sequence continues; a human's first order after a reset is
  applied (slice 03's stamp).
- The live digest and the replay's digest agree, with both events.
- A battle with no seat event has the digest it had before slice 22.

## Must stay green

Every existing digest and replay. No room path calls this.

## Delegated

What the AI of a reset seat does when the seat is later handed back to it
with few units: the walk found an AI with no units rebuilds at one purchase
every five seconds. That is accepted for the alpha.

## Feedback that would change this

P17 is the user's to overturn: a player who rejoins a game they left starts
with no credits. The alternative is letting them take their old seat back as
it stands if its AI still holds it.

## As built (2026-11-15)

`Battle::reset_seat(player, opening)` records `SeatEvent::Reset` for the
next tick. It is refused for a seat that still holds a living unit or a
purchase on its way. It sets the seat's wallet to the opening credits (the
same sum a battle starts every seat with) or to nothing, as the event
says, and income resumes on the next tick. The seat's order sequence, its
purchase counter, its controller and its history carry on.

The example now hands a seat over, resets it, and has it buy its first
unit, which drives in by that side's road; it matches its replay at every
tick with both events. A replay whose reset is moved a tick, named for
another player, or flipped to no opening credits does not reproduce the
battle.
