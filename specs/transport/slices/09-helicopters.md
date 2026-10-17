# 09 · Helicopters

**Owns:** D6 (low hover), D11 (the air row), D13. **Depends on:** 01 (or its recorded skip), 08.

Load `tweak-mechanics` first.

## Contract
- **One low-hover predicate.** `air::sinks` (`crates/sim/src/movement/air.rs:155-163`) today needs an empty order queue, no engagement, and a serving supply truck. It becomes one predicate with two reasons:
  - resupply (unchanged);
  - a carrier hold timer in progress, for boarding or unloading, regardless of engagement.

  The helicopter sinks to `low_hover` (`:165-178`), and the hold timer runs once it has settled (`units::Settle` ended). Any other order lifts it, as today.
- **Boarding and unloading at the hover:** squads board from the ground beneath it. Unload spots are searched outward from the helicopter's x/y, with z from `surface_at` and live hulls under the hover excluded (L12).
- **A helicopter shot down with passengers** (D13):
  - Passengers stay aboard while it falls, and their positions follow `crash.position`, not the frozen `unit.position` (`battle.rs:2869-2879`).
  - At `land()` (`battle.rs:2689`), call `bail_out` with the `air` row **after** the crash burst, so the burst can't hit them twice.
  - The air row first rolls `squad_loss` (0.5) once per squad.
  - A crash that returns `LeftMap` (`crash.rs:126`) kills every passenger (`Died { cause: LeftMap }`), reported with the helicopter's killer.
- **New test unit:** `test_transport_heli` (16 seats) in `fixtures/units/test/aircraft.json`. `test_heli` keeps no seats; `panelSpecimens.ts:60` pins it.

## What a human can see
Variant `heli` of the transport lab: a helicopter sinks, a squad boards, it flies, sinks again and unloads. A second one is shot down, and its squad bails out around the wreck, or doesn't.

## Verification
- `crates/sim/tests/transport.rs`:
  - `a_helicopter_boards_and_unloads_from_its_low_hover`.
  - `a_helicopter_shot_down_keeps_its_passengers_until_it_hits_the_ground`. It searches seeds for both outcomes of the squad-loss roll, and includes the off-map case.
- The existing resupply-hover tests are unchanged.
- The transport scene, variant `heli`.
- Visual: unprimed `screenshot-critique` of the hover unload and the crash bail-out, then the non-blocking `preview-shots` checkpoint.

## Delegated
Whether the settle must end before the timer starts, or the timer may start when the hover height is reached (default: the settle ends).

## Must stay green
Resupply hover, crash and wreck behaviour without passengers, the `air-*` scenes.

## Feedback that would change it
The air row's numbers; whether the 50% squad loss reads too harsh in play.
