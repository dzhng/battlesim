# 13 A body draws its type's mesh

**Question:** can a dead soldier be drawn from his type's one mesh, so no
posed copy of it is ever packed?

**Depends on:** 09. Added 2026-11-15, replacing [slice 12](12-an-arrival-over-frames.md),
whose first confirm-first question found this.

## What happens today

A skinned appearance has one row for the living body and one for each way it
lies dead ([`register_appearance`](../../../crates/presentation/src/presentation/posing.rs)).
The living row draws the authored mesh, skinned each frame by a palette. Each
corpse row draws a copy of that mesh skinned once on the CPU at admission
([`posed_mesh`](../../../crates/renderer-core/src/models/geometry.rs), the
`lies != NONE` branch of
[`Models::from_world`](../../../crates/renderer-core/src/models/admission.rs)),
all four tiers of it, keyed by its pose.

So every infantry appearance is packed three times: once living and twice
dead. The reference battle's RPG team is 85.7 MB of packed vertices, 57.2 MB
of it in 18 corpse keys, packed in the one frame the team is first seen and
before any of them has died. The same is true of every infantry type in the
first scene, the player's own roster included.

## Confirm first

- How a corpse instance reaches the GPU today (pipeline, batch, held or
  per-frame) and what the living path needs that a corpse row lacks: a row's
  joint count, a palette in the frame, anything in the instance record.
- That a corpse row's card and bounds already come from the unposed mesh and
  a palette, as the living far pose's do. If not, say what differs.
- The cost of drawing bodies skinned: with the corpse cap's worth of bodies
  near the camera at high graphics, GPU and CPU frame time before and after
  (the renderer's own GPU timing), three runs each. This is the number the
  slice turns on.

## Contract

- A body lies as its fall ended, drawn from its appearance's living mesh held
  in that pose. No geometry is packed, uploaded or keyed for a pose.
- It is the same picture: a corpse draws as the living body would if frozen
  at its fall's last frame, near and far.
- Bodies of one appearance in one pose share one palette, however many lie.
- Nothing about who sees a body, when it sinks away or how it is picked
  changes.

## Seam

- A corpse row routes to its appearance's living geometry and names a rest
  palette; the store's geometry key loses its pose. `posed_mesh` stays only
  where bounds are derived, or goes if nothing else needs it.
- Presentation owns the rest palettes already
  ([`rest_palette`](../../../crates/presentation/src/presentation/posing.rs));
  the frame carries one per corpse row that has bodies, not one per body.
- The card key keeps the rendered rest palette: a corpse's card is still its
  own.

**Fixed:** no new pipeline or shader: a body uses the living body's. No
switch or setting. `AdmissionWork` keeps its fields; their values fall. The
700 ms tripwire in `battle-growth` is not loosened.

## Verification

Write first; test 1 is red today.

1. In `crates/renderer-core/tests/asset_generations/`: staging a skinned
   appearance with corpse rows packs its living tiers' bytes and no more, and
   holds one geometry per content.
2. A frame of a corpse row equals a frame of the living row drawn with that
   row's rest palette, at a near tier and as a card.
3. Many bodies of one row add one palette to the frame.
4. `battle-growth` and its helper compute the expected vertex bytes from the
   living meshes; its other assertions are unchanged.

Then, in the ledger: the reference battle's infantry arrival before and
after (longest frame, staging to commit, vertex bytes), the first scene's GPU
buffer bytes and whole-browser settled memory before and after, and the
confirm-first frame times.

## What you can run or see

An infantry team's first sighting no longer hitches; a battlefield of bodies
looks as it did.

## Delegated

Where the per-row palette is written and held, the names, and whether
`posed_mesh` survives.

## Stays green

Everything in `asset_generations`, `tests/impostors.rs`, `model_geometry`,
the presentation tests of falls and corpses, and the scenes `battle-growth`,
`stand-in`, `armory` and `workbench`. A scene baseline that shows bodies may
move by rounding only; show any that moves.

## Feedback that would change this slice

- Bodies drawn skinned cost a frame time a player would feel at the corpse
  cap. Then stop and report the numbers: the alternative is to keep the
  copies and pack a pose's when its first body lies, which needs a decision
  about what a body draws until then.
- A corpse's picture differs from the frozen living body by more than
  rounding. Record where.
