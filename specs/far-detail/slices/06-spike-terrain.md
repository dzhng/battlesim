# 06: Spike: cluster detail for terrain

An experiment. It ends in a go or no-go with evidence, and no production code.
It needs only [slice 00](00-census-and-ceiling.md)'s session and can run
beside the production slices.

## Question

Terrain is the one large single mesh, and it is drawn at full resolution
wherever it is in view. Would a cluster hierarchy, coarser far from the
camera, save GPU time worth owning?

Earlier evidence says the prize is small: terrain shadows cost about 0.4 ms at
the wide hold, and choosing terrain ranges by view saved 0.22 ms. Those are
reasons to test cheaply first, not proof.

## Ladder: stop at the first rung that answers

**1. Count.** No GPU. At the wide hold, record terrain triangles submitted to
the view and to each cascade, their share of all submitted triangles, and
triangles per covered pixel. Do it for the receipt's city map and for the most
hilly map the generator admits. Flat ground is already merged into large
triangles (`crates/sim/src/world/terrain.rs`). If terrain is a small share
and its triangles are a pixel or larger, the verdict is no-go. Slice 00
already counted the city: 10 triangles to the view and 2,871 to the cascades,
against 12.5 million and 54 million for props. Only the hilly map is left to
count.

**2. Ceiling.** In scratch, draw the terrain from a uniformly coarser copy of
itself (about a quarter, then a sixteenth, of its triangles; cracks ignored),
keeping its shading. Never measure by omitting terrain, which also removes
its shading and everything it hides. Measure by
[the protocol](../README.md#measuring). If the sixteenth does not clear the
noise, terrain geometry is not the cost: no-go.

**3. Prototype.** Build the hierarchy with the pinned reference builder
([three-nanite at `b730aac`](https://github.com/apekshik/three-nanite/tree/b730aac13ab0ea67b75214613eea56c14151f6e5):
128-triangle clusters, groups of about 16, halved, locked borders, error that
only grows) over the exported terrain, and choose clusters by projected error
per region.

## What a go must also answer

1. **One owner.** `TerrainPartition`
   (`crates/presentation/src/static_world/ground/partition.rs`) stays the only
   owner of terrain ranges. Clusters would replace its 256 m tiles, not sit
   beside them.
2. **Exact where it is law.** Picking, shadow fitting, grass and prop contact
   and water stay on the simulation's exact mesh. Only what is drawn is
   coarser. Measure how far a prop's base or a painted scar sits off the drawn
   ground; it must be under a pixel by construction.
3. **Flat shading and tint.** Terrain is a triangle list with a normal and a
   tint per face, and the traversal view tints blocked ground. Those edges
   must survive as seams; do not weld across them to make the builder
   progress.
4. **Shadows.** Each cascade keeps its own choice of terrain, so ground
   outside the view still casts. If keeping that erases the win, the verdict
   is no-go.
5. **No cracks** between regions at mixed detail, and the existing
   `crates/presentation/tests/terrain_partition.rs` passes unchanged.
6. **Cost.** The hierarchy is built at load in both clients. Measure build
   time and the extra memory of holding every level.

## What the user can see

`assets/terrain-verdict.md`: the counts, the ceiling table, the prototype's
comparison if built, answers to the six questions, and go or no-go. If go, a
sketch of a separate plan. Any picture offered as evidence gets
[`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
against the current game, then an unprimed
[`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md),
then [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md) without
blocking.

## Stop

Half a day for rungs 1 and 2. One working day for rung 3, one map region and
one adversarial fixture, two error thresholds at most.

## Delegated

- Which hilly map, how the coarser copy is made, and the scratch builder's
  language.

## Must stay green

Production is untouched. Delete the scratch checkout once the verdict is
written.

## Feedback that would change this slice

- "I care about big maps more than this one view": aim the ceiling at the
  `geometry-scaling` scene instead of the wide hold.
