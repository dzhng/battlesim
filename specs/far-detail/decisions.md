# Decisions ledger

What was decided for [far detail](README.md), by whom, and why. Add to it
whenever a slice settles something the plan left silent or confirms a fact the
plan only read from the code.

## From the request, 2026-11-15

The user asked whether the technique in a post about
[three-nanite](https://github.com/apekshik/three-nanite) could apply to our
renderer, then asked for a plan: ship the far tier for props, and spike the
other two ideas.

| # | Question | Decision and why | Closed by |
| --- | --- | --- | --- |
| 1 | Port the cluster hierarchy as it stands? | No. It gives one huge mesh detail near and coarseness far. Our cost is a hundred thousand meshes of 40 to 300 triangles, each smaller than one of its 128-triangle clusters. The part of Nanite that handles many triangles per pixel is a software rasteriser the port lacks and WebGPU cannot host. | the measurements in [performance research](../done/performance/research.md) |
| 2 | What ships? | A coarser far mesh for small, high-count props. | user |
| 3 | What is only tested? | Merging a block's props into one mesh, and cluster detail for terrain. Each ends in a verdict, not code. | user |
| 4 | Backward compatibility, migrations, a setting? | None. Hard cutover, on for everyone. | the repository's own rules |

## From the synthesis of three drafts

Three independent drafts (fewest slices, risk first, seam quality; two Claude,
one Codex) agreed on these without seeing each other: an optional fifth mesh
on static props only; the card's tier number moves to make room; a prop
without a far mesh stays on tier 3; one validated threshold in data; never
re-cut tier 3; and a ceiling measurement before either spike builds anything.

Where they split:

| # | Question | Decision and why | Rejected |
| --- | --- | --- | --- |
| 5 | Raise the tier count to five everywhere? | No. Every unit, kit, tree and grass source would need a fifth tier and every runtime file would rebake, for a need a few prop kinds have. | global `TIER_COUNT = 5` |
| 6 | Re-cut tier 3 instead of adding a tier? | No. Tier 3 is what the tactical view draws at twenty pixels, and a coarser wall there was already rejected on sight. It is used once, on purpose, as the scratch probe in slice 00. | recut tier 3; retune `lod_px` |
| 7 | Bump the bundle format version? | No. A bundle already records each state's mesh count, and the Rust decoder reads any count. Widening what the validator allows is not a layout change, so no file without a far mesh changes. | a version bump and a full re-encode |
| 8 | What does a tier-3 prop cast with? | Tier 3, as today. Only a prop drawn with its far mesh casts with it. In the wide view every small prop is far-drawn, so this keeps the saving there and changes no shadow at tactical range, which makes the first comparison a single variable. | "one coarser among what it carries" (tier 3 casts far): kept as a named, separately measured arm in slice 03 |
| 9 | What does a prop outside the view cast with? | Tier 3, as today. Few props are outside the view at the wide hold, and a coarse shadow near a close camera would show. | far mesh for off-view casters |
| 10 | Which size decides the far tier? | The one every tier already uses. One measure keeps one rule. The plan first called it the model's height; slice 02 found it is the largest of the box's three extents (`world::bounds_size`), so a long, low prop is already judged by its length. | a second, separate measure for the far band |
| 11 | The card's new number? | 5, next to the far mesh's 4. Every reader is Rust and uses the constant. | 15 (the top of the four-bit tier key) |
| 12 | How does `view.rs` learn what an appearance carries? | One value (nothing, far mesh, or card) replacing the `impostor` flag, so "card and far mesh" cannot be written. | a second flag; a coarsest-tier number |
| 13 | Where is the threshold? | Its own field, `presentation.models.far_px`. | a fourth `lod_px` entry, which tangles with the card's order |
| 14 | Must a kind declare that it carries a far mesh? | No. The bundle's bytes say so. Most kinds have no rule of their own to hang a declaration on. | a required per-kind field |
| 15 | Which count arrays widen? | The props' only. Buildings and scenery never carry a far mesh. | widening every per-tier array |
| 16 | A throwaway probe before the plumbing? | Yes, with the census, as slice 00. The last proof of the mechanism predates the tiny-prop shadow skip. | starting with the format change |
| 17 | May the plumbing be written before the probe's verdict? | Yes, in its own checkouts, since the measurement holds the GPU and not the keyboard. It does not reach `staging` before a go. | strictly serial |
| 18 | One slice per kind? | Cars alone first, as the measured win. Then one slice for the rest, with a verdict per kind. Walls have a failure on record and are judged on their own picture. | one production slice for everything; a slice per kind |
| 19 | Hysteresis at the threshold? | None. No existing tier has it. If the zoom path shows chatter, that is a new slice. | planning it up front |
| 20 | A view farther than 2,400 m as an acceptance view? | Not a gate. Slice 00 settled reach: the fixture's `zoom_max` of 2000 is a base that each battle widens to one and a half times the map's span (19,500 m here), so all three holds are reachable. Slice 03 adds a whole-map hold as a diagnostic. | the distant-impostors plan's "farther view" as a required gate |
| 21 | Hand-made far meshes or generated? | Hand-made, in the Blender sources, like the existing tiers. Automatic simplification may drop the colour bands that sank the wall. If slice 05 finds generated output acceptable, a follow-up can cover the long tail. | simplifying at bake or load |
| 22 | The spikes as one slice or two? | Two. They share slice 00's session but stop at different rungs, and terrain can run early while block merging must wait for the shipped far mesh. | one merged spike |
| 23 | A reference reproduction before each spike? | Inside each spike, and only after its ceiling clears. Reproducing the builder before knowing there is a prize is wasted. | a separate replication slice up front |

**The acceptance numbers** (two of three pairs, 5%) are the agent's picks,
borrowed from the [distant-impostors](../distant-impostors/README.md) plan.
They are loose tripwires. The real bar is a wide-view win outside the
session's own noise and no defect a fresh critic can find.

## Facts confirmed from the code

| Fact | Where |
| --- | --- |
| The card tier is read only through the Rust constant | `view.rs`, `presentation.rs`, `posing.rs`, `lab.rs`, `renderer-core/src/models.rs` |
| The Rust decoder reads however many meshes a state has | `crates/renderer-core/src/bundle.rs`, `tiers` |
| The renderer's per-tier diagnostics are four-wide and would overflow on tier 4 | `crates/renderer-core/src/models/diagnostics.rs` |
| The native count reader clamps a tier to 3 | `native/client/src/counts.rs` |
| `presentation.models` is checked only for finite numbers today | `crates/presentation/src/settings/validate.rs` |
| Only parked cars budget their tiers; walls and planters use the plain `prop` rule | `packages/scene-assets/src/scenery.ts` |
| The reference port's head is `b730aac13ab0ea67b75214613eea56c14151f6e5` | `git ls-remote`, 2026-11-15 |

## From slice 01, 2026-11-15

Decided by the implementer where the plan was silent; audited by the reviewer
against the merged code.

| # | Question | Decision and why | Verdict |
| --- | --- | --- | --- |
| 24 | `validate.ts` was already over the file-size limit and could not grow. How to make room? | The vehicle fit and the soldier fit moved to their own files, beside the existing box, tree and wreck fits, in a commit of their own before the change. The reviewer compared the removed lines with the new files: a pure move. | sound |
| 25 | New finding codes for the far rules? | None. Each far rule is an existing rule's meaning: a wrong set of tiers, a tier not coarser than the one before, a tier over budget. | sound |
| 26 | Where do the far rules live? | `packages/scene-assets/src/farMesh.ts`, called from the validator. | sound |
| 27 | Does a wreck's tier-ratio rule cover its far mesh? | No. The far mesh has one triangle rule: at least one, fewer than tier 3. | sound |
| 28 | Does the review sheet need a new layout? | No. The sheet, the workbench's tier picker and `asset validate` already list every mesh a state has, so a fifth number appears on its own. | sound |
| 29 | What do the Blender helpers use for a far part's segment count and paint edge? | A fifth entry in each table (0.12 and 6 m), so `lods=(…, FAR)` works with every primitive. No source uses them yet. | provisional: slice 03 revisits them when the car's far mesh is authored |

Confirmed in slice 01: skinned, articulated and kit sources already refused a
`_LOD4` mesh before this work; the new refusal is for trees, dressing and
grass. The whole catalog, baked with the new code, matches every one of the
940 committed runtime files.

## From slice 02, 2026-11-15

| # | Question | Decision and why | Verdict |
| --- | --- | --- | --- |
| 30 | What does presentation receive about a prop's art? | The raw mesh count of its first state (`Art.meshes`), from the decoded bytes. One Rust function turns a count into "carries a far mesh", so no client decides it. | sound |
| 31 | Where does a prop remember what it carries? | On each placed model, set from its candidate. A parallel per-look list would have to be kept in step when looks are refitted. | sound |
| 32 | What does the model lab hold a model at? | Its card when it shows one; otherwise what its mesh count says. A forced tier clamps to the meshes the art has. | sound |
| 33 | The first `far_px`? | 4. Inert until art carries a far mesh. | provisional: slice 03 sets the shipped value from the zoom crops |
| 34 | How was "the game does not change" proved? | A fresh browser count run from the branch, replayed through the base commit's presentation with each prop row cut back to four numbers: equal on all 699 counted frames, and the far slot sums to zero. Chosen over a two-build GPU session while slice 00 held the GPU. | sound for counts; the pixel check against the frame baselines is separate |
| 35 | Which per-tier arrays widened? | The props' counts, as planned, and also the renderer's two non-building diagnostic arrays, which a prop's tier indexes. No reader checks their length. | sound |

Found in slice 02, and older than this work: the native client's committed
count run had been stale on `staging` since a publication-layout change, so
its `counts` test already failed there. The one new recording fixes that and
carries the five-number shape.

## From slice 00, 2026-11-15

| # | Question | Decision and why | Verdict |
| --- | --- | --- | --- |
| 36 | Go or no-go? | Go. The candidate won all three wide pairs by about 11.5 ms (34.6 to 23.0 ms), against an A-against-A spread of 0.03 ms or less. Counts were checked before timing: equal instances and exactly the expected triangle drop, in every batch. Three batches with load spikes were rerun once; the numbers agree within 0.25 ms. | sound |
| 37 | The frozen receipt's digest no longer reproduces. What is measured against? | A fresh control at today's state (`assets/control.json`), same scene, settings and camera rule. The map generator moved three versions since the receipt. | sound |
| 38 | The close view missed the 5% tripwire pair by pair (+7% and +11%; another pair fell 6%). Retune it? | No. Recorded as missed. Pooled over every hold the arms are equal (6.53 and 6.55 ms), and three holds per batch spread by up to 1.2 ms there. Slice 03 holds close for longer. | sound |
| 39 | Which kinds after cars? | Courtyard walls only. Cars are 43% and walls 38% of the triangles props submit at the wide hold; the next kind is 3.7%. | sound |
| 40 | Should small props outside the view cast with their far mesh? | Not in the first comparison (decision 9 stands). It becomes a named arm in slice 03: such a prop casts far only when it is small enough that it would draw far, so a shadow near a close camera cannot coarsen. Slice 00 counted these at about 8 of the 50 million triangles removed. | open: slice 03 measures it |

Facts slice 00 established:

- Shadow casts are most of the cost. Cars and walls stand about a pixel long
  at the wide hold, yet nearly every one casts into three or four cascades.
- The tiny-prop shadow skip (`shadow_caster_px`) skips no visible car there
  and 2,740 of 51,990 walls, because it judges a conservative horizontal
  footprint. Its value is outside this plan.
- After props, buildings and scenery each submit about 9.5 million cascade
  triangles at the wide hold. They become the larger share once the far mesh
  ships.
- Terrain submits almost nothing on the flat city (10 view triangles).

## From slice 06, 2026-11-15

| # | Question | Decision and why | Verdict |
| --- | --- | --- | --- |
| 41 | Is terrain geometry worth a cluster hierarchy? | No. On the map with the most terrain a player can get (mixed XL, seed 12), terrain is at most 2.6% of a frame's submitted triangles, and 1.3% at the whole-map view. Generated maps have no hills: the only height is one river at most and swamps, carved down to about 10 m below flat land. The spike stopped at its first rung. | sound |
| 42 | The slice's rule said no-go needs a small share *and* triangles a pixel or larger. Carved ground is finer than a pixel (about a third of one at the wide hold). Run the timing rung anyway? | No. The rule's second half was the agent's guess at how to tell there is no prize. The share settles it: a terrain that cost nothing would remove a fortieth of the frame's triangles. | sound |
| 43 | Which map counts as hilliest? | The one with the most terrain triangles among 72 generated XL maps, not the deepest, since triangle count is what the GPU sees. | sound |

Reopen the terrain question only if the generator gains real relief.

A fact slice 06 turned up, outside this plan: at the whole-map view, which the
player can reach, a frame submits 124 to 154 million triangles, and scenery
(trees and dressing) is 87 to 98 million of them.

## From slice 02's second pass, 2026-11-15

| # | Question | Decision and why | Verdict |
| --- | --- | --- | --- |
| 44 | How do the two languages' far index stay honest? | One committed bundle, the bake of a synthetic far prop (`crates/renderer-core/tests/fixtures/far_prop.bundle`, 14 KB, a plain file outside LFS and outside anything served). A TypeScript test requires the bake to produce exactly those bytes; a Rust test decodes them, reads five meshes as presentation does, and draws the far tier. The TypeScript test rewrites the file when run with `UPDATE_FAR_BUNDLE=1`. | sound |
| 45 | The workbench holds one model. Build a three-model page for the picture? | No new product code. The three models are shot one after another under one camera rule. | sound |
| 46 | A prop batch naming a tier past its five slots: an error or a panic? | A panic that names the prop and the tier. It can only mean presentation broke its own rule. | sound |

Found and fixed in that pass, older than this work: `asset sheet` failed on
`staging` with a silent 60-second timeout, for two separate reasons. It never
built the validator the workbench needs, which is fixed. And when art is
still an unfetched LFS placeholder the page cannot load; the tool now quotes
the page's own error, which names the file.

## From slice 03, 2026-11-15

| # | Question | Decision and why | Verdict |
| --- | --- | --- | --- |
| 47 | `far_px`? | 4. At 6, 32 cars at the tactical hold drew far and street rows at 550 m lost detail. At 4 the close and tactical frames are pixel-identical to `staging`. | sound |
| 48 | Build the "tier-3 cars cast far" arm? | No. At the wide hold every car in view is already far-drawn, so it can save nothing there; it would only change shadows at tactical range, where a few thousand props are drawn. | reviewer, on slice 03's counts |
| 49 | A native capture of the wide hold? | None. The native client replays only saved parity reels, and the committed reel never draws a car far. Native's far path rests on the shared renderer's GPU test and the count reader's unit test. A later pass checks whether the reel can take one far-camera frame cheaply. | accepted gap |
| 50 | Does a far part's name matter? | Yes. The exporter orders materials by first use in object-name order, so a far part that introduces a material earlier than the near tiers do renumbers tiers 0 to 3. The car's far parts reuse the existing window objects. Walls must check the same. | sound |
| 51 | The helper's fifth segment and paint-edge entries (decision 29)? | They stand for the car: its far parts use no cylinders and no edge is long enough to split. Walls check again. | sound for cars |
| 52 | Accept the car's far mesh on its numbers? | Not yet. The timing and counts pass (wide 34.60 to 29.51 ms, three of three pairs, an A-against-A spread of 0.00 ms; 1,008 count checks). An unprimed critic found light cars darker once far-drawn and a pale patch in the candidate's wide frame; the reviewer re-measured both. Sent back with that evidence. | open |

A fact slice 03 established: a model stays in the view list while its bounding
sphere plus 3 m touches the frustum, so at the measured sun every car whose
shadow can reach the frame is still drawn at its own tier.

## From slice 05, 2026-11-16

| # | Question | Decision and why | Verdict |
| --- | --- | --- | --- |
| 53 | Is merging a block's far props into one simplified mesh worth building? | No. Run through the pinned reference builder, separate props join in at most 0.4% of the triangles, even in a run of touching walls. Below about a tenth of a pixel of error nothing is saved; from 0.4 px on the saving is whole props deleted (at 1 px a block keeps 15 of 417), which breaks the rule that a visible object never vanishes. In between, each prop is simplified on its own, which is what a per-kind far mesh already does. | sound |
| 54 | Can a merged mesh keep the look? | Only unsimplified. Fog judges each pixel by its surface's position and facing, and paint is per instance. Once simplified, 12 to 25% of the area lies in planes its prop never had, the change slice 03 showed is visible. | sound |
| 55 | Start the spike before walls shipped, and skip its GPU ceiling? | Yes to both. The fog rule and the offline builder run each settle it alone, so no GPU session was spent. Cars were measured at tier 3 because their far mesh was not merged yet. | reviewer |

What would reopen it: a fog rule that judges far props whole, accepted for its
own look. Even then the first thing to test would be one hand-made prism per
wall run, not a generated block.

Not reached, because the first two rungs settled it: how many blocks would
stay merged late in a battle, merged shadow casters, and a generated mesh's
identity and lifetime.

