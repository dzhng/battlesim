# 04 — Distances in 3D, layers from the catalog

**Delivers:** D32′.

**Unlocks:** no unit freezes between a 2D "in reach" and a 3D fire gate, and height never decides a layer.

## Seam
- **One helper,** `body_distance(a: V3, b: V3)`, in sim math. It replaces the 2D checks:
  - reach (`weapons.rs` ~1101);
  - ranking and area distance (~1218, ~1242);
  - dogfight start and break (`battle.rs` ~3231, ~663);
  - radar detection (`knowledge.rs` ~326).
- **Delete the height thresholds** (`weapons.rs` ~528, `battle.rs` ~756).
- **Add a round target:** `Target::Round(ProjectileId)` (or the equivalent slice 00 found). Interceptors lock incoming interceptable rounds through their `interception` property and use the row slice 00 settled on.
- **Re-record the pinned ground digests** as a named decision in `choices.md`.

## You see
`/labs/air-defense`: AA engages a jet at 250 m exactly within its 3D reach.

## Tests (red first)
- `reach_and_fire_gate_agree_so_nothing_freezes` (red: 2D reach with a 3D gate)
- `a_gun_holds_fire_at_a_jet_out_of_3d_reach_overhead`
- `an_interceptor_engages_an_incoming_missile_at_any_height`
- `radar_detection_range_is_3d`
- `a_ground_order_on_a_hill_stays_ground`

## Firewall and what stays green
- **Firewall:** no change to weapon rows or flight.
- **Stays green:** guidance, protection, radar and AA layering tests (re-recorded digests only).
