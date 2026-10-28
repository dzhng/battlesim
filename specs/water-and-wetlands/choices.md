# Choices

Decisions the implementing agent made where the plan was silent or wrong
against the code. Each says what was chosen, why, and how sure the agent is.
[`decisions.md`](decisions.md) still holds the user's givens.

## Slice 10: swamp terrain

1. **A swamp's outline is a polygon.** Admission refuses a stroke. *Why:* a
   swamp is an area, and a polygon gives an exact distance in from its edge,
   which the cut, the membership and presentation all read. A stroke's
   square-cut ends would need their own distance rule. *Confidence:* high.
2. **The water stands at one level per swamp, and the land under it must be
   level.** The level is the land's height at the outline's first point.
   Admission refuses a swamp where any height sample it cuts, or a triangle
   it moves, stands at another height. *Why:* decision 1 puts the water flat
   at the old ground level, which only exists when the land is level; one
   level makes the water a plane, as a river's is, for `water_hit` and for
   drawing. Generated maps have no relief, so mapgen loses nothing.
   *Confidence:* high.
3. **The edge eases in over three height samples (12 m on a 4 m grid) by a
   smoothstep.** Admission refuses a depth whose steepest grade (1.5 × depth
   over the edge's width) exceeds `steepest_grade`. *Why:* decision 21; the
   smoothstep has no kink at either end, so no triangle is steeper than the
   curve's middle. *Confidence:* high for the shape, medium for the width,
   which slice 13 may want wider on screen.
4. **A road ends the cut a triangle clear of its side, and eases in from
   there as from the outline.** The cut is the smoothstep of the lesser of
   the distance in from the outline and the distance from the nearest road
   less one cell diagonal. *Why:* a cut sample lowers every triangle it is a
   corner of, so a cut that started at the road's edge would tilt the road's
   own sides; easing in keeps the causeway's sides as gentle as the outline's.
   Road distance is the exact distance to a stroke's stretches, square ends
   included (`contract::ground::stretch_distance`). *Confidence:* high.
5. **A road across a swamp is dry ground.** `Surface.swamp` and
   `water_hit` exclude road surfaces: the causeway takes marks and rounds
   land on it. *Why:* decision 4 makes the road a causeway; standing water
   would otherwise cover it at its own height. *Confidence:* high.
6. **Minimum size: some height sample lies (1.5 − √½) cells inside the
   outline.** *Why:* any swamp holding a disc three cells across has such a
   sample wherever the grid falls on it (decision 22), and a sliver much
   narrower does not. *Confidence:* medium; the threshold is a loose
   tripwire, not a target.
7. **River gap: no height sample within an edge's width and one more sample
   of the outline is river water or river-cut bank.** *Why:* a swamp's rim
   then stands on the land's own height on every side, and the carve's two
   owners never meet. *Confidence:* medium on the exact width.
8. **Buildings and bridge approaches: no height sample the swamp cuts lies
   within a cell diagonal of a building part's footprint, or of a bridge deck
   lengthened by an edge's width at each end.** *Why:* no triangle under them
   is then cut, so their absolute `base_z` and the deck's ramp stay true.
   *Confidence:* high.
9. **Overlapping swamps are allowed;** the deeper cut wins at each point.
   *Why:* level land makes their water the same height, so nothing needs
   refusing. *Confidence:* medium.
10. **The export is one stride per outline edge:** `ax, ay, bx, by, swamp,
    surfaceZ, depth` (`SWAMP_STRIDE`, `swampFields` in the layout). *Why:*
    presentation rebuilds each outline and its membership from it, and
    `swamp` is the index slice 03's variant pick seeds from. *Confidence:*
    high.
11. **`NavGrid::swamp_at` is a public query on what the planner reads.**
    Today only the seam test calls it; slice 11's cost reads `Cell.swamp`
    directly. *Why:* the slice's seam test compares the planner's cell with
    movement's surface, and navigation keeps its cells private. Delete it in
    slice 11 if the planning tests there make it redundant. *Confidence:*
    medium.
12. **Decision 21's sight test is run on the ground sight sweep, not on
    identification.** With water at the land's level stopping sight
    (decision 32), a line between two points above the water never passes
    under the rim, so the dip cannot hide a target that stands above its
    water, and one whose points lie under the water is hidden from everyone.
    The terrain-only sweep (the fog) does show the mechanic: on a 3 m test
    dip, a far observer on flat ground loses the dip's floor and a near or
    high one sees it. *Why:* decisions 21 and 32 cannot both hold for unit
    identification; 32 is the slice contract's one water rule. *Confidence:*
    high on the geometry. Whether a deep swamp should hide units from far
    observers at all is the user's call.
13. **The rays (`raycast`, `sight_clear`, `water_hit`) moved to
    `world/rays.rs`, and mapgen's plan types to `mapgen/src/plan.rs`,** each
    in its own commit with no behaviour change. *Why:* both files were over
    the size budget and slice 10 had to touch them. *Confidence:* high.

## Slice 11: swamp slowdown

1. **First shipped factors: foot 0.5, wheeled 0.3, tracked 0.55**
   (`movement.swamp` in `fixtures/game.json`). *Why:* wading thigh-deep
   halves a soldier's pace, trucks bog worst, tanks churn through a little
   faster than a soldier wades. They are tuning, not requirements; the
   paired battle report is still owed (see below). *Confidence:* low, by
   design.
2. **`Footing { road_factor, forest, swamp, slope_deg }` lives beside
   `Mobility` and is built by `Surface::footing()`, `Cell::footing()` and a
   road `Arc::footing()`.** *Why:* each owner of a piece of ground says what
   a mover's speed reads of it; the five read sites shrank rather than grew,
   which the size budget on `soldier.rs`, `journey.rs` and `navigation.rs`
   needed. *Confidence:* high.
3. **Off a road the speed is open speed × min(forest factor, swamp
   factor),** each 1 where its ground is absent. *Why:* decision 4's "the
   slower applies"; for ground with neither it is bit-identical to the old
   arithmetic, so only the rules hash moves digests. *Confidence:* high.
4. **`SlowGround` keeps three counts per tile off any road: forest only,
   swamp only, both,** and the estimate weighs "both" by the slower factor.
   The expression keeps the old one's order, so a map with no swamp
   estimates bit for bit as before. *Confidence:* high.
5. **`NavGrid::swamp_at` is deleted.** The planning seam test now compares
   each cell's planned cost with movement's speed at its middle
   (`ground_speed::planning_costs_each_cell_at_the_speed_movement_takes_there`),
   which is the claim the query stood in for. *Confidence:* high.
6. **`MovementRules` and `SwampRules` refuse unknown keys; `DriveRules`
   does not yet.** *Why:* the slice named `MovementRules`; the drive row is
   unchanged here. *Confidence:* medium.
7. **The dense reference planner in `examples/navigation_quality` reads
   `sim::navigation::Mobility` and its `speed`;** its road cells are road
   factor 1 and never swamp. *Why:* the slice deletes its private copy.
   *Confidence:* high.
8. **Ground-speed tests live in `tests/ground_speed.rs`,** with the old
   `speed_follows_surface_and_slope` folded in as a slope test. *Why:*
   `tests/navigation.rs` was over the size budget, and speed over ground is
   one concept. *Confidence:* high.
9. **No paired battle report on the `wetlands` map yet.** That map comes
   from slice 00 in the other lane; run the report once it is merged.

## Slice 12: swamps in map generation

1. **Swamps come in pairs: one in each half, of one drawn size.** *Why:*
   the halves then hold the same wet ground by construction; `verify`
   still measures the split against `swamps.fairness` and refuses a plan
   outside it, as it does woods. *Confidence:* high.
2. **Each map type's chance is `swamps.chance.<type>` in the presets' own
   swamp section, not a `swamp_chance` field beside `river_chance`.**
   *Why:* `layout/presets.rs`, which holds the type rows, is over the size
   budget and may not grow; the swamp section owns every swamp number.
   Shipped chances: open 0.45, mixed 0.35, metro 0.2. *Confidence:* high
   on the place, low on the values (tuning).
3. **Placement:** after towns, woods and the skirmish reservation, before
   `verify`, from the `swamps` stream. A swamp lies wholly inside its half
   and the map, keeps `settlement_gap_m` from every settlement outline (both
   rings' vertices) and every skirmish objective (its whole radius) and
   entry, keeps `road_gap_m` from every road's edge and `wood_gap_m` from
   every wood, keeps off every reserved corridor, keeps `bridge_gap_m` from
   a deck lengthened by its approach (a circle test), keeps `swamp_gap_m`
   from other swamps, and with a river lies between `river_gap_m` and
   `river_reach_m` of the water with its long axis along the river's course
   there. *Why:* the first unprimed critique of a six-seed sheet found roads
   running through swamps, swamps notched into woods, one crowding a
   village, and round ponds unrelated to the river; generated swamps now
   keep off all of those (hand-made maps may still lay a causeway across
   one). The second critique found no placement error. *Confidence:* medium
   on the gaps and sizes (tuning).
4. **Later passes keep off swamps through what they already keep off:** the
   open country counts a swamp with the woods it never furnishes
   (`open_country/country.rs`), and the ground-sight coverage counts its
   depth in the lowest ground. Parcels and street furniture stand in
   settlements, which swamps keep clear of. *Why:* no new owner, and
   `open_country/mod.rs` is over the size budget. *Confidence:* medium:
   street furniture along a rural road across a swamp could stand in its
   water; none of the corpus seeds did.
5. **The compiler runs `contract::swamp::validate` on the lowered map** and
   refuses with `InvalidSwamp`, so a placement rule that misses a building
   or a bridge approach is a named seed refusal, not a world assertion.
6. **Preset validation refuses a swamp the terrain cannot carve:** a depth
   past `contract::swamp::deepest_m` (now the contract's one rule), a
   narrowest drawn swamp under four height samples across, and a river gap
   inside a river's cut bank plus the swamp's edge. *Confidence:* high.
7. **Versions:** generator `layout-20`, presets `layout-presets-20`. Plans
   and maps without swamps serialize as before (`swamps` is skipped when
   empty). *Confidence:* high.
8. **The deck clearance is belt and braces:** with the shipped river gap no
   swamp reaches a bridge approach, so its test is the compiled map's
   admission, not the placement rule alone.
9. **Splits for the size ratchet:** the presets document
   (`layout/presets/definitions.rs`) and the open country's clearance
   reading (`open_country/country.rs`) moved out in their own commits.
10. **Left for tuning and slice 13:** the second critique still read the
    swamps as neat ponds of one family, small at 6 km, and noted the
    riverside ones never meet the river. The gap is the contract's (a
    swamp may not stand on a river's cut bank); size and variety are preset
    numbers; the crisp outline is the plan picture's symbol, and the look on
    the battlefield is slice 13's.
