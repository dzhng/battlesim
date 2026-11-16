# Decisions

The choices made for [far detail](README.md) where nobody had decided in
advance, as they stand in the shipped code. Each entry stands alone. They are
grouped by how settled they are, least settled first.

## Decided with the user

1. **A far mesh is judged at normal size, by the player's eye.** The work
   first held far meshes to pixel parity under enlarged difference images and
   shipped a car that only lost its tyres, for 2.65 ms. The user looked at the
   rejected hull, saw no difference at twelve times zoom, and asked for the
   large saving. Everything below follows from that.
2. **Every static prop's far mesh is a generated box,** ten triangles, made at
   bake from the prop's coarsest tier. Hand-authoring sixty kinds was the
   wrong shape. The user: "why can't the lowest lod car literally be a black
   box on top of a white box?", and "same goes for all other props".
3. **The car authors its own: a paint box under a dark glass box.** A single
   generated box cannot carry both the paint and the dark cabin.
4. **A prop under 5 projected pixels casts no shadow, in view or out of it.**
   The user's idea: nobody notices a one-pixel car's shadow. It replaced a
   narrower candidate, off-view props casting with their far mesh, which was
   deleted.
5. **The shadow threshold follows the graphics tier: 5 at High, 7 at Medium,
   9 at Low.** The user's choice. It is one size for every kind, authored once
   as 5 and scaled by the tier's `propShadowScale` like the other detail
   scales; the native client keeps 5. At 5 a car stops casting beyond about
   1,280 m of camera depth. At 9 a car's shadow stops at about 700 m, and
   small props 260 m away at the tactical view start losing theirs: counted
   earlier as 17, 99, 289 and 541 casters lost across the four cascades of
   about 4,700 to 13,300. High keeps the tactical view's shadows whole; the
   cheaper tiers trade them for fewer casters.
6. **Fog still judges a far prop surface by surface.** A box at the edge of
   sight can take the fog's unseen look differently from its real mesh.
   Changing that would only remove a difference nobody can see.
7. **No full repository run closed this work,** and the whole-catalog asset
   check was skipped on the last push. The user asked not to over-verify
   while `staging` moves. Each change ran the tests it reaches.

## Sound, with a known limit

8. **No native capture of the wide view exists.** The native client replays
   only saved reels, and the committed reel never draws a car far. Native's
   far path rests on the shared renderer's GPU test and the count reader's
   unit test.
9. **The far threshold is 4 pixels.** At 6, cars in the tactical view switched
   and street rows at 550 m lost detail. At 4 the close and tactical frames
   are pixel-identical to the build before.
10. **One threshold for every kind, measured on a model's largest extent.** It
   is the measure every tier already uses, so a long, low prop is judged by
   its length. A per-kind threshold would be a second owner.
11. **The close view's 5% timing tripwire was missed in single pairs and never
   retuned.** That view varies by more than 5% between holds of one build.
   Pooled over every hold the builds are equal.
12. **The first merge into `staging` was committed without the commit hook.**
    The hook would have re-linted about a thousand files that came from
    `staging`. The lint had run on the merged tree.

## Sound

13. **An optional fifth mesh on static props only**, not a fifth tier
    everywhere. Raising the tier count would touch every unit, kit, tree and
    grass source for a need one prop kind has.
14. **Tier 3 was left alone.** It is what the tactical view draws, and a
    coarser wall there was rejected long before this work.
15. **The bundle format's version did not move.** A bundle already records how
    many meshes each state has, and the Rust decoder reads any count, so every
    bundle without a far mesh is byte-identical. The bake's own codec accepts
    four, or five on a static state.
16. **The far mesh is tier 4 and the unit card moved to 5.** Every Rust reader
    uses the constant. The browser's count reader and the Blender helper
    mirror it under their own names.
17. **The tier rule takes one value for what an appearance carries past tier
    3:** nothing, a far mesh or a card. A second flag would let "card and far
    mesh" be written.
18. **Presentation receives a prop's mesh count, not a flag.** One function
    turns the count into "carries a far mesh", so no client decides it.
19. **A prop drawn with its far mesh casts with it; one drawn at tier 3 casts
    with tier 3.** Tier 3 casting with the far mesh could save nothing at the
    wide view, where every car is already far-drawn.
20. **The props' counts widened to five numbers, and with them the renderer's
    two per-tier diagnostic arrays for non-building models.** Buildings and
    scenery never carry a far mesh and stay at four. Readers refuse the old
    shape, and the native count run was recorded again.
21. **No new finding codes for the far rules.** Each is an existing rule's
    meaning: a wrong set of tiers, a tier not coarser than the one before, a
    tier over budget.
22. **A kind does not declare that it carries a far mesh.** The bundle's bytes
    say so.
23. **A far part is shaded in Blender as tier 3 is:** same segment count, same
    paint split, occluded by tier 3's parts. Occluded only by other far parts,
    it would bake lighter beside whatever it dropped.
24. **A far part must not introduce a material earlier than the near tiers
    do.** The exporter orders materials by first use in object-name order, and
    a new name can renumber tiers 0 to 3. Nothing checks this; comparing the
    baked tiers before and after catches it.
25. **One committed bundle keeps the two languages' far index honest.** The
    TypeScript bake must reproduce its bytes and the Rust renderer must draw
    its far mesh.
26. **The validator's vehicle fit moved to its own file** to make room under
    the file-size limit, as a pure move. The soldier fit lives in the file
    `staging` gave it.
27. **Merging a block's props into one mesh is not worth building.** Separate
    props join in at most 0.4% of the triangles, and any reshaping changes the
    fog look.
28. **Terrain is not worth a cluster hierarchy.** It is at most 2.6% of a
    frame's triangles, and generated maps have no hills.
29. **No hysteresis at the threshold.** Each zoom stop gave the same picture
    going out and coming back.
30. **Measurements use a control recorded during the work,** because the
    older frozen scene stopped reproducing when the map generator changed.
31. **Only a frame-timing measurement takes the shared GPU lock.** Pictures,
    recordings and tests do not wait on it.
32. **`asset sheet` builds the tools its page needs and reports the page's own
    error.** It had been failing with a silent timeout.
