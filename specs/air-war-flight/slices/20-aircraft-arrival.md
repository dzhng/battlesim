# 20 — Aircraft arrival, per-unit height, `test_jet` model

**Delivers:** D38–D43 ([flight-model map](../flight-model-map.md)). Starts after `awf/flight-model` merges with D29c.

## Contract
- **Catalog:** `mobility.air.arrival.delay_s: [min, max]` (optional) and `mobility.air.cruise_agl_m` (optional override), on any aircraft. The jet profile sets `arrival.delay_s: [15, 30]`; helicopter profiles set nothing.
- **Sim:**
  - One cruise-height lookup per unit type: the override, else the layer's height. High air's floor stays hard.
  - A purchased aircraft with a wait leaves the purchase queue and waits on its own seeded clock; the queue keeps spawning behind it.
  - Every aircraft spawns at the back-edge point nearest its side's entry site.
  - The hard-coded 15–30 s in `dispatch_reinforcements` is deleted.
- **Publication:** the pending-purchase record carries the arrival tick, 0 when there is none. `AirTransit` uses the same tick.
- **UI:** the pending card reads "Arriving in N s" while an aircraft waits.

## Tests (red first)
- [x] A jet and a tank bought together: the tank spawns without waiting for the jet (`skirmish::a_waiting_aircraft_leaves_the_queue_and_waits_on_its_own_clock`).
- [x] Two jets bought together wait in parallel (the same test; it also pins the pending record's and the approach's arrival tick, through the packed publication).
- [x] Any aircraft spawns at the back-edge point nearest its entry site, whatever its destination (`skirmish::any_aircraft_enters_flying_at_the_edge_point_nearest_its_entry_site`).
- [x] A row without `arrival` appears at the edge at once, in flight (the same test).
- [x] A cruise override moves the flight height (`air::a_cruise_override_is_the_flight_height_floored_in_high_air_and_uncapped_in_low_air`), the approach height (`AirTransit` is placed at the unit type's cruise height, by the same lookup) and the order ghost (`web/tests/unitGhosts.test.ts`). Below the high-air floor it flies at the floor. A low-air unit set above high air still flies.
- [x] D44: `attack_passes::every_loadout_flies_a_pass` buys every fixed-wing row in the game catalog, which arrives within its wait at its back edge, turns home on low fuel, leaves by its edge and is refunded.
- [x] The pending card counts down (`web/tests/armyDeck.test.tsx`; scene `air-ingress`).

## You see
- The pending card counting down.
- The approach drawn at the entry side of the back edge.
- `test_jet` in its own grey model on its sheet.

## Delegated
- Countdown wording and placement, within the existing card style (load `game-ui`).
- The exact shape of `test_jet`, within D43. Run `screenshot-critique` on its sheet.

## Stays green
Digests for every battle without purchased aircraft. Scenes `air-ingress`, `air-hud` and `supply`.
