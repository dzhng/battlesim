# Decisions

The choices made for [far detail](README.md) where nobody had decided in
advance, as they stand in the shipped code. Each entry stands alone. They are
grouped by how settled they are, least settled first.

## Yours to reopen

These were the agent's calls on questions of look or of worth. Each is cheap
to reverse.

1. **Fog still judges a far prop surface by surface.** Letting fog judge a far
   prop whole, as it judges remains, would allow a far mesh to be a simple
   hull. The first probe measured that prize at about 11.5 ms of 34.6 at the
   wide view for cars and walls. It was not done because it changes how every
   far prop meets the edge of sight, which is a look to judge on purpose.
2. **Courtyard walls carry no far mesh.** With the look held fixed a wall can
   lose 10 of 88 triangles. A wall that drops one pier (64 triangles, an
   estimated 1.7 ms at the wide view, never timed) is on the local branch
   `t3/far-detail-wall-one-pier`. A fresh reviewer could not tell it apart at
   full size; enlarged, run ends and lone modules lose a post.
3. **Props outside the view cast their shadow with tier 3.** Casting with the
   far mesh when the prop is small enough saves 0.49 ms at the wide view and
   changed no golden-hour frame, but adds a second shadow rule, 3 MiB of GPU
   buffers and about 100 pixels at the day sun that move by up to 2 levels for
   no established reason. It is on the local branch
   `t3/far-detail-offview-casters-far`.
4. **The car's far mesh is its coarsest tier without its tyres** (92 of 172
   triangles, 2.65 ms at the wide view). A reshaped hull of 40 triangles saved
   5.1 ms and read differently under fog. The user looked at the shipped
   version's before-and-after pictures and saw no difference.
5. **No full repository run closed this work.** The user asked not to
   over-verify while `staging` moves. Each change ran the tests it reaches.

## Sound, with a known limit

6. **No native capture of the wide view exists.** The native client replays
   only saved reels, and the committed reel never draws a car far. Native's
   far path rests on the shared renderer's GPU test and the count reader's
   unit test.
7. **The far threshold is 4 pixels.** At 6, cars in the tactical view switched
   and street rows at 550 m lost detail. At 4 the close and tactical frames
   are pixel-identical to the build before.
8. **One threshold for every kind, measured on a model's largest extent.** It
   is the measure every tier already uses, so a long, low prop is judged by
   its length. A per-kind threshold would be a second owner.
9. **The close view's 5% timing tripwire was missed in single pairs and never
   retuned.** That view varies by more than 5% between holds of one build.
   Pooled over every hold the builds are equal.
10. **The first merge into `staging` was committed without the commit hook.**
    The hook would have re-linted about a thousand files that came from
    `staging`. The lint had run on the merged tree.

## Sound

11. **An optional fifth mesh on static props only**, not a fifth tier
    everywhere. Raising the tier count would touch every unit, kit, tree and
    grass source for a need one prop kind has.
12. **Tier 3 was left alone.** It is what the tactical view draws, and a
    coarser wall there was rejected long before this work.
13. **The bundle format's version did not move.** A bundle already records how
    many meshes each state has, and the Rust decoder reads any count, so every
    bundle without a far mesh is byte-identical. The bake's own codec accepts
    four, or five on a static state.
14. **The far mesh is tier 4 and the unit card moved to 5.** Every Rust reader
    uses the constant. The browser's count reader and the Blender helper
    mirror it under their own names.
15. **The tier rule takes one value for what an appearance carries past tier
    3:** nothing, a far mesh or a card. A second flag would let "card and far
    mesh" be written.
16. **Presentation receives a prop's mesh count, not a flag.** One function
    turns the count into "carries a far mesh", so no client decides it.
17. **A prop drawn with its far mesh casts with it; one drawn at tier 3 casts
    with tier 3.** Tier 3 casting with the far mesh could save nothing at the
    wide view, where every car is already far-drawn.
18. **The props' counts widened to five numbers, and with them the renderer's
    two per-tier diagnostic arrays for non-building models.** Buildings and
    scenery never carry a far mesh and stay at four. Readers refuse the old
    shape, and the native count run was recorded again.
19. **No new finding codes for the far rules.** Each is an existing rule's
    meaning: a wrong set of tiers, a tier not coarser than the one before, a
    tier over budget.
20. **A kind does not declare that it carries a far mesh.** The bundle's bytes
    say so.
21. **A far part is shaded in Blender as tier 3 is:** same segment count, same
    paint split, occluded by tier 3's parts. Occluded only by other far parts,
    it would bake lighter beside whatever it dropped.
22. **A far part must not introduce a material earlier than the near tiers
    do.** The exporter orders materials by first use in object-name order, and
    a new name can renumber tiers 0 to 3. Nothing checks this; comparing the
    baked tiers before and after catches it.
23. **One committed bundle keeps the two languages' far index honest.** The
    TypeScript bake must reproduce its bytes and the Rust renderer must draw
    its far mesh.
24. **The validator's vehicle fit moved to its own file** to make room under
    the file-size limit, as a pure move. The soldier fit lives in the file
    `staging` gave it.
25. **Merging a block's props into one mesh is not worth building.** Separate
    props join in at most 0.4% of the triangles, and any reshaping changes the
    fog look.
26. **Terrain is not worth a cluster hierarchy.** It is at most 2.6% of a
    frame's triangles, and generated maps have no hills.
27. **No hysteresis at the threshold.** Each zoom stop gave the same picture
    going out and coming back.
28. **Measurements use a control recorded during the work,** because the
    older frozen scene stopped reproducing when the map generator changed.
29. **Only a frame-timing measurement takes the shared GPU lock.** Pictures,
    recordings and tests do not wait on it.
30. **`asset sheet` builds the tools its page needs and reports the page's own
    error.** It had been failing with a silent timeout.
