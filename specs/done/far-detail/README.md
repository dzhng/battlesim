# Far detail

**Status:** closed. A prop a few pixels long draws as a box, and a prop too
small to see casts no shadow. The wide view's GPU time fell from 34.6 to
18.8 ms. **Closed:** 2026-11-16.

## What it is for

In the wide view the game draws about 120,000 props, most of them a pixel or
so across, each at the same mesh it uses at twenty pixels. Parked cars and
courtyard walls are 81% of the triangles those props submit there, and their
shadow casts cost about five times one view pass ([census](assets/census.md)).

The idea came from Unreal's Nanite and its three.js port,
[three-nanite](https://github.com/apekshik/three-nanite/tree/b730aac13ab0ea67b75214613eea56c14151f6e5):
choose how much geometry to draw from its error on screen. Only that idea
transfers. Our props' coarsest meshes are 40 to 170 triangles, about one of
its 128-triangle clusters, and its answer to many triangles inside a pixel is
a rasteriser WebGPU cannot host.

## What shipped

- **A far mesh.** A static prop's bundle carries a fifth, coarsest mesh. A
  prop draws it below `presentation.models.far_px` (its largest extent, in
  projected pixels) and casts with it while it does.
- **The far mesh is a box.** The bake makes it from the prop's coarsest tier:
  ten triangles, no bottom, as tall as the prop, its footprint sized to what
  the prop covers from above, each face in the material that covers most of
  the prop from that side. A source may author its own instead. The parked car
  does: a paint box under a dark glass box, 20 triangles against 172.
- **Small props cast no shadow.** A prop whose projected footprint is under
  `presentation.models.shadow_caster_px` casts nothing, in view or out of it.

Measured on the [control scene](assets/control.json) in interleaved pairs
([measurements](assets/measurements.md)), in two steps:

| View | First step: the car alone, its tyres dropped | Second step: boxes for every prop, and the shadow rule |
| --- | --- | --- |
| Close, 65 m | none | none |
| Tactical, 260 m | none | none; 655 props in the frame's far strip draw far |
| Wide, 2,400 m | 34.59 to 31.94 ms | 31.95 to 18.75 ms |
| Whole map, 19,500 m | 1.95 ms lower | 50.51 to 42.46 ms |

The simulation, replays and what a side is told are untouched.

## The bar is what the player sees

The work first held itself to a stricter bar: no difference a reviewer could
find, even in enlarged crops with an amplified difference image. Under that
bar almost nothing survives. A reshaped car reads differently under the fog
of war, which judges each surface by its own facing
(`fogSeenBy` in `crates/renderer-core/src/shaders/modules/fog.wgsl`); a wall
without a pier loses a post; even an unseen underside can tie the nearest
depth and move a pixel. That version shipped first and saved 2.65 ms.

The user then looked at the rejected pictures, saw no difference at twelve
times zoom, and set the bar this record now keeps: **a far mesh is judged by
how the prop reads in a full frame at normal size.** Boxes and missing
shadows change many pixels by small amounts, about 155,000 at the 850 m stop,
and nobody looking at the frames can tell.

So the differences below are known and accepted, not defects to fix:

- far props at the edge of sight can take or lose the fog's unseen look
  differently from their real mesh;
- a far wall has no piers and a far car no wheels or window outlines;
- beyond about 1,280 m from the camera a car casts no shadow, a wall beyond
  about 820 m, a bin beyond about 310 m.

## Invariants

- Tiers 0 to 3 of every prop are what they were. The far mesh is added beside
  them, and nothing asks for a mesh a bundle lacks.
- An authored far mesh (`_LOD4` in the source) wins over the generated box.
  Either way it has fewer triangles than tier 3, every state of an appearance
  carries one or none does, and only a prop kind may. Units, kits, trees,
  dressing and grass keep exactly four tiers.
- A prop whose coarsest tier is already about a box gets no far mesh.
- At the tactical view the two nearest shadow cascades keep every caster.
  The shadow threshold is one size for every kind, so it cannot also cut a
  car's shadow nearer than about 1,280 m without removing a bin's at 260 m.
- Whether an appearance has a far mesh is read from its bundle's bytes. No
  list of names decides it, and no renderer code picks a tier.
- `far_px` is required and sits above zero and below the last `lod_px` entry.
  There is no setting for either threshold.
- The far mesh's index is the same number in both languages. One committed
  bundle holds them together: the TypeScript bake must produce exactly its
  bytes, and the Rust renderer must draw its far mesh at that tier.

## Where it lives

| Concept | Code |
| --- | --- |
| The tier rule and what an appearance carries past tier 3 | `Carries`, `FAR`, `MESH_SLOTS`, `detail_at`, `caster_tier` in [`view.rs`](../../../crates/presentation/src/view.rs) |
| A prop's mesh count, from its bundle | `Art.meshes` in [`props.rs`](../../../crates/presentation/src/static_world/props.rs) |
| The threshold and its check | `Detail.far_px` in [`world.rs`](../../../crates/presentation/src/world.rs); [`settings/validate.rs`](../../../crates/presentation/src/settings/validate.rs) |
| The generated box | [`farBox.ts`](../../../packages/scene-assets/src/farBox.ts) |
| What a valid far mesh is | [`farMesh.ts`](../../../packages/scene-assets/src/farMesh.ts); `FAR_TIER` in [`schema.ts`](../../../packages/scene-assets/src/schema.ts) |
| Which props cast | `shadow_caster_px` and `projected_footprint_px` in [`props.rs`](../../../crates/presentation/src/static_world/props.rs) |
| Authoring one by hand | `FAR` in [`parts.py`](../../../packages/scene-assets/blender/parts.py); the car in [`street_car.py`](../../../packages/scene-assets/blender/street_car.py) |
| Counting it | [`diagnostics.rs`](../../../crates/renderer-core/src/models/diagnostics.rs); [`counts.rs`](../../../native/client/src/counts.rs); `PROP_LEVELS` in [`visualPresentation.ts`](../../../web/src/battle/benchmark/visualPresentation.ts) |

Tests that pin it: [`props/tests/far.rs`](../../../crates/presentation/src/static_world/props/tests/far.rs),
[`far_mesh.rs`](../../../crates/renderer-core/tests/far_mesh.rs) with its
[bundle](../../../crates/renderer-core/tests/fixtures/far_prop.bundle),
[`farBundle.test.ts`](../../../web/tests/sceneAssets/farBundle.test.ts) and the
far cases in [`bake.test.ts`](../../../web/tests/sceneAssets/bake.test.ts).

## Dead ends

- **Holding a far mesh to pixel parity.** It left a car's tyres and nothing
  on a wall. Three wall meshes and two car meshes were made and measured
  under that bar before it was dropped ([measurements](assets/measurements.md)).
- **Merging a block's props into one simplified mesh**
  ([verdict](assets/block-merge-verdict.md)). Separate props join in at most
  0.4% of the triangles; the saving is each prop simplified alone, which a box
  per prop already is.
- **Cluster detail for terrain** ([verdict](assets/terrain-verdict.md)).
  Terrain is at most 2.6% of a frame's triangles. Generated maps have no
  hills, only carved rivers and swamps.
- **Changing how fog judges a far prop,** so a box would match its real mesh
  under fog. It would only remove a difference nobody can see.
- **Off-view props casting with their far mesh.** The shadow rule replaced
  it: a prop that small casts nothing.
- **Raising the tier count to five everywhere, or re-cutting tier 3.** The
  first touches every unit, kit and tree; the second changes what the
  tactical view draws.

## What is left in the wide view

After this, buildings and scenery are the larger share: each submits about
9.5 million shadow triangles at the wide view, and at the whole-map view
scenery is most of a frame of well over a hundred million triangles.
`laundry_poles` has no far mesh, because it covers nothing seen from above.

## Evidence

- [The census](assets/census.md) and [the first probe](assets/ceiling.md).
- [What was measured after the probe](assets/measurements.md): the shipped
  car, and each rejected candidate.
- [The control scene](assets/control.json); the [older receipt](assets/baseline.json)
  stopped reproducing when the map generator changed.
- [Block merge](assets/block-merge-verdict.md) and [terrain](assets/terrain-verdict.md),
  each with its receipt.
- The before-and-after pictures were judged by the user at normal size, at
  the day sun and at golden hour, and were not kept.

The full repository run that normally closes a spec was not made, on the
user's instruction not to over-verify while `staging` moves; each change ran
the tests it reaches.
