# 1. Independent view measurements

Status: implementing. Unlock a trustworthy baseline before choosing optimizations.

The existing frame timer owns GPU durations; reset starts a new measurement epoch
and excludes delayed readbacks from earlier frames. The viewport pilot owns camera
placement, and benchmark recording owns frame/CPU samples. Extend these owners,
not a second renderer or independent camera. Hold the README's three distances
with production clearance and record their resolved poses. Preserve unrelated
model/effect diagnostic views.

## Passes

- [x] Test then repair timer epoch/reset behavior, including pending, delayed and
  failed readbacks. Missing timestamp support is explicitly unavailable, never zero.
- [x] Integrate fresh measurement boundaries into benchmark/endurance phases.
  Wait for a declared number of valid samples with a deadline, reporting incomplete
  runs. Neither elapsed sleep nor drawn-frame count proves GPU sample completeness.
- [x] Count submitted triangles, instances and draw calls at actual draw sites,
  separately for world depth/color and each shadow cascade. Distinguish installed
  capacity and selected population. Models include props/cards; effects, grass
  and overlays are explicitly excluded. Spatial traversal counters remain in
  the visibility pass.
- [ ] Freeze paused Metro XL seed-4 baseline at close/normal/wide, plus live/late
  states and separately recorded transitions. Reuse the production scene harness.
- [x] Build a matched-local-view control with three remote extents: extend one
  admitted world's remote geometry without regenerating its local content or IDs.
  Keep camera, light, render dimensions and local publications identical; check
  both traversed candidates and submitted work. Record residency/startup separately.

The review artifact is a per-view report and screenshots, with run identity,
quality, resolved pose, dimensions, valid GPU sample counts and distributions.
CPU frame cost, worker throughput, frame intervals and GPU duration are distinct.
Do not infer additive CPU/GPU percentages from overlapping work.

Use narrow timer/recording tests and the existing benchmark contracts. Internal
counter layout and bounded sample target are delegated; document their rationale.
Compare shots against the unchanged production view with compare-screenshots;
run unprimed screenshot-critique as the final visual gate and show preview-shots.
A bad framing requires correcting and refreezing both arms, never keeping an
empty fast view. Keep simulation digests and existing scene coverage unchanged.

## Current evidence

Fresh paused holds at simulation time 3.1 s resolve to 65/260/2400 m through
production map camera configuration. One Low 1280×800 capture (960×600 world)
measured GPU means 5.05/6.74/34.77 ms; this is an initial baseline, not a gain.
The wide draw submits 13,253,044 model/prop triangles to each shadow cascade
(53,012,176 across four), while camera color submits 13,254,206. This motivates
independent caster selection before spending effort on less expensive passes.
Evidence: `throwaway/evidence/endurance/views-paused.json`; preserve a frozen
copy before later runs overwrite it. The first failed capture is not a control.

Hold windows require at least 120 valid GPU readbacks and have a 60-second
failure deadline. This is enough to expose gross costs without assuming a frame
rate; final A/B uses repeated runs. Paused/live/late holds pass the production scene. Moving-view measurements,
matched map extensions and visual acceptance remain open. Live/late results
are recorded separately; their CPU times include active presentation and
should not be inferred from the paused attribution run.

Independent screenshot critique accepted the clean views for geometry comparison.
The HUD counterparts obscure the battlefield with army cards and overlapping
labels; wide ground overlays dominate readability. Preserve both capture sets,
keep the HUD present during timing, and use clean shots only to inspect geometry.
These existing presentation limitations are not claimed fixed by this pass.

For paired comparisons the scripted session now holds its existing warm-up
promise at exactly tick 90 until the harness releases it. Three control rounds
share digest `60ede87ba60aab0b`; GPU medians are 5.49/6.65/35.03 ms. Frozen
reports and images: `throwaway/three-view/control-pinned/`; immutable production
build: `throwaway/three-view/control-build/`. This supersedes the first capture
as the timing control; the full-state capture still proves live/late coverage.

## Moving-window instrumentation

Collection completed for both frozen builds; visual acceptance is tracked separately.

