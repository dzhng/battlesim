# 13 — Approach and departure effect

**Delivers:** D35 and D28′ (the visible half).

Load `game-ui` and `renderer` first.

## Seam
A pure presentation function, `crates/presentation/src/effects/transit.rs::tracks(&[AirTransit], now, &TransitFeel) -> Vec<TransitPose>`. Each pose is drawn as an ordinary aircraft record.
- **Inbound:** about 10 s of approach, ending exactly at the spawn point and tick.
- **Outbound:** continues the exit for about 10 s.
- **Visibility:** own side only, never fogged, never selectable.

## You see
`/labs/air-ingress` on `air-jet`: a bought jet streaks in over the margin and backdrop, and a fuel-low jet leaves by its own back edge.

## Tests
- `an_inbound_track_meets_the_spawn_point_at_the_spawn_tick`
- `the_enemy_frame_has_no_transit`
- `an_outbound_track_ends_after_its_duration`
- A native parity frame.

## Visual gate
- **Variable:** the jet over the margin. **Crop:** the map edge plus the backdrop.
- Compare against the mock's ingress scenario, run `screenshot-critique` last, then `preview-shots`.
