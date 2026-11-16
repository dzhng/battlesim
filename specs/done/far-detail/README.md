# Far detail

**Status:** closed. Parked cars draw a coarser far mesh when they are a few
pixels long. Two wider ideas were tested and rejected. **Closed:** 2026-11-16.

## What it is for

In the wide view the game draws about 120,000 props, most of them a pixel or
two across, each at the same mesh it uses at twenty pixels. Parked cars and
courtyard walls are 81% of the triangles those props submit there, and their
shadow casts cost about five times one view pass ([census](assets/census.md)).

The idea came from Unreal's Nanite and its three.js port,
[three-nanite](https://github.com/apekshik/three-nanite/tree/b730aac13ab0ea67b75214613eea56c14151f6e5):
choose how much geometry to draw from its error on screen. Only that idea
transfers. Our props are already smaller than one of its 128-triangle
clusters, and its answer to many triangles inside a pixel is a rasteriser
WebGPU cannot host.

## What shipped

A static prop's bundle may carry a fifth, coarsest mesh, its **far mesh**. A
prop that has one draws it below `presentation.models.far_px` (a model's
largest extent, in projected pixels) and casts its shadow with it while it
does. Parked cars carry one: their coarsest tier without its four tyres, 92
triangles against 172.

Measured on the [control scene](assets/control.json), three interleaved pairs
against the build before it:

| View | Change in GPU time |
| --- | --- |
| Close, 65 m | none |
| Tactical, 260 m | none; the frame is pixel-identical |
| Wide, 2,400 m | 34.59 to 31.94 ms |
| Whole map, 19,500 m | 1.95 ms lower |

Nothing else changes: the simulation, replays and what a side is told are
untouched.

## Why it is this small

The [first probe](assets/ceiling.md) gave cars and walls crude hulls and cut
the wide view from 34.6 to 23.0 ms. That saving is real, and almost none of it
survives one constraint: **the player must not see the switch**. Three rules
came out of holding to that, and they bind any later far mesh.

1. **A far mesh may not reshape a surface.** The side's fog of war judges
   every pixel by its own surface's position and facing
   (`fogSeenBy` in `crates/renderer-core/src/shaders/modules/fog.wgsl`). A car
   hull with a single roof face passed or failed that test differently from
   the real car: light cars read about 20 levels darker, and one car park took
   the pale unseen look together. With fog off the builds matched. So every
   surface a far mesh draws is one tier 3 draws.
2. **A far mesh may not drop a part a player can see.** A wall without one
   pier lost a post at every run's end and on every lone module.
3. **It may not even drop a face that ties the nearest depth.** The opaque
   colour pass lets an equal depth paint, so a wall's underside lying in the
   terrain's plane still changes pixels when it goes.

What is left to drop is a car's tyres. On a wall it is 10 of 88 triangles,
which is not worth owning, so walls carry no far mesh.

## Invariants

- A prop without a far mesh draws tier 3 at any small size. Nothing asks for a
  mesh its bundle lacks.
- A far mesh has at least one triangle and fewer than tier 3, every state of
  an appearance carries one or none does, and only a prop kind may. Units,
  kits, trees, dressing and grass keep exactly four tiers.
- Which props cast a shadow, and into which cascades, does not depend on the
  far mesh. A prop outside the view casts with tier 3.
- Whether an appearance has a far mesh is read from its bundle's bytes. No
  list of names decides it, and no renderer code picks a tier.
- `far_px` is required and sits above zero and below the last `lod_px` entry.
  There is no off value and no setting.
- The far mesh's index is the same number in both languages. One committed
  bundle holds them together: the TypeScript bake must produce exactly its
  bytes, and the Rust renderer must draw its far mesh at that tier.

## Where it lives

| Concept | Code |
| --- | --- |
| The tier rule and what an appearance carries past tier 3 | `Carries`, `FAR`, `MESH_SLOTS`, `detail_at`, `caster_tier` in [`view.rs`](../../../crates/presentation/src/view.rs) |
| A prop's mesh count, from its bundle | `Art.meshes` in [`props.rs`](../../../crates/presentation/src/static_world/props.rs) |
| The threshold and its check | `Detail.far_px` in [`world.rs`](../../../crates/presentation/src/world.rs); [`settings/validate.rs`](../../../crates/presentation/src/settings/validate.rs) |
| What a valid far mesh is | [`farMesh.ts`](../../../packages/scene-assets/src/farMesh.ts); `FAR_TIER` in [`schema.ts`](../../../packages/scene-assets/src/schema.ts) |
| Authoring one | `FAR` in [`parts.py`](../../../packages/scene-assets/blender/parts.py); the car in [`street_car.py`](../../../packages/scene-assets/blender/street_car.py) |
| Counting it | [`diagnostics.rs`](../../../crates/renderer-core/src/models/diagnostics.rs); [`counts.rs`](../../../native/client/src/counts.rs); `PROP_LEVELS` in [`visualPresentation.ts`](../../../web/src/battle/benchmark/visualPresentation.ts) |

Tests that pin it: [`props/tests/far.rs`](../../../crates/presentation/src/static_world/props/tests/far.rs),
[`far_mesh.rs`](../../../crates/renderer-core/tests/far_mesh.rs) with its
[bundle](../../../crates/renderer-core/tests/fixtures/far_prop.bundle),
[`farBundle.test.ts`](../../../web/tests/sceneAssets/farBundle.test.ts) and the
far cases in [`bake.test.ts`](../../../web/tests/sceneAssets/bake.test.ts).

## Dead ends

- **A reshaped car hull, 40 triangles.** 5.1 ms at the wide view, and visibly
  different under fog (rule 1).
- **Walls.** Without piers they read darker; with one pier they lose posts;
  with only unseen faces dropped they save 10 triangles
  ([decisions](decisions.md)).
- **Merging a block's props into one simplified mesh**
  ([verdict](assets/block-merge-verdict.md)). Separate props join in at most
  0.4% of the triangles; the saving is either each prop simplified alone or
  props deleted outright, and any reshaping breaks rule 1.
- **Cluster detail for terrain** ([verdict](assets/terrain-verdict.md)).
  Terrain is at most 2.6% of a frame's triangles. Generated maps have no
  hills, only carved rivers and swamps.
- **Small props outside the view casting with their far mesh.** 0.49 ms at the
  wide view for a second shadow rule. Left on branch
  `t3/far-detail-offview-casters-far`.
- **Raising the tier count to five everywhere, or re-cutting tier 3.** The
  first touches every unit, kit and tree for a need one prop kind has; the
  second changes what the tactical view draws.

## If more is wanted

The large saving needs fog to judge a far prop whole, the way it judges
remains, so that a far mesh may be a simple hull. That changes how every far
prop meets the edge of sight. It is a look to judge for its own sake, not an
optimisation to slip in.

After props, buildings and scenery each submit about 9.5 million shadow
triangles at the wide view, and at the whole-map view scenery is most of a
frame of well over a hundred million triangles.

## Evidence

- [The census](assets/census.md) and [the first probe](assets/ceiling.md).
- [The control scene](assets/control.json); the [older receipt](assets/baseline.json)
  stopped reproducing when the map generator changed.
- [Block merge](assets/block-merge-verdict.md) and [terrain](assets/terrain-verdict.md),
  each with its receipt.
- The before-and-after pictures were judged by reviewers given only the
  images, and by the user, and were not kept: the far mesh is accepted
  because nobody could tell the builds apart without a difference image.

The full repository run that normally closes a spec was not made, on the
user's instruction not to over-verify while `staging` moves; each change ran
the tests it reaches.
