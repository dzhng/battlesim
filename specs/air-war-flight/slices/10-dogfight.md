# 10 — Dogfights

**Delivers:** D8′, D11, D29b.

## Seam
`crates/sim/src/air/dogfight.rs` is the **only writer of `dogfight_target`**:
- `on_fire(shooter, target)`: the shooter's link is set. The target adopts the shooter only if it has no link, and only jet-to-jet.
- `breaks(distance_3d, &JetAir)` uses `jet.dogfight_break_m`.
- `intent(own, target_track, seed) -> Intent::Pursue`, with lag pursuit, a merge break, and height wander inside `dogfight_band_agl_m`.
- **Delete** the mutual overwrite (`battle.rs` ~3228–3257) and both 1200 literals.

## You see
`/labs/air-dogfight`, restaged on `air-jet`: a 2-against-2 fight, a third jet joining, and a break at range. The target is the mock's dogfight scenario.

## Tests (red first; `crates/sim/tests/jet_dogfight.rs`)
- `firing_makes_it_your_target_and_it_adopts_you_only_if_free`
- `a_third_jet_joins_by_targeting_an_engaged_one`
- `the_link_breaks_past_the_rule_distance_in_3d`
- `a_jet_makes_passes_at_a_helicopter_and_neither_dogfights` (since D29c, which supersedes D29b: only two fixed-wing aircraft in one layer dogfight)
- `on_a_head_on_merge_both_break_away`
- `pursuit_aims_where_the_target_was`
- `dogfight_height_stays_in_its_band`

## Visual gate
- **Variable:** the dogfight's turning paths. **Crop:** the trace sheet.
- Compare against the mock with `compare-screenshots`, run `screenshot-critique` last, then `preview-shots`.

## Delegated
The lag gain, and what triggers a merge break.
