# 00: Census and ceiling probe

Scratch only. No production file changes.

## What this unlocks

Two answers the later slices stand on:

1. **Census.** Which prop appearances cost the most in the wide view today,
   counted, not remembered.
2. **Ceiling.** Whether drawing those props from a far-class mesh saves GPU
   time worth the new tier. The last measurement of this mechanism (cars from
   300 to 172 triangles, about 3.9 ms) predates the rule that lets a tiny
   visible prop skip its shadow, so it is a hint and not a promise.

## Seam

Existing surfaces only:

- the renderer's per-appearance, per-tier instance counts
  (`crates/renderer-core/src/models/diagnostics.rs`) and its submitted
  triangles by pass (`crates/renderer-core/src/submitted.rs`);
- the prop population's caster lists
  (`crates/presentation/src/static_world/props.rs`);
- the paused view-cost hold (`web/scenes/endurance.mjs`,
  `web/scenes/_viewCost.mjs`) at the [receipt](../assets/baseline.json).

The probe needs no new tier. In a scratch checkout, give the top census kinds
a far-class mesh **as their tier 3** (a closed hull of a few dozen triangles
in the kind's own paint), bake, and build. That is deliberately wrong at
tactical range, where tier 3 is twenty pixels tall. It is honest at the wide
hold, where every one of those props is a pixel or two and would draw the far
mesh in the finished feature too.

## Steps

1. Reproduce the receipt on a production build of `staging`. Record the digest
   and the applied camera. Record the player's `zoom_max` and whether 2,400 m
   is inside it.
2. Write the census to `assets/census.md`: per appearance, instances in view at
   each tier, tier-3 triangles, view triangles, how many cast and into how many
   cascades, and the projected size in pixels (5th, 50th and 95th centile) at
   the wide hold. Rank by submitted triangles, view and cast together.
3. Author far-class tier-3 meshes for the top one or two kinds. Parked cars
   are expected first.
4. Measure by [the protocol](../README.md#measuring): one A against A pair,
   then three interleaved pairs, wide hold only. Assert equal instance counts
   and the expected triangle drop before reading time.
5. Write the verdict.

## What the user can see

`assets/census.md`, and `assets/ceiling.md` with the pair table and a
one-paragraph verdict. No pictures are judged: the probe is knowingly wrong
close up.

## Verdict

- **Go** when the candidate wins at least two of three wide pairs by more than
  the A against A spread. Slices 01 to 04 proceed, and the census names the
  kinds for 03 and 04.
- **No-go** otherwise. Record the numbers and what the census shows about
  where the time went (view pass, cast pass, or neither). Stop the production
  slices and reslice in the README. The requirement stands; this
  implementation does not.

Stop after one working session of measurement and at most one rerun for a
named, fixed cause.

## Delegated

- The probe meshes' exact shape.
- How the census is read out (a lab call or a scene script).
- Whether the probe edits the Blender source or swaps the mesh at bake, as
  long as both arms are production builds.

## Must stay green

Nothing changes in the repository except `specs/far-detail/assets/`. Delete
the scratch checkout and its build once the verdict is written.

## Feedback that would change this slice

- "Measure a different view": add it as a second hold here, because every later
  comparison inherits this slice's views.
- "Skip the probe, just build it": the census still runs; the probe is the
  only cheap proof that the tier is worth its plumbing.
