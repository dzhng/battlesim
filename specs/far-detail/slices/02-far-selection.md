# 02: Presentation picks the far mesh

## What this unlocks

A prop whose bundle carries a far mesh draws it below `far_px` and casts with
it, in both clients. No shipped appearance carries one yet, so the game's
pixels, counts and digests do not change.

## Seam

**The tier rule, `crates/presentation/src/view.rs`.**

- `FAR = 4` is the far mesh's tier. `IMPOSTOR` moves to 5. `TIERS` stays 4:
  the tiers every appearance has. One new constant is the number of mesh slots
  a drawn tier can name (5).
- `detail_at` and `model_detail` stop taking `impostor: bool`. They take one
  value saying what the appearance carries past tier 3: nothing, a far mesh, or
  a card. Carrying both cannot be written.
- Order of the choice: above `lod_px[0]`, `[1]`, `[2]` as today. Below that, a
  card-carrying model takes the card under `impostor_px` and tier 3 otherwise,
  as today. A far-carrying model takes `FAR` under `far_px` and tier 3
  otherwise. Anything else takes tier 3.
- `caster_tier` keeps today's answer for tiers 0 to 3 and returns `FAR` for
  `FAR`. `batch_caster` follows it.
- The projected size is the one every tier already uses: the model's height in
  pixels at its distance.

**Data.**

- `Detail` (`crates/presentation/src/world.rs`) gains `far_px`, required, with
  no default. `Detail::scaled` leaves it alone: only units are scaled, and no
  unit carries a far mesh.
- `fixtures/presentation.json` `models.far_px` gets a first value under
  `lod_px[2]`. Slice 03 tunes it.
- `crates/presentation/src/settings/validate.rs` refuses `far_px` outside
  `0 < far_px < lod_px[2]`, and `lod_px` that does not fall and stay positive.
- The browser's typed copy of the detail (`ModelDetail` in
  `web/src/battle/sim/presentationModels.ts`) gains the field for typing only.

**Props, `crates/presentation/src/static_world/props.rs`.**

- `Art` gains whether its states carry a far mesh, read from the bundle's
  mesh count (the bake guarantees all of an appearance's states agree).
  All three places that assemble `Art` fill it from the decoded bytes in one
  commit: the browser's `propArt()`
  (`packages/battle-renderer/src/models/propAppearance.ts`), the native
  `prop_input` (`static_world/files.rs`), and art replacement's `patch_props`
  (`presentation/catalog/static_art.rs`). No caller passes a flag of its own.
- `select` passes each look's carried value to `model_detail`. A prop outside
  the view casts with tier 3, as today.
- The props' list strides (`select`, `each_list`, `props/admission.rs`) use the
  mesh-slot count. Buildings, trees and dressing keep `TIERS`.

**Drawing and counting.**

- `crates/renderer-core/src/models.rs` already draws whatever tier a batch
  names and treats `IMPOSTOR` as the card; it follows the renumber.
- The per-tier arrays that a drawn tier indexes take the mesh-slot count:
  `crates/renderer-core/src/models/diagnostics.rs` (it would index out of
  bounds on tier 4 today) and the props' per-appearance counts.
- The count layout widens for props only, in every reader at once:
  `web/src/battle/benchmark/visualPresentation.ts`,
  `web/src/battle/render/sceneStats.ts`, `native/client/src/counts.rs` (which
  today clamps a tier to 3 and would hide the far mesh), and the committed run
  under `fixtures/parity/capture/counts`, recorded again. Readers refuse the
  four-number shape.
- The model lab (`crates/presentation/src/presentation/lab.rs`) can hold a
  far-carrying model at its far mesh, the way it can hold a unit at its card.

## What the user can see

In the model workbench, a synthetic far-carrying prop beside an ordinary prop
and a card-carrying unit, zoomed through the threshold: the first switches to
its far mesh, the second stays on tier 3, the third goes to its card.

## Tests, written first

Presentation tests set their own `Detail` and fake art.

1. A far-carrying prop lists at `FAR` just below the threshold and at tier 3
   just above it.
2. A prop without a far mesh lists at tier 3 at every small size, and is never
   listed at a tier its art lacks.
3. A card-carrying unit still takes its card below the card threshold and
   never the far tier.
4. A visible prop drawn at `FAR` casts with `FAR`; drawn at tier 3 it casts
   with tier 3; outside the view it stays in its cascades and casts with
   tier 3. The tiny-visible shadow skip is unchanged.
5. Replacing a prop's art with far-carrying art, and back, changes what it
   draws on the next frame and loses no learned state.
6. Settings with `far_px` at zero, negative, or not below `lod_px[2]` are
   refused with the field named; so is `lod_px` that rises.

Renderer, under the GPU lock:

7. A batch at `FAR` submits exactly that row's far-mesh triangles and draws
   opaque pixels without the card atlas; a card still draws from the atlas.
8. After [slice 01](01-far-mesh-in-bundles.md): a bundle baked by the
   TypeScript bake with five meshes is decoded by Rust, reported to
   presentation as five, and drawn at tier 4. This is the test that keeps the
   two languages' far index honest.

Then the equality check, which reuses slice 00's numbers: at the wide hold,
submitted triangles, instance counts and the digest equal `staging`.

## Delegated

- The carried value's type and names.
- The mesh-slot constant's name.
- Test helper shapes, and whether existing `Detail` literals in tests gain a
  constructor.

## Must stay green

Presentation view, props, settings and catalog tests; renderer-core
asset-generation and card tests; native `counts` and `props`;
`cargo check -p renderer-core --target wasm32-unknown-unknown`; the frame
baselines, pixel for pixel.

## Feedback that would change this slice

- "Tier-3 props should cast with the far mesh too": it saves more shadow work
  and changes shadows at tactical range. It is one line in `caster_tier`, and
  it is measured as its own named arm in slice 03 before it is adopted.
- "Use the prop's longest side, not its height": a long low prop would then
  switch later. Adopt it only if a kind's crop fails at the size where cars
  pass.
