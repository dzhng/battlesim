# 1. Independent view measurements

Status: implementing. Unlock a trustworthy baseline before choosing optimizations.

The existing frame timer owns GPU durations; reset starts a new measurement epoch
and excludes delayed readbacks from earlier frames. The viewport pilot owns camera
placement, and benchmark recording owns frame/CPU samples. Extend these owners,
not a second renderer or independent camera. Hold the README's three distances
with production clearance and record their resolved poses. Preserve unrelated
model/effect diagnostic views.

## Passes

- [ ] Test then repair timer epoch/reset behavior, including pending, delayed and
  failed readbacks. Missing timestamp support is explicitly unavailable, never zero.
- [ ] Integrate fresh measurement boundaries into benchmark/endurance phases.
  Wait for a declared number of valid samples with a deadline, reporting incomplete
  runs. Neither elapsed sleep nor drawn-frame count proves GPU sample completeness.
- [ ] Count submitted triangles, instances and draw calls at actual draw sites,
  separately for world depth/color and each shadow cascade. Distinguish installed
  capacity, selected population and visited spatial candidates.
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
