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

Correct static caster selection against the fitted shadow volumes first. The
integrated prop candidate preserves camera lists but its approximate height
sweep fails a translated-below-zero caster regression. The saved failing test
and log are in `throwaway/three-view/caster-height-regression.patch` and
`caster-height-test.log`. Share conservative clip math with terrain selection;
do not make local work depend on unrelated remote terrain elevation.

Timer epochs, actual submission counters and fixed-tick three-view captures are
integrated. The terrain partition candidate (`5dc40aebd`) is also integrated;
its browser/native shared selection has focused tests, but GPU/image acceptance
is still open. Public traversal stats and rebuilt WASM pass focused integration checks. The
car-card pilot and moving-window instrumentation are integrated; their GPU/image
gates remain open. Shared offline card assets and native consumption are in progress.
The terrain hierarchy is extracted into one allocation-free bounds index for
reuse by static populations; port those queries after the fitted-caster correction.

Evidence lives in `throwaway/three-view/`. `control-pinned`, `caster-candidate`
and `control-return` match tick 90, digest `60ede87ba60aab0b`, camera and render
size. Wide model shadow submissions fall from 53,139,840 to 34,821,352 triangles;
camera geometry is unchanged. Paired paused images show no visible regression.
Timing drift in the return control prevents claiming a speedup: the
`interleaved.mjs` comparison alternates control/candidate pages at each zoom.
Freezing one page during the other's renderer initialization stalled startup;
the current terrain comparison initializes both before freezing inactive pages.
The initial paused/live/late capture remains in `baseline/`; its clean views are
geometry references, with HUD crowding recorded as an unresolved limitation.

After caster correctness, validate terrain and prop candidates across all three
views, then finish moving-view and matched-local-world scaling measurements.
Bound static object traversal as well as terrain traversal. Complete shared
far assets and actual native card consumption, with explicit wall/vegetation
choices. Live/late/moving/resource/digest acceptance, whole-spec review and full
checks remain open. No slice is closed and the current candidates are not pushed.
