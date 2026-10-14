# 03 — Steering: circle and edge

**Delivers:** D4/D10, D36.

**Unlocks:** goals are circled, and a jet never leaves the square while it is fighting.

## Seam
`crates/sim/src/air/steer.rs`:
```rust
enum Intent { Orbit { centre: V2 }, Line { through: V2, heading: f64, agl: f64 },
              Pursue { point: V3, vel: V3, lag_s: f64 }, Exit { to: V2 } }
steer(&Intent, &JetState, &JetLimits, &Bounds) -> Steer
```
- **Orbit:** turn at the full turn rate, so radius = speed ÷ turn rate, in the direction the jet is already turning (`turn_sign`). Close in on the centre first when the jet is far from the circle.
- **Edge guard:** applies to every intent except `Exit`. It turns the jet back before the edge, using `jet.edge_margin_m`.
- **Composition:** `air/mod.rs::step_jet` goes from order to intent to steer to `fly`.
  - A move goal gives `Orbit` around the goal.
  - An attack gives `Orbit` around the target's current position, until slice 08 adds passes.
  - No order gives `Orbit` around the jet's current position.
- **When a Move counts as done:** once the jet reaches its circle, so queued orders still advance.

## You see
`/labs/air-high?variant=circle`:
- a move order flies a straight leg into a circle of about 143 m;
- afterburner widens it to about 215 m;
- a goal near the edge gives a circle that turns back inside the square.

The target is the behaviour mock's "move goal → circle" scenario.

## Tests (red first)
- `a_move_goal_is_circled_at_speed_over_turn_rate`
- `a_jet_circles_the_way_it_was_already_turning`
- `an_attack_circles_the_targets_current_position`
- `a_circle_by_the_edge_turns_back_inside_the_square`
- `a_queued_order_follows_once_the_circle_is_reached`

## Visual gate
- **Variable:** the circle's shape and radius. **Crop:** the trace sheet.
- Compare against the mock with `compare-screenshots`, run `screenshot-critique` last, then `preview-shots`.

## Firewall and delegation
- **Firewall:** no passes, dogfights or weapons.
- **Delegated:** how closing in on the circle is computed.
