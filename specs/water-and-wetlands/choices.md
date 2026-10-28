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