The endurance scene's opt-in `VIEW_TRANSITIONS=1` flies close→tactical→wide→close
through the existing benchmark sampler and viewport pilot. Each leg changes pan,
yaw and zoom continuously; pitch endpoints come from the production rig and every
frame still resolves production clearance. Its requested keyframes and actual
per-frame poses are retained, so a clearance detour cannot masquerade as the
intended tour. The scene header owns invocation settings.

Each fixed-duration leg starts a fresh GPU epoch on its first moving frame and
collects disjoint one-second windows, keeping the timer's rolling retention from
reducing a fast run to its tail. Pending readbacks at each reset are excluded.
The report states the total valid count and fails an incomplete sample target
rather than padding it with destination holds. It freezes before the stationary
endpoint is drawn, retaining the preceding frame's full interval even if a stall
crosses the endpoint. CPU/frame intervals, worker ticks and periodic submission/
resource snapshots remain separate from the disjoint `gpuWindows`. The recorder
reports their sample-weighted mean and worst-window p95, not a whole-leg p95;
`stats.gpu` is only the last window. Overlapping diagnostic snapshots in `samples`
must not be added together.
Reports are written before a sample-count failure. Slow or absent frames have a
bounded deadline.

Pictures replay the path separately, with HUD and clean variants, so readback
cannot distort timing. They retain camera/time brackets around each screenshot;
live replay pictures are later battle states, not exact frames from the timed
run. These are review inputs, not visual acceptance. No GPU run or screenshot
review has been performed for this instrumentation pass; baseline freezing and
moving-view acceptance above remain open.

## Matched-local control: construction gate

The focused `remote_ground_growth_preserves_actual_fitted_cascade_records` test
in `crates/presentation/tests/shadow.rs` appends offscreen triangles at three horizontal extents, both flat and elevated.
Local vertex words stay identical. Every arm installs its complete mesh into the
terrain partition and calls production `sun_shadow`; it compares all receiver and
caster record words at the three view distances. This proves fitting locality
for that controlled mesh, not the browser scaling contract.

A construction hazard remains before the browser control can be accepted:

- Resizing and recompiling the saved endurance map changes local terrain
  triangulation. A CPU probe doubling its width found 10,212 versus 10,288
  triangles touching the fixed local region (x < 2500, y < 1800), with the first
  sorted triangle already different. Preserve admitted exports and append remote
  geometry; do not call independently recompiled worlds matched.

The existing geometry lab owns saved-world exports and production viewport
composition; the endurance scene owns production timing execution, and its
view-measurement helper already records CPU, GPU and resource samples. A minimal
browser extension should reuse those boundaries, retaining the same local static
identities and publications and checking resolved camera/light/dimensions before
comparing actual `submitted` and `terrainVisitedNodes`. Residency and startup
must remain separate. No browser extension or GPU run is claimed by the CPU
construction gate. Population traversal, three-extent submission/timing results,
local screenshot preservation and visual review remain open.

## Local receiver fitting

The production fit now takes immutable terrain rather than its global box. It
queries the existing terrain hierarchy, clips each candidate triangle's bounds
(with the existing standing-object headroom) to the camera, and reduces only
those local depth intervals. Tile bounds reject branches but never become the
receiver envelope. The ray-sampling grid is gone, so thin edge and horizon
receivers are not dependent on landing under a sample. Native shares its draw
partition with presentation; WASM constructs the same Rust owner at installation.
The browser still installs separate draw and fit partitions; that duplicate
startup/residency cost is not per-frame work.

The original horizontal-growth test also covers elevated extensions and compares
all production receiver/caster words. Local elevation changes the fit; empty,
near-plane-contained, thin distant and same-tile remote inputs have focused
coverage, alongside off-camera caster retention. The prior box-fit golden vectors
are deliberately superseded by the terrain coverage contract, not re-recorded
as an unexplained baseline change.

This remains conservative triangle-box fitting, not occlusion. Its query cost
is hierarchy traversal plus triangles in intersecting tiles; horizon-visible
terrain can legitimately increase that work. Existing standing headroom, caster
margin and configured shadow reach remain. GPU timing, browser matched-growth
construction and visual acceptance are still open; these CPU tests do not close
those gates.
## Matched-local browser harness

