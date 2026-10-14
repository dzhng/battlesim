# Air-war flight: choices

Decisions made where the spec was silent, appended each pass (`audit-choices`). Decisions the user made live in [the map](map.md).

| Pass | Choice | Verdict | Why |
|---|---|---|---|
| write-spec | A ground-missile pass flies the strafe profile (nose-on, under `strafe_ceiling_m`). | Settled | The user described the missile pass as "the same type of strafe maneuver where its nose is pointed at the enemy". D13 left its ceiling unset. |
| write-spec | A shadow is thrown from the aircraft's actual height, along the sun. | Settled | A fixed offset would leave a diving jet's shadow floating. D2's one height source still covers the ghost and cruise height. |
| write-spec | Jets get an attitude slice (14) that pitches them along their path. | Settled | Today the attitude comes from acceleration, so dives draw level. |
| write-spec | A new jet test map, `air-jet` (about 3×3 km). | Settled | All four drafts found the 640×520 m air lab map is crossed in about 3.5 s at jet speed. |
| 00 | During the off-map delay the own unit is published with `traversing` and `ingress_remaining` (`units.rs:312-334`, `observation.rs:494-560`). The presentation draws only unit records, so an approach needs its own record. | Settled: the spec's `AirTransit` | Slice 12 replaces `traversing` with an own-side `AirTransit` record; slice 13 draws it. No phantom unit. |
| 00 | `interception` names only `Missile` (`contract/src/weapons.rs`). | Settled | Slice 04: an interceptor engages a round through the target row its `interception` names, never a row picked by height. |
| 00 | No saved encounter relies on the four-sides spawn: encounters place units directly (`contract/src/encounter.rs`). | Settled | Slice 12 deletes the spawn without any encounter work. |
| 00 | All 19 fixed-wing family scripts call `vehicle_export.run_disabled` (`blender/vehicle_export.py:242`). Blender wasn't run in the sandbox; it is installed at `/Applications/Blender.app`. | Settled | Slice 16 builds them. |
| 00 | The test map `air-jet` is 3×3 km with `render_margin_m` 2000. The lab camera frames any fixture's field from its map's `size_m`, and scenes read the field size the same way. | Settled | One owner for each field's size: the map metadata. |
