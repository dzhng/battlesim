# 3. Projected-size prop representations

Status: car pilot rejected; retain authored distant meshes. Final combined
rendering acceptance remains in [acceptance](acceptance.md).

The car pilot did not earn its complexity. Surface-depth reconstruction repaired
the flat-card ground cut, but sample-frequency shading cost roughly 41 ms extra
GPU time. Cheaper centroid sampling erased eligible cars. A bounded filtered
atlas experiment, including coverage-weighted surface position, still lost two
eligible camera phases. More rendering machinery is not a simple proven win.

Remove the new prop-card opt-in, prepared atlas format/bake path, native car-card
consumer, depth atlas and sample shading. Keep the already shipped simplified
car mesh and existing unit/corpse cards. Keep generic allocation cleanup and
appearance-cache fixes that protect those existing consumers. Independent camera
and cascade selection remains shared in Rust; removing car cards cannot remove
car shadows or alter simulation bodies.

- [x] Trial the opaque parked-car pilot against real selection and empty controls.
- [x] Reject candidates that lose visible cars or fail the GPU cost gate.
- [x] Resolve walls, vegetation and other measured heavy classes explicitly.
- [ ] Remove pilot-only code/assets/tests and verify existing representations in
  both clients, including production visual and performance acceptance.

This resolves the spec's supported/unsupported representation policy through
measured rejection, not a claim that cards cannot work. A future pilot needs a
simpler coherent reconstruction with demonstrated coverage, depth, motion and
GPU savings. Do not revive the rejected implementation merely because its
triangle count is lower.

## Rejected pilot evidence

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
appearance uncertain. The subsequent measurements below reject its fragment cost.
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
shadow approximations are unjustified while the opaque-car pilot itself remains
unaccepted. Existing spatial selection applies to these populations.

Planters and remaining props keep their coarse meshes. Their prior attribution
was smaller and noisier than cars; there is no validated replacement with both
an image-quality pass and a repeatable time saving. This is an explicit scope
choice under the user's simple-wins rule, not a claim that their rendering is
free or optimal. The car pilot remains the only new far representation under
acceptance. Revisit other classes only with new per-class cost and footprint
evidence. Counts above come from `cascade-paused/views-paused-1.json`; they are
camera selections, not all-pass triangle totals or final-build timings.


### Filtered footprint trial: rejected

Filtering coverage, color, normals and depth together removed most disappearing
states. Adding coverage-weighted position within the card plane repaired one
reproduced depth failure, but two eligible phases still failed in the expanded
370+240-state set. Mesh controls remained visible. The candidate also grew its
atlas to about 1.05 MB GPU and 165 KB compressed. It was not integrated or timed:
visual correctness failed first. Preserve its scratch evidence for future work,
not its unaccepted format and shaders in production.