The production `geometry-scaling` scene reuses the geometry lab's admitted saved
endurance world, `LabViewport`, and the existing view-measurement pilot/recorder.
Run `bun run --cwd web scene -- geometry-scaling` after the GPU lock is released
and this revision's WASM/assets are prepared. The registry requires a production
build and serial execution; the normal scene runner prepares native tools, so do
not run it while native compilation is prohibited. Interactive entry:
`/labs/geometry-scaling`.

One export remains resident through every arm. Appending 1, 4, then 16 translated
terrain copies grows storage and horizontal extent, separately at zero and 100 m
added elevation. Original vertex/index/surface prefixes and static identities
remain intact. The complete extended mesh is passed through the ordinary
`ScenePresentation.setShadow` receiver input; no old receiver box, culling
configuration, or alternate renderer is substituted. Root owns integration with
any changed receiver signature.

Material inputs retain the admitted local domain: expanding it would regenerate
field plots. The original skirt, visual backdrop, height grid, props, buildings, scenery, camera
configuration and clock remain fixed. Remote patches are terrain geometry only,
without their own material grids or populations. This proves terrain scaling,
not population growth or a generated enlarged world's physical queries. The
report distinguishes mesh extents from the material domain. The backdrop may surround
a separate local mesh; that composition input never controls terrain selection or
shadow fitting. Simulation ticks and
worker throughput are unavailable, explicitly, because this fixture has no battle.

The scene brackets each elevation series with local-only controls and measures
65/260/2400 m through production clearance, waiting for scenery streaming before
a fresh 120-valid-sample GPU window (60 s deadline). It saves raw CPU/frame/GPU,
submission, terrain and population traversal, resource, camera/light/dimension
and clock observations, construction/install/startup times separately, and clean
pictures with frame brackets. Evidence is `throwaway/evidence/geometry-scaling/`.
A missing GPU stream writes partial evidence and fails. Input mismatches fail;
submission equality is reported, not required, because rebuilding a hierarchy
can change selected leaf granularity. Traversal growth and timing distributions
need review against residency growth; no automatic speedup or bounded-work
verdict is inferred from successful collection.

The construction test is falsified by perturbing a local vertex, then restored
green. No GPU run, screenshot acceptance, population scaling proof, or receiver
fix is claimed here. Run compare-screenshots, unprimed screenshot-critique and
preview-shots on the produced pictures before accepting the scaling control.


### Executed terrain-growth evidence

`throwaway/evidence/geometry-scaling/scaling.json` records all 30 matched windows:
local control, 1/4/16 remote copies, then returned control, for flat and elevated
extensions at every zoom. Submitted geometry is identical in every arm. GPU
means remain roughly 4–5 ms; close/tactical CPU means remain around 0.5 ms and
wide around 5.4 ms. With 17 times the terrain storage, combined terrain camera/
cascade bounds tests grow from 163 to 225 close, 191 to 243 tactical, and 355 to
405 wide. These are work-bound evidence, not a GPU speedup claim.

Tactical and wide pictures are byte-identical. Sparse close-view ground speckles
also vary in returned local controls; independent image review finds no missing
terrain/buildings, shifted ground, or changed occlusion. It records an existing
horizontal shadow-tone seam shared across the arms. Reports and receipt-backed
crops live in `throwaway/three-view/growth-visual-review.md`. The result accepts
this terrain-growth control, not population-growth/browser or temporal parity.
Shared Rust population growth tests cover complete ordered camera/caster lists
and traversal through knowledge changes; the final report must keep these proof
scopes explicit.

## Executed receiver work

The integrated fit cache publishes actual bounds visits, triangle tests, exact
clips and reuse per frame through the existing measurement path. The Low
terrain-growth rerun passes all 30 windows with unchanged submissions; hold
frames report zero receiver work and `reused=true`. At 16 remote copies, wide
main-thread means are 0.34–0.64 ms, versus 4.76–4.90 ms before caching in this
fixture. This is fixed-camera evidence, not moving-view or Metro XL timing.
Fresh-fit invalidation/growth is covered separately by Rust tests.

Public frame statistics now include terrain, water and boundary traversal.
Before/after tactical and wide growth images are pixel-identical; close images
retain the sparse variation seen in returning controls. Evidence is preserved
in `throwaway/three-view/growth-receiver-water/`. The fixture extends terrain,
not static populations or water; their structural growth proofs remain distinct.
