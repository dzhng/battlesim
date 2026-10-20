# 4. Production acceptance and closeout

Status: pending visibility and distant representations.

- [ ] Run interleaved matched A/B at all three frozen views, with fresh GPU
  windows, actual submission/traversal counters and CPU/frame distributions.
- [ ] Verify live contact, late state and continuous pan/orbit/zoom transitions.
  Keep their distributions separate from paused attribution.
- [ ] Verify matched-local-view scaling at all three remote extents, including
  offscreen casters and unchanged local content/cascade fit.
- [ ] Verify browser/native representation and selection parity, unchanged battle
  digest, resource stability across repeated reset/rebuild and no abandoned assets.
- [ ] Review every appearance class decision and every README contract; do not
  close an architectural gap merely because the wide mean is acceptable.
- [ ] Run whole-spec review, consolidate the choices ledger against shipped code,
  then run full repository closeout checks once. Fix unrelated failures separately.
- [ ] Commit/push the reviewed passes with hooks, archive with close-spec only
  after all contracts are resolved, and report actual production gains and cost.

Use compare-screenshots for frozen before/after views and targeted shadow/depth
crops. Run unprimed screenshot-critique as the last visual gate, then preview-shots
for the user. No universal FPS threshold or average across zooms replaces the
user's simple, clearly measurable wins rule. Report uncertainty and regressions
alongside gains; absence of a detectable gain is not a measured speedup.

The final report includes per-zoom baseline/candidate milliseconds, sample scope,
CPU/GPU interpretation, remaining limits, the consolidated decision ledger and
production/test/documentation change sizes. Internal report formatting is delegated;
acceptance criteria and battle density are not.

Moving-view instrumentation is available through the endurance scene's
`VIEW_TRANSITIONS=1` option (see [measurement](measurement.md#moving-window-instrumentation-execution-pending)).
Run it under the parent's exclusive GPU lock on both builds; inspect each leg's
valid sample count, resolved camera path, periodic counters and separate picture
replay. Instrumentation alone does not satisfy the moving-view acceptance item.
