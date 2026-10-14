# Air-war flight: choices

Decisions made where the spec was silent, appended each pass (`audit-choices`). Decisions the user made live in [the map](map.md).

| Pass | Choice | Verdict | Why |
|---|---|---|---|
| write-spec | A ground-missile pass flies the strafe profile (nose-on, under `strafe_ceiling_m`). | Settled | The user described the missile pass as "the same type of strafe maneuver where its nose is pointed at the enemy". D13 left its ceiling unset. |
| write-spec | A shadow lies on the ground under the aircraft's actual position. | Settled | A fixed offset would leave a diving jet's shadow floating. D2's one height source still covers the ghost and cruise height. |
| write-spec | Jets get an attitude slice (14) that pitches them along their path. | Settled | Today the attitude comes from acceleration, so dives draw level. |
| write-spec | A new jet test map, `air-jet` (about 3×3 km). | Settled | All four drafts found the 640×520 m air lab map is crossed in about 3.5 s at jet speed. |
