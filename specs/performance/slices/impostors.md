# 3. Projected-size prop representations

Status: pending baseline and independent caster contract. Unlock cheap far props
in production while close geometry retains its appearance.

Scene assets owns representation data. Shared Rust presentation selects from
projected bounds; both clients consume the same identity and tier. Extend the
existing atlas/card owners instead of inventing a second sprite renderer. Existing
body cards are not complete prop support: they assume unit scale, omit important
material/depth behavior and have no native atlas consumer.

## Passes

- [ ] Pilot opaque parked cars using shared, content-addressed baked impostor
  assets. Measure two-triangle cards against the existing far mesh at frozen poses.
- [ ] Extend selection and card transforms for rotated/nonuniformly scaled bounds.
  Use screen footprint, not height alone; a long wall is not a tiny object.
- [ ] Preserve tint, fog, material response, damage identity and depth/occlusion.
  Test pitch/headings and continuous mesh/card transitions. Resolve required atlas
  channels from the visual pilot, keeping only channels that earn their cost.
- [ ] Keep independent cheap mesh casters where required. Never substitute the
  camera-facing card into shadow passes or accidentally drop an existing shadow.
- [ ] Implement real native asset/card consumption and parity; a coarsest-mesh
  fallback is not the same representation or its performance proof.
- [ ] Explicitly resolve walls, vegetation/cutouts and other measured heavy prop
  classes. Implement cards where footprint/work and visuals justify them; retain
  documented meshes for unsupported or visually harmful cases, never missing props.

Thresholds, atlas resolution/view count and material channels are delegated to
measured visual trials. Preserve existing near tiers and simulation bodies.
Tests cover scale/identity/tier transitions, unsupported fallback, asset validation,
resource rebuild/failure disposal and offscreen casters. Compare silhouette,
lighting and occlusion crops against full meshes at all three zooms and along
transitions with compare-screenshots; run unprimed screenshot-critique last.
Show preview-shots for a non-blocking review window while working independently.
No card implementation is accepted from triangle counts alone: pair GPU/frame
measurements and images, reporting regressions at each view separately.
