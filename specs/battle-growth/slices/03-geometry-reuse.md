# 03 Geometry reuse

**Question:** can a second generation reuse every unchanged model's packed
geometry and draw the same picture as a clean build?

**Depends on:** 02.

## What happens today

[`Models::from_world`](../../../crates/renderer-core/src/models/admission.rs)
gives each bundle a material base in the order its name is first seen, and
[`pack_vertices`](../../../crates/renderer-core/src/models/geometry.rs) writes
`base + material` into every vertex as a 16-bit number that wraps silently.
Unit rows sort by name, so a new type renumbers the bases after it and every
vertex buffer changes. That is the 1.56 s.

## Contract

A model's geometry is packed and uploaded once per scene. Its vertex bytes do
not depend on the generation.

## Seam

- **The art store:** one per scene and device, owned by
  [`Runtime`](../../../crates/renderer-core/src/runtime/mod.rs) and shared with
  a pending generation.
- **Material slots:** assigned per `ContentId`, append-only and idempotent. A
  base never changes while the scene lives. A slot reserved by a pending
  generation that is later dropped stays reserved, so cancelled work cannot
  renumber anything. Passing the 16-bit range is an error, never a wrap.
- **Geometry index:** weak, keyed by content, state, the pose that was baked
  into the mesh (a corpse's rest palette) and the track-side signature. It
  replaces the per-admission map. A generation holds the geometry it draws;
  when the last one drops, the buffers go.
- **The materials table** stays per generation: it is small, and it is rebuilt
  from the slot table each time. Row tables are rebuilt too, because rows
  renumber.
- **Full rebuild:** an admission with a fresh store. It runs for the first
  scene, a recreated device, and when dead slots pass a bound or slots run
  out; the report says which.

## Verification

Write first; test 1 is red today.

1. Commit {x, y}, then stage {a, x, y} where `a` sorts first: vertex bytes
   packed and buffer bytes uploaded equal `a`'s alone.
2. The committed frame is pixel-equal to a clean runtime built with
   {a, x, y}.
3. A changed corpse pose under the same bundle is a miss.
4. The same name with new content is replaced.
5. A slot bound set by the test forces the fresh-store path, with the same
   picture; running out of the 16-bit range is an error.
6. Slot assignment alone, without a GPU: reordered arrival and exhaustion.

## What you can run or see

`battle-growth` at high graphics: geometry bytes at growth are the new type's,
and the largest part of the freeze is gone.

## Delegated

The store's internal structure and names. The dead-slot bound's value: a
tripwire well above what a long battle reaches, logged with its kind, bound,
owner and outcome when it fires.

## Stays green

`tests/model_geometry.rs` (rows that route the same art share one mesh),
`models/tests.rs` (track flags follow a replaced rig), the native client's
startup.

## Feedback that would change this slice

If test 2 fails for a reason the key does not cover, widen the key here.
