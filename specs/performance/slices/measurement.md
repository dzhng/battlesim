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
- [ ] Build a matched-local-view control with three remote extents: extend one
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
