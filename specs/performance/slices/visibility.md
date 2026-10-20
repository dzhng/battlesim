# 2. Bound camera and caster work

Status: pending measurement baseline. Unlock spatially bounded production work
without losing visible objects or their shadows.

Shared Rust presentation owns static selection; browser and native draw its
results. Audit terrain, buildings, scenery and props. Query an immutable spatial
index rather than scanning every chunk/model on each view change. Camera batches
and cascade caster batches are separate outputs: offscreen casters remain when
they shadow visible receivers. Preserve the shared depth/camera convention.

## Passes

- [ ] Measure each owner's visited candidates and emitted geometry at all zooms.
- [ ] Test spatial selection against a brute-force geometric oracle, including
  near-plane boundaries, long/scaled props, camera movement and remote extension.
- [ ] Implement bounded queries where the audit finds map-wide traversal.
- [ ] Independently select shadow casters; test an offscreen object shadowing
  visible ground and unrelated map extension leaving cascade fit unchanged.
- [ ] Reassess the prior terrain selector at all views. Scratch code is an
  experiment, not a validated winner; retain only the justified production shape.
- [ ] Integrate both clients and measure paired GPU/CPU/frame results per view.

The terrain candidate reduced wide submitted terrain from 194,458 to eight
triangles but only about 0.22 ms GPU in that view. This is limited evidence, not
permission to omit culling elsewhere. Infinite-far views can see added world;
matched-view assertions must hold admitted local content and visibility constant.
No new distance cutoff or occlusion policy is silently introduced here.

Index layout and range coalescing are delegated, with simplicity and allocation
reuse preferred. Geometry conservation, unchanged simulation digests, native/
browser selection parity and resource lifetime stay green. Compare frozen
before/after frames and offscreen-shadow crops using compare-screenshots; inspect
transitions for holes/popping, run screenshot-critique last, and show preview-shots.
The user can redirect the visual tradeoff while independent work continues.
