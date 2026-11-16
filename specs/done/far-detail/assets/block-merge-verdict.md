# Block merge: verdict

**No-go.** Recorded 2026-11-16 for [far detail](../README.md). The numbers are
in the [receipt](block-merge-receipt.json).

Merging a block's far-drawn props into one simplified mesh can't keep the game's look. Even if it
could, it has nothing to save:

- **Islands don't simplify across each other.** The pinned reference builder joins two props in at
  most 0.4% of a cut's triangles. Every triangle it saves is either a prop reshaped on its own (what
  a per-kind far mesh already does) or a prop removed.
- **Reshaping changes the fog.** The side's fog of war judges every surface by its own plane and
  facing. Even at errors where every prop survives, 12–25% of the simplified area lies in planes its
  prop never had. Slice 03 showed that this kind of change is visible at the wide hold.

The two rungs that ran settle it, so the ceiling's GPU session and the drawing rung were not run.

## Rung 1: fog and paint (reading, then one measurement)

What the code does:
- **Fog per surface.** A prop draws with fog class `FOG_FACES` (`view::model_fog`). For every pixel,
  the model shader passes the surface's world position and interpolated normal to `fogSeenBy`
  (`renderer-core/src/shaders/modules/fog.wgsl`).
  - It probes `faceProbeM` out along the normal.
  - It refuses a face turned away from the eye.
  - It widens the horizon test by the face's incline to the line of sight.
  - It lets tops with `normal.z ≥ 0.7` pass by the roof rule.
- **Paint per instance.** A prop's paint is the instance's own `tint`, mixed in through each
  material's tint mask. The instance anchor matters to fog only for remains (`FOG_GROUND`).

What a merged mesh would need:

- **To keep each member's fog look:** each member's own surfaces, in their own planes. The fog test
  needs nothing more from a vertex than its position and normal. So a block that only concatenates
  its members' meshes, with no simplification, is fogged exactly like them.
- **To keep each member's paint:** per-vertex paint, or a member index into a table of paints. The
  vertex colour slot is already the bundle's own colour, and the tint mask can come from a texture,
  so baking paint into colour isn't exact. Either way it is a new vertex attribute and a new model
  pipeline variant.
- **The measurement.** For each cut, the area of simplified triangles lying in no plane of their own
  prop's original faces (normals within 2.6°, offset within 2 cm):

  | Wide-hold error | 128 m block | 256 m block | Walls only |
  | --- | --- | --- | --- |
  | 0.1 px (0.36 m) | 12.4% | 17.7% | 3.7% |
  | 0.25 px (0.91 m) | 24.8% | 24.7% | 15.3% |
  | 0.4 px (1.45 m) | 81.6% | 88.0% | 58.3% |

- **Keeping the look while simplifying across instances** would take a different fog rule for far
  props. The candidate is to judge a far prop whole, from its anchor, the way remains are judged.
  - That adds a per-vertex anchor or member index.
  - It changes how *every* far prop meets the edge of sight, not only merged ones; otherwise merged
    and unmerged props would disagree at the same distance.
  - Judging it would need a capture at the wide hold, an unprimed critique and the user's taste
    call.
  - Rung 2 shows the merge would still save nothing worth it, so that change isn't recommended for
    this purpose.

## Rung 2: does merging collapse anything (offline, no GPU)

**The scene.**
- Metro XL seed 4, generated natively from the game's documents: generator `layout-22`, presets
  `layout-presets-22`, map hash `28fec86e…`.
- Its props fitted as native presentation fits them, at `control.json`'s wide hold.
- 120,762 prop models are in view, every one at tier 3: the census's count, within 0.3%.
- They project 0.5–1.3 px (5th to 95th percentile).

**The reference, reproduced first.**
- three-nanite at `b730aac`, its unchanged `buildClod` on meshoptimizer 1.3.0.
- On its own connected sample (262,144 triangles) every one of its smoke checks passes:
  - 11 levels and a 94-triangle root;
  - `verifyClod`: no cracks, no monotonicity violations;
  - fewer triangles far away.

**The blocks.**
- The densest 128 m and 256 m squares near the hold's target, merged in world space: 417 props /
  46,780 triangles and 1,439 / 161,908.
