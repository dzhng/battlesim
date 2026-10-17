# 07 · Unloading

**Owns:** D8 (simulation), D9, D12 (carrier search), L12. **Depends on:** 06. **Outcome change:** none for battles without carriers.

## Contract
- `Order::Unload { units, goal, gesture, route }`:
  - `goal: Some(p)` lowers to `[Move(p), UnitOrder::Unload]`, and Shift appends it.
  - `goal: None` (U pressed twice) replaces the queue, stops the carrier, and unloads where it stands.
- **All or nothing.** The unload timer is the carrier's 1 s hold timer, run in reverse (`Exiting`). Passengers stay aboard, invisible, during the timer. Spots for every passenger of every squad aboard are found when the timer ends; if any soldier has no spot, nobody gets out.
- **Refusals** (`NoRoomToUnload`):
  - `goal: None` checks spots when the order arrives and refuses at once in `CommandAck.refused`.
  - At a goal, or as a queued step, the check happens when the timer ends. If there's no room, the step is dropped, the refusal goes in `ObservationFrame.refusals`, the carrier stays stopped with everyone aboard, and later queued steps stay.
- **Spots:** `carriage::unload_spots(world, units, carrier) -> Option<Vec<V3>>`, using the shared spot search from slice 04 with `rules.carriage.unload_search_m` (5 m). It excludes props, live hulls and soldiers, and takes height from `surface_at`. Over water or a roof it finds the bank or the building's foot, or nothing.
- **An unload never kills.** No death path exists here.

## What a human can see
Variant `unload` of the transport lab:
- a carrier unloads at a point;
- one unloads in place;
- a queued drive-unload-drive runs through;
- one is refused beside water.

## Verification
- `crates/sim/tests/transport.rs`:
  - `a_carrier_unloads_where_sent_and_queued_orders_follow`.
  - `an_unload_with_no_room_is_refused_and_everyone_stays_aboard`. It covers the in-place refusal at order time, the refusal on arrival with later queued steps kept, and no death in either.
- The transport scene, variant `unload`.
- Visual: the unload and refusal frames get an unprimed `screenshot-critique`, then the non-blocking `preview-shots` checkpoint.

## Delegated
Spot arrangement (behind the hull, or around it) and spacing.

## Must stay green
Garrison exits, movement placement, every digest of battles without carriers.

## Feedback that would change it
Where soldiers appear relative to the hull.
