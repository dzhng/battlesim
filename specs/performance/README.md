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

Continue slice 1. The timer-epoch fix and draw-counter pass are delegated in
isolated worktrees; integrate their reviewed commits before GPU measurements. Existing timing keeps 240 GPU samples: the endurance
1.5-second omission arms and benchmark two-second rolling reads are not reliable
phase boundaries at stress frame rates. Reuse the viewport pilot and recording
owners for resolved camera/frame CPU; add a fresh-sample timing boundary and
actual submission counters. Capture all three views before choosing culling or
impostor thresholds. Existing prop selection disables impostors and ignores the
shadow argument; native currently substitutes meshes for body impostors. Those
are implementation seams, not evidence that static prop cards already work.
Update this handoff after every committed pass.
