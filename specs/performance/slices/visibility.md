# 2. Bound camera and caster work

Status: shared implementation integrated; combined GPU/visual acceptance pending. Unlock spatially bounded production work
without losing visible objects or their shadows.

Shared Rust presentation owns static selection; browser and native draw its
results. Audit terrain, buildings, scenery and props. Query an immutable spatial
index rather than scanning every chunk/model on each view change. Camera batches
and cascade caster batches are separate outputs: offscreen casters remain when
they shadow visible receivers. Preserve the shared depth/camera convention.

## Passes

- [ ] Measure each owner's visited candidates and emitted geometry at all zooms.
- [x] Test spatial selection against a brute-force geometric oracle, including
  near-plane boundaries, long/scaled props, camera movement and remote extension.
- [x] Implement bounded steady selection for terrain, buildings, standing and
  felled scenery, props, water and boundary meshes, plus receiver accounting.
- [x] Remove population-sized spatial rebuilds when a new fall is learned.
  Account separately for full publication-snapshot reconciliation; stationary
  query bounds do not prove live mutation bounds.
- [x] Independently select shadow casters; test an offscreen object shadowing
  visible ground and unrelated map extension leaving cascade fit unchanged.
- [x] Reassess the prior terrain selector at all views. Scratch code is an
  experiment, not a validated winner; retain only the justified production shape.
- [ ] Integrate both clients and measure paired GPU/CPU/frame results per view.

The integrated terrain candidate submits 4/4/10 camera triangles instead of
194,458 at close/tactical/wide. Same-world alternating full/selected draws produce
pixel-identical captures: no detectable close GPU benefit, about 0.57 ms tactical
and 0.22 ms wide. Static populations now use the same hierarchy; caster lists
retain independent per-cascade membership in both clients. Combined timing and
shadow continuity still require acceptance. Infinite-far views can see added world;
matched-view assertions must hold admitted local content and visibility constant.
No new distance cutoff or occlusion policy is silently introduced here.

Index layout and range coalescing are delegated, with simplicity and allocation
reuse preferred. Geometry conservation, unchanged simulation digests, native/
browser selection parity and resource lifetime stay green. Compare frozen
before/after frames and offscreen-shadow crops using compare-screenshots; inspect
transitions for holes/popping, run screenshot-critique last, and show preview-shots.
The user can redirect the visual tradeoff while independent work continues.

Water and boundary meshes reuse the same spatial partition and range coalescing
as terrain (`3a1fd9988`). Water queries camera ranges; boundary/debug geometry
also queries independent cascades. The constant-size backdrop retains direct
draws. GPU capacities and material settings are unchanged; retained CPU vertices
cost 120 bytes per triangle plus hierarchy/range metadata. Focused actual browser
draw-routing tests pass after integration. Public traversal diagnostics are wired; rendered parity and performance
acceptance remain open.

Receiver fitting reuses identical camera/light/terrain inputs and invalidates on
replacement or world reload. Counters report executed work, including zero
traversal on reuse. With 64/1,024/16,384 off-axis remote tiles, fresh fits visit
13/21/29 bounds and test/clip two triangles; this does not bound terrain genuinely
visible at the horizon. Standing and felled trees share one immutable hierarchy
widened once for conservative fall envelopes; knowledge changes update membership
without rebuilding geometry or the hierarchy. Offscreen time still advances.
The remote-fall test performs zero additional envelope initialization and 12 local
membership checks at populations of 67/1,027/4,099. Query visits are 64/104/124;
wider bounds trade some over-selection for avoiding mutation rebuilds. Standing
geometry stays resident (1.62 MB of geometry/chunk vectors in the largest sparse
fixture, excluding index/hash/output overhead). Snapshot equality, reconciliation
and replacement still scale with known falls. The integrated static-world suite
passes 74 tests; browser diagnostics expose the owner's work counters without
claiming they include all snapshot handling.
