# Performance across gameplay views

**Status:** reopened; implementing three-view measurement, spatial selection and
far prop representations. **Updated:** 2026-10-20.

Make production gameplay faster at close, normal and wide zoom. A benchmark is
an instrument, not the objective. The earlier wide-only experiment cannot rule
out culling elsewhere. Keep the shipped Rust reuse, Low render scale and car
mesh improvement; [prior results](prior-results.md) and [research](research.md)
record their exact scope. No simulation or side-knowledge changes are allowed.

## Three zoom levels

Use one admitted Metro XL seed-4 battle with 100 units per side and one pinned
contact target. Use the actual production camera configuration and clearance;
record resolved position, target, pitch, yaw, FOV and target dimensions.

- **Close combat: 65 m orbit distance.** The configured gameplay default; detailed
  units, cover and street geometry must remain correct.
- **Normal tactical: 260 m.** The existing benchmark's normal pan scale; observe
  formations and several local blocks.
- **Wide strategic: 2,400 m.** Preserve the previous stress scale, including its
  crowd of tiny visible props. This is not the map's maximum possible zoom.

Distances are requested poses, not overrides of production camera clearance.
Use one yaw and contact target for stationary comparisons. Baseline screenshots
must prove the intended subject is present at each level; freeze the resolved
poses before A/B. Run paused attribution, live contact, and the late state
(800 fallen soldiers, 100 wrecks). Add continuous pan/orbit/zoom transitions
between these same three levels; do not mix transition samples into hold metrics.

## Work contracts

1. **Measure each view independently.** Report frame interval/FPS distribution,
   main-thread frame cost, GPU duration, worker simulation rate, actual submitted
   triangles/instances/draws per pass and cascade, spatial candidates visited,
   uploads and GPU resource counts. CPU/GPU overlap; no additive percentages
   from unrelated averages. Use fresh GPU samples after transitions, not a
   rolling window containing the preceding view. Counters must describe draws,
   not buffer capacity or installed meshes.
2. **Growing unseen world must not amplify local frame work.** Supplement the
   real battle with matched local geometry/publications/camera/light and remote
   map extensions at three extents. Preserve IDs and local triangles. Measure
   candidate traversal as well as submitted geometry; avoid replacing a GPU
   full-map draw with a CPU full-map scan. Startup and residency are separate.
3. **Camera and shadow selection have separate visibility needs.** Cull outside
   the actual camera; select casters against fitted cascade receiver needs.
   Offscreen geometry that shadows visible ground must survive. Do not restrict
   shadows to the camera-visible list or refit shadows differently merely because
   unrelated map bounds grew. Share selection through existing Rust owners and
   browser/native consumption. Terrain, props, buildings and scenery are audited.
4. **Detail follows projected footprint.** Close meshes, simplified middle
   meshes and far sprite/impostor representations form one asset-owned policy.
   At roughly 1–2 render pixels, a supported prop should use a cheap card rather
   than hundreds of triangles. Use projected bounds, including elongated walls
   and nonuniformly scaled props; height or distance alone is insufficient.
5. **Impostors are production representations.** Reuse the existing atlas/card
   owners, extending shared assets and both clients. Preserve scale, tint,
   lighting, fog, damage identity, occlusion and sensible depth. Camera-facing
   cards must not create billboard-shaped shadows or silently remove shadow
   contributions. Choose an appropriate cheap caster independently. Verify
   transitions, headings and pitch changes, not just three still screenshots.
6. **No invisible quality cuts to win measurements.** Subpixel fade is allowed
   only when the object stops contributing visibly and does not hide gameplay
   information. Do not remove objects, lower quality, change battle density or
   invent a far cutoff to make a test pass. Infinite-far/horizon views can admit
   genuinely more world; any finite visibility policy must be explicit, visually
   coherent and independently reviewed. Occlusion culling is distinct from
   frustum selection; add it only if measurements justify its cost.

## Slices and dependency graph

