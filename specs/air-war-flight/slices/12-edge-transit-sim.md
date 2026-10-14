# 12 — Edge transit in the sim

**Delivers:** D35 (the sim side), D28′, D31. **Jets only:** the user kept helicopters' current arrival (Q13), so the four-sides spawn, the off-map wait and `traversing` stay for helicopters.

## Seam
`crates/sim/src/air/edge.rs`:
```rust
BackEdge { a: V2, b: V2, inward: V2 }
back_edge(side, &sites, &World) -> BackEdge   // the map edge nearest the side's EntrySite.center; one lookup for every caller
entry(&BackEdge, toward: V2) -> (V2, heading)
nearest_exit(&BackEdge, pos) -> V2
crossed(&BackEdge, prev, next) -> bool
```
- **Arrival:** a bought jet stays a reservation through its seeded delay, then spawns at `entry` at its layer's cruise height and speed, heading in.
- **Published record:** own-side `AirTransit { kind, point: V3, heading, speed_mps, edge_tick, dir: In | Out }`, carried through publication, the reader and WASM.
- **Withdrawal:** heads to `nearest_exit`. The jet is retired and refunded at `crossed`, and an `Out` transit is published. The fuel estimate uses the edge distance.
- **Delete for jets** (helicopters keep them):
  - the jets' use of `traversing`, `ingress_remaining`, the four-sides spawn and the snap;
  - the entry-centre retirement;
  - the crash clamp (`crash.rs` ~111): a downed aircraft crossing the edge just ends.
- **Rounds leaving the map already expire.** Keep that.

## Tests (red first)
- `a_bought_jet_spawns_on_its_back_edge_at_cruise_heading_in_after_its_delay`
- `a_bought_helicopter_still_arrives_as_before`
- `inbound_transit_is_published_to_its_own_side_only`
- `nothing_senses_or_targets_an_aircraft_before_it_arrives`
- `withdrawal_retires_and_refunds_on_crossing_the_nearest_back_edge_point`
- `fuel_return_uses_the_edge_distance`
- `a_downed_jet_crossing_the_edge_just_ends`

## Firewall and delegation
- **Firewall:** no drawing; slice 13 draws.
- **Delegated:** the exact shape of the transit record, kept minimal.
