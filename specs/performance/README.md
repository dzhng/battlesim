# Performance across gameplay views

**Status:** reopened; implementing three-view measurement, spatial selection and
evaluating far prop representations. **Updated:** 2026-10-20.

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
   At roughly 1–2 render pixels, evaluate a cheap card against the existing mesh;
   retain the mesh when the replacement has not earned its visual/runtime cost.
   Use projected bounds, including elongated walls
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

Implementation and whole-spec source review are complete; finish verification,
repair its failures, then archive and push. Nothing from this reopened run is
pushed. The final retained shape uses spatial terrain/static selection, local
receiver fits and per-cascade caster lists. Car cards and the unmeasured building
member refinement were removed; keep the shipped simplified car mesh.

The complete final three-view matrix passed, including candidate live, late and
moving windows. Repeated paused control/candidate medians show **no net GPU
speedup**: close 5.03/5.63 ms, tactical 5.50/5.74 ms, wide 32.64/34.41 ms.
Wide correct caster coverage adds a net 3.07 million shadow triangles and
53.47 MiB of allocated buffers versus the old approximation. Do not trade away
legitimate offscreen casters to recover the old timing. Terrain-only remote-growth
measurements demonstrate bounded selection and receiver work; they are not a
whole-frame speedup claim. [Research](research.md) records the scope and evidence.

Full Rust tests passed. The full web unit run's two fixture failures were repaired
and focused checks passed. Full browser verification completed; affected reruns
resolved the rendering/fixture failures except the ally-help stall. Do not repeat
the full suites without an invalidating change. Remaining work:

1. Diagnose the ally-help stall from the captured battle. Native and WASM now
   match the original 578-command prefix after the explicit u32 seed parity fix.
   The trace identifies static prop 13879, a nonzero rejected forward step, and
   a clear backing prediction. Recovery currently requires a zero forward step;
   that eligibility condition is under test. Earlier allied-traffic and acceleration
   diagnoses were contradicted by the trace.
2. The contact-stroke correction is committed with hooks: minimum coverage uses
   actual raster resolution while authored widths and body radius remain CSS-sized.
   Focused visual/behavior checks passed; a type-only viewport extraction keeps
   the existing oversized file shrinking. Preserve this contract in final docs.
3. Complete the matched moving-control comparison and continuous visual review.
   Final mesh resource-reset checks passed. Native replay matches all submitted
   counts across 18 plates; image review found no broad geometry/shadow defect,
   but tiny distant infantry differ between native meshes and browser cards.
   Do not claim pixel-identical parity or proven historical attribution.
4. Refresh acceptance statuses and the consolidated choices against final code,
   complete archival claim review, run hooks, and push the reviewed result.

Full known-fall snapshot reconciliation remains O(F), while immutable membership
avoids geometry/index rebuilds on learned falls. Conservative chunks and fall
envelopes can over-admit. Browser draw/receiver terrain partitions remain separate
allocations. These limitations stay explicit in the final rationale.

The filtered car experiment retained pixels in the corrected boundary captures,
but appearance and motion remained uncertain; it was not qualified. The earlier
filtered disappearance report was invalidated by a capture decoder bug. Do not
repeat it as evidence against cards generally.

Evidence and detailed continuity live in ignored `throwaway/three-view/`, with
`SESSION-HANDOFF.md` and `final-contract-reconciliation.md` as the current pickup.
Pinned paused comparisons use tick 90, time 3 and digest `60ede87ba60aab0b`.