- [ ] [1. Measurement and baseline](slices/measurement.md): reliable windows,
  three zooms, real submission counters and matched-local-view growth proof.
- [ ] [2. View and caster bounds](slices/visibility.md), after 1: correct shared
  spatial selection, measured across all zooms and map extents.
- [ ] [3. Distant representations](slices/impostors.md), after 1 and the caster
  contract in 2: opaque car pilot, then explicit wall/vegetation decisions and
  implementation where the measured footprint/work warrants cards.
- [ ] [4. Acceptance](slices/acceptance.md), after 2–3: moving/live/late-state
  visual and performance gates, native/browser parity, reset lifetime and review.

Asset-path research can proceed alongside measurement. Keep GPU runs exclusive.
Each completed pass is reviewed and committed; it is not the stopping point.

## Acceptance and stopping rule

The user's preference for simple, clearly measurable wins remains. Apply it to
representative gameplay and scaling, not solely the wide stress mean. A culling
change may earn its cost through a demonstrated work-bound repair plus close/
normal-view gains; a wide-view near-noise result alone cannot reject it. A lower
triangle count alone cannot establish a speedup. Report each zoom's result and
regressions independently; do not average away a bad view. No universal FPS
floor or arbitrary percentage is presumed.

Completion requires production culling and far-representation behavior verified
across the matrix, with each appearance class explicitly resolved; unsupported
or visually harmful cards retain a documented mesh representation, not missing
objects. Known architectural gaps cannot disappear into a benchmark verdict.
Full closeout checks run once after implementation, with narrower checks per pass.

## Next Agent Prompt

Core terrain/static bounds, local receiver fitting, per-cascade caster lists and
shared browser/native car cards are integrated. No slice is closed and nothing
from this reopened run is pushed. Keep final acceptance separate from focused
contract tests.

Finish three independent architectural repairs in progress: bound felled trees
and stumps, select water/boundary draw ranges, and cache unchanged receiver fits
while counting their actual traversal/triangle work. The current terrain-growth
counters omit receiver fitting; do not claim complete bounded CPU work yet.
Browser draw/fit terrain partitions also duplicate immutable storage; resolve
ownership if the measured maintenance/memory benefit warrants the change.

The car depth/coverage correction is integrated as `fc485aaf4`. Its shared shader
uses a third, RG8 depth atlas and per-sample coverage. All 96 tested threshold
cases contribute pixels against empty controls retaining mesh casters. This
proves contribution, not perceptual equivalence. Motion contrast and native
individual-car parity remain open. The corrected shader regresses wide GPU cost
to about 72 ms in three rounds; diagnose and resolve this before acceptance. The candidate
adds 128 KiB GPU texture memory. Focused integration and shader-export tests pass.

Then measure the final Low production build at all three zooms: paused, live,
late and separate moving windows, with matched controls and resource resets.
Explicitly resolve walls/vegetation from footprint, cost and images. Review the
whole spec, consolidate choices, run full closeout checks once, and push only
after the acceptance gates pass.

Evidence is retained in `throwaway/three-view/`; detailed continuity is in
`SESSION-HANDOFF.md`. Pinned paused comparisons use tick 90, time 3 and digest
`60ede87ba60aab0b`. Same-world terrain full/selected images are identical; median
GPU full/selected is 5.457/5.493 ms close, 6.463/5.897 tactical and 35.390/35.169
wide. These isolate draw cost, not selector CPU. Earlier car gains used the old
caster approximation and are not final combined results. Correct per-cascade
submission reduces wide caster triangles, but its repeated GPU timings do not
yet establish a gain and allocated buffers increased about 39 MB.

The terrain-only growth harness passed 30 windows at explicit Low after its
original default-profile run. It preserves local geometry and appends flat or
elevated remote extensions. Population/water/felled growth and complete receiver
accounting remain separate gates. Existing moving/growth image reviews found no
convincing missing geometry; they precede the pending changes and do not certify
sprite motion or new draw-range selection.