- An adversarial fixture: the 128 m block's 247 courtyard wall modules alone. They touch end to end,
  with 1,733 shared corners, so runs are connected; this is the slice's "merge walls by run" arm.
- Cars draw tier 3 (172 triangles; slice 03's far mesh is not merged), walls 88. Cars and walls are
  about 87% of each block's triangles.

**Triangles against error.** The error is converted to pixels at the block's distance from the
wide-hold eye (about 2,320 m). "Props kept" counts props that still have any triangle; "across"
counts triangles whose corners belong to different props.

| Wide-hold error | 128 m block: triangles, props kept, across | 256 m block | Walls only |
| --- | --- | --- | --- |
| 0.05 px (0.18 m) | 100%, 100%, 0 | 99.5%, 100%, 0 | 100%, 100%, 0 |
| 0.1 px | 72.1%, 99.0%, 41 | 62.1%, 98.5%, 147 | 90.9%, 100%, 0 |
| 0.25 px | 21.4%, 93.0%, 12 | 22.4%, 93.3%, 91 | 23.5%, 100%, 18 |
| 0.4 px | 3.5%, 72.9%, 0 | 2.2%, 55.9%, 0 | 6.6%, 94.3%, 0 |
| 0.5 px | 0.1%, 3.6%, 0 | 0.03%, 0.7%, 0 | 4.1%, 70.4%, 0 |
| 1 px (3.6 m) | 54 triangles, 15 props, 0 | 50 triangles, 10 props, 0 | 0.2%, 6.1%, 0 |

At the whole-map view (19,500 m), 0.25 px is already 7.6 m of error, and the 128 m block is
54 triangles on 15 of its 417 props.

**What this says:**
- **Islands don't simplify across each other.** Even connected wall runs join modules in at most
  18 of 5,100 triangles. The builder never fell back to vertex clustering, the one step that could
  fuse islands.
- **Below 0.1 px nothing is saved. From 0.4 px on, the saving is props removed outright.** That
  breaks the plan's firewall that a visible object never vanishes.
- **Between the two, the saving is each prop simplified alone,** with 12–25% of the area in new
  planes. That is what an authored, instanced far mesh does per kind, without generated geometry.
  Slice 03 showed the far mesh must keep tier 3's own surfaces.
- **At the plan's own bar** ("well under the instanced total at a one-pixel error"), the only way
  under is deleting all but 1–4% of the props.

## The slice's other questions

1. **Side knowledge.** Not reached. A block would be valid only while its side knows no hidden,
   replaced, shoved or destroyed member; that is `PropPopulation::refit`'s knowledge today.
2. **Shadows.** Not reached.
3. **Per-instance paint and per-face fog.** Answered by rung 1: both survive concatenation, given
   per-vertex paint or a member index. Neither survives simplification.
4. **Identity and lifetime.** Not reached. A generated block would need a key over its members' art,
   placements and paints, with one owner in both clients.
5. **Build cost.**
   - The reference builder (JavaScript on meshoptimizer's WebAssembly) takes 0.21 s and +52 MB for
     the 128 m block, 0.81 s and +216 MB for the 256 m block.
   - At that rate, all 213,874 prop models on the map would take on the order of two minutes at load.
     For comparison, map generation is 67 s natively.
   - A Rust simplifier for the browser: the `meshopt` crate (meshoptimizer's C++ through `cc`)
     compiles for `wasm32-unknown-unknown` but does not link on macOS. Apple's `ar` writes no wasm
     symbol index (`undefined symbol: meshopt_simplify`), and the crate switches to `llvm-ar` only on
     Windows hosts. It would need an LLVM archiver on every build machine, or a pure-Rust simplifier.

## What would reopen it

- **A fog rule that judges far props whole,** if the user accepts its look at the wide hold for its
  own sake. Even then, merging needs a builder that joins neighbours into shared hulls (a car row as
  one box, a wall run as one prism), since a simplifier's islands don't. The cheaper arm to test
  first would be an authored prism per wall run, not a generated block.
- **Per-kind far meshes running out:** a far-drawn kind that can't drop surfaces without changing
  its look, while still costing a measurable share of the wide frame.
