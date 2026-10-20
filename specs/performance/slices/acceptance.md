# 4. Production acceptance and closeout

Status: implementation settled; moving comparison and closeout repairs remain.

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
`VIEW_TRANSITIONS=1` option (see [measurement](measurement.md#moving-window-instrumentation)).
Run it under the parent's exclusive GPU lock on both builds; inspect each leg's
valid sample count, resolved camera path, periodic counters and separate picture
replay. Instrumentation alone does not satisfy the moving-view acceptance item.

The [matched-local browser harness](measurement.md#matched-local-browser-harness)
collects terrain-growth controls through production scene composition. Its
successful collection is not acceptance: review traversal versus resident growth,
local pictures and receiver records for both horizontal and elevated extensions.
Static population growth and offscreen caster coverage still need their own
proof; this terrain-only fixture does not close those contracts.

## Final retained resource and native checkpoint

The final mesh-only Low Metro XL lifetime run passed repeated resets and live/late
segments. Resets reproduced battle identity, clock, corpse IDs, camera and exact
resource totals: 5,982 buffers, 41 textures, 3,300,445,908 buffer bytes and
560,699,569 texture bytes. This establishes reset stability, not low residency.
Live/late simulation throughput was approximately 20 Hz, below nominal 30 Hz;
this run does not establish a real-time simulation performance budget. Evidence:
`throwaway/three-view/final-resource.log` and `final-resources/`.

Native replay matched submitted model/building/prop/scenery/effect counts on all
18 market-town plates. Independent image review found no broad missing geometry,
shadow displacement or depth inversion. Tiny distant infantry silhouettes differ;
native uses posed coarse meshes while browser uses an image atlas. This supports
close rendered agreement, not pixel identity or conclusive historical attribution
of those differences. Evidence: `final-native-plates/`, `final-native-diff/` and
`final-native-visual-review.md` in the same scratch directory.

## Matched moving evidence

The frozen production input adapter completed all nine control and candidate legs
across paused, live and late states, with at least 120 valid GPU samples per leg.
The moving measurements do not establish a general speedup. Paused close-to-tactical
GPU means were 5.53/5.23 ms control/candidate, tactical-to-wide 15.11/15.65 ms,
and wide-to-close 12.67/13.82 ms. Live main-thread means increased from
17.61/19.90/21.80 ms to 20.84/28.49/29.70 ms across those legs. Live and late
runs advance different tick counts in equal wall time, so their workloads are
not identical state-by-state and their differences are not causal speedup estimates.
CPU diagnostic jobs were also not consistently excluded from the candidate run;
its CPU/frame comparisons are observations under that load, not isolated renderer
regression measurements. A causal moving-performance claim requires a quiet repeat.

Separate recordings include deployment setup before the actual camera tour. The
first review mixed those phases and incorrectly treated striped ground as a
streaming fallback; those claims are withdrawn. A new review of all six trimmed
control/candidate tours, matched by path phase, found no candidate-only missing
geometry, model-detail, shadow-continuity or framing defect. The striped treatment
retains building silhouettes and roads in both builds; its wide-view readability
cost and abrupt boundary remain shared presentation limitations. This is bounded
visual evidence for the recorded paths, not pixel identity or a universal no-pop
claim. Reports and trimmed tours are under ignored `throwaway/three-view/moving-trimmed/`
and `throwaway/perf/native-scratch/moving-trimmed-visual-critique.md`.
