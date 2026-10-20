# 3. Projected-size prop representations

Status: car pilot rejected; retain authored distant meshes. Final combined
rendering acceptance remains in [acceptance](acceptance.md).

The car pilot did not earn its complexity. Surface-depth reconstruction repaired
the flat-card ground cut, but sample-frequency shading cost roughly 41 ms extra
GPU time. Cheaper centroid sampling erased eligible cars. A bounded filtered
atlas experiment, including coverage-weighted surface position, still lost two
eligible camera phases. More rendering machinery is not a simple proven win.

The new prop-card opt-in, prepared atlas format/bake path, native car-card
consumer, depth atlas and sample shading were removed in `8ce8d4d96`. Keep the already shipped simplified
car mesh and existing unit/corpse cards. Keep generic allocation cleanup and
appearance-cache fixes that protect those existing consumers. Independent camera
and cascade selection remains shared in Rust; removing car cards cannot remove
car shadows or alter simulation bodies.

- [x] Trial the opaque parked-car pilot against real selection and empty controls.
- [x] Reject candidates that lose visible cars or fail the GPU cost gate.
- [x] Resolve walls, vegetation and other measured heavy classes explicitly.
- [x] Remove pilot-only code/assets/tests; focused browser/native checks pass.
- [ ] Verify retained representations through final production visual/performance
  and native image acceptance.

This resolves the spec's supported/unsupported representation policy through
measured rejection, not a claim that cards cannot work. A future pilot needs a
simpler coherent reconstruction with demonstrated coverage, depth, motion and
GPU savings. Do not revive the rejected implementation merely because its
triangle count is lower.

## Rejected pilot evidence

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Flat card | Ground cut from plane depth; pixel-center alpha erased 44 of 96 cases against caster-retaining empty controls. | Reject. |
| Surface depth + sample interpolation | All 96 boundary cases contributed pixels, but repeated Low wide GPU was 72.43/72.01/72.72 ms versus fresh previous-build control 30.82/30.84/30.24. Matched camera, time, digest and triangles. | Reject fragment cost. |
| Surface depth + centroid | Wide GPU 31.57/31.76/31.51 ms. Expanded render-size-matched probe erased 7 of 306 eligible states; mesh/sample controls erased none and 804 shared controls were byte-identical. Critic found contrast jumps. | Reject coverage loss. |
| Coherent footprint filtering + surface-position moments | Expanded 370+240-state probe still lost two eligible phases; mesh controls remained visible. About 1.05 MB GPU atlas and 165 KB compressed. | Reject before timing; not integrated. |

The depth correction added one coverage-weighted channel and restored ground
contact, but that alone did not establish correct subpixel appearance. Centroid
sampling isolated the sample-shading cost; simply using the existing cutout mask
or bypassing depth did not recover its transparent atlas samples. Filtering
coverage, color, normal and depth together then exposed inconsistent surface
position. Even the bounded position-moment correction failed selected phases.
No candidate met both quality and performance, so none ships.

Narrow stationary visibility was not enough: horizontal/vertical raster phase,
nonuniform scale, high pitch and adjacent view headings exposed failures. Future
trials must preserve those cases, real automatic selection and independent mesh
casters. Nonzero pixels alone are not a contrast or motion-quality pass. Native
smoke validated shader compilation; crowded diagnostic plates never established
individual-car visual parity. Evidence remains in the card delegates' scratch
handoffs and `throwaway/three-view/depth-*`; rejected probe assets stay out of
production.

## Other appearance classes: retain existing representations

Courtyard walls retain their authored meshes. The earlier simplified far mesh
lost visible upright contrast and failed independent visual review
([research](../research.md#distant-wall-simplification-rejected)). The pinned
three-view capture submits no courtyard walls close, 501 tactical and 58,430
wide. Their high wide count warrants future asset work, but does not validate a
card: elongated bounds and wall continuity must survive. No unsupported wall
sprite or invisible cutoff is introduced in this pass.

Hedges, other cutout props and scenery retain their existing authored mesh/card
policy. The same capture selects 38/178/4,794 hedge instances across the three
views. Earlier cutout-shadow omission and equivalent-discard experiments were
near noise; neither establishes a simple measurable GPU win. New coverage or
shadow approximations lack evidence of a simple win. Existing spatial selection applies to these populations.

Planters and remaining props keep their coarse meshes. Their prior attribution
was smaller and noisier than cars; there is no validated replacement with both
an image-quality pass and a repeatable time saving. This is an explicit scope
choice under the user's simple-wins rule, not a claim that their rendering is
free or optimal. No new far representation ships. Revisit these classes only
with new per-class cost and footprint evidence. Counts above come from `cascade-paused/views-paused-1.json`; they are
camera selections, not all-pass triangle totals or final-build timings.
