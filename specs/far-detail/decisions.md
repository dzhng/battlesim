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
| 10 | Which size decides the far tier? | The projected height every tier already uses. One measure keeps one rule. | a conservative whole-bound projection for the far band: adopted only if a long, low kind fails where cars pass |
| 11 | The card's new number? | 5, next to the far mesh's 4. Every reader is Rust and uses the constant. | 15 (the top of the four-bit tier key) |
| 12 | How does `view.rs` learn what an appearance carries? | One value (nothing, far mesh, or card) replacing the `impostor` flag, so "card and far mesh" cannot be written. | a second flag; a coarsest-tier number |
| 13 | Where is the threshold? | Its own field, `presentation.models.far_px`. | a fourth `lod_px` entry, which tangles with the card's order |
| 14 | Must a kind declare that it carries a far mesh? | No. The bundle's bytes say so. Most kinds have no rule of their own to hang a declaration on. | a required per-kind field |
| 15 | Which count arrays widen? | The props' only. Buildings and scenery never carry a far mesh. | widening every per-tier array |
| 16 | A throwaway probe before the plumbing? | Yes, with the census, as slice 00. The last proof of the mechanism predates the tiny-prop shadow skip. | starting with the format change |
| 17 | May the plumbing be written before the probe's verdict? | Yes, in its own checkouts, since the measurement holds the GPU and not the keyboard. It does not reach `staging` before a go. | strictly serial |
| 18 | One slice per kind? | Cars alone first, as the measured win. Then one slice for the rest, with a verdict per kind. Walls have a failure on record and are judged on their own picture. | one production slice for everything; a slice per kind |
| 19 | Hysteresis at the threshold? | None. No existing tier has it. If the zoom path shows chatter, that is a new slice. | planning it up front |
| 20 | A view farther than 2,400 m as an acceptance view? | Only if the player can reach it. The camera's `zoom_max` in the fixture reads 2000; slice 00 records what that means for the 2,400 m hold. | the distant-impostors plan's "farther view" as a required gate |
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
