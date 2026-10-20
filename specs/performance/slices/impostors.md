# 3. Projected-size prop representations

Status: shared browser/native candidate integrated; depth/coverage correction integrated; acceptance open. Unlock cheap far props
in production while close geometry retains its appearance.

Scene assets owns representation data. Shared Rust presentation selects from
projected bounds; both clients consume the same identity and tier. Extend the
existing atlas/card owners instead of inventing a second sprite renderer. The candidate extends body cards with full fitted bounds, nonuniform transforms,
shared prepared assets and actual native atlas consumption. The shared depth/coverage correction restores surface depth and evaluates coverage
at MSAA sample positions. Boundary contribution checks pass; moving appearance,
native individual-car parity and GPU acceptance remain open.

## Passes

- [ ] Pilot opaque parked cars using shared, content-addressed baked impostor
  assets. Measure two-triangle cards against the existing far mesh at frozen poses.
- [x] Extend selection and card transforms for rotated/nonuniformly scaled bounds.
  Use screen footprint, not height alone; a long wall is not a tiny object.
- [ ] Preserve tint, fog, material response, damage identity and depth/occlusion.
  Test pitch/headings and continuous mesh/card transitions. Resolve required atlas
  channels from the visual pilot, keeping only channels that earn their cost.
- [x] Keep independent cheap mesh casters where required. Never substitute the
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

## Depth and coverage evidence

The integrated correction (`fc485aaf4`) retains the appearance atlas bytes and
adds one coverage-weighted RG8 depth atlas. Surface reconstruction drives depth,
lighting and fog in the shared shader. Pixel-center alpha testing erased 44 of
96 tested cards against an empty-view control retaining their mesh casters;
sample-position interpolation reduced that count to zero. Automatic selection
matches mesh above and card below the boundary in all 96 cases. Nonzero pixels
do not establish matching contrast or motion stability.

The prepared asset costs 90,570 compressed bytes and 655,376 GPU bytes, an
additional 31,109 compressed and 131,072 GPU bytes. The native smoke validates
the shared pipeline; crowded native diagnostic plates are not individual-car
parity. Final visual critique found no obvious ground cut but left transition
appearance uncertain. Root must measure the fragment cost before acceptance.
Detailed captures and reports remain in the card delegate's scratch handoff.

The first corrected production run exposes a blocking cost regression: three
paused Low wide-view rounds measure 72.43/72.01/72.72 ms GPU versus the prior
frozen cascade build's 32.98/29.73/33.79 ms. Cameras, time, digest and submitted
triangles match. Close/tactical are 4.65/5.99 ms medians. A fresh return control
and a scratch centroid-interpolation diagnostic are underway to separate
sampling cost from depth output. This candidate is **not accepted**. Cheaper
sampling must still pass the empty controls and motion gate; restoring known
disappearance is not an acceptable optimization.

The fresh previous-build control returns to 30.82/30.84/30.24 ms wide. Changing
only interpolation to centroid in a scratch build retains surface depth and
measures 31.57/31.76/31.51 ms, isolating per-sample shading as the large cost.
The migrated current-cascade probe produces 96 exact automatic mesh/card matches;
all eligible below-threshold cards contribute pixels against caster-retaining
empty controls. Seven forced-card cases above the eligibility threshold do not;
automatic selection correctly retains meshes there. Broader subpixel positions,
continuous motion and independent visual review remain required. No production
shader change is accepted from these stationary results alone.

### Expanded centroid trial: rejected

A render-size-matched probe expands to 370 states: 306 eligible automatic cards
and 64 automatic meshes, with horizontal subpixel translations and consecutive
orbit/zoom states. Seven eligible centroid states vanish against empty controls;
mesh and sample-frequency cards vanish in none. All 804 shared control images
are byte-identical. A fresh critic also finds contrast jumps. The cheaper
centroid candidate therefore does not qualify. Earlier narrower stationary
results must not be generalized to motion or other raster phases.

Feeding centroid alpha to the existing cutout sample mask does not recover a
reproduced failure; a depth-bypass diagnostic also remains empty. Diagnostic
alpha output identifies transparent atlas sampling. The next bounded experiment
adds footprint-filtered coverage coherently with color, normal and depth through
the existing card/asset owners. Keep the threshold, geometry and independent
casters. It requires renewed visual and GPU acceptance, not an assumed mip win.
Detailed evidence is in the centroid delegate's `throwaway/car-cards/handoff.md`.
