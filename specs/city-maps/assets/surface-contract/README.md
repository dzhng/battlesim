# Shared physical surfaces

Road strokes retain the original capsule distance arithmetic. Polygon roads and
sidewalks use one closed authored ring in the simulation, with native triangles
exported for drawing. Sidewalks retain ground movement cost. A road wins a sidewalk
overlap; the world's existing water and bridge rules retain precedence.

The final export has three streams: stroke rows with six floats, membership
triangles with seven, and exposed polygon-union boundary segments with five.
Native splitting removes covered fragments and shared interior edges. Coincident
exterior edges retain one copy, preferring the road tag. Drawing classifies any
membership triangle, then signs distance to the exposed union boundary once.
Triangulation diagonals never become material edges. Simple rings accept either
winding and collinear intermediate vertices, without holes or a repeated endpoint.

The earlier grouped per-polygon distance export is rejected. It left internal
seams at touching and overlapping polygons; retained earlier receipts identify
that source and cannot establish acceptance of the final export.

## Native evidence

[`native/identity.json`](native/identity.json) pins the compiler, original commit,
input and pre-union native source identities. [`native/files.sha256.json`](native/files.sha256.json)
pins the retained probe, inputs and raw reports. The probe calls public
`WorldGeometry::new` and measures `NavGrid::build` with the process instruction
counter used by the existing simulation reports. It is evidence, with no runtime
test hook or shipping example.

The pre-union indexed representation used three separate processes per arm on the
unchanged village and produced median build
instructions of 217,999,027 before and 215,114,408 after, a 1.323% reduction.
Both arms stored 45,111 cells. HashMap capacity varies between processes; the
report retains its ranges instead of claiming identical allocations.

The native geometry, navigation, aggregate-building and frozen complete
publication checks pass. Frozen inputs are explicitly translated from the old
road schema inside test loaders; expected observations, physical samples and
digests remain unchanged. A removed closed-edge guard fails at the concave ring's
edge, then the restored implementation passes. A second regression finds a
polygon's exact maximum on the next bucket boundary after an extent crossing the
map origin. Keeping actual minima/maxima fixes it: reconstructing a maximum from
minimum plus width had rounded below the boundary.

`bucket-edge-green.log` records a failed intermediate test that also expected a
zero-width stroke to include its endpoint. The original stroke arithmetic already
rounds that endpoint away by a tiny distance. That assertion was removed, and the
polygon-only regression passes; the stroke rule was preserved. The configured
independent CLI review model was unavailable, so its log is not a review verdict.
Root source review and a navigation agent's independent source review found no
remaining native finding.

## Admission

The village proof covers this representation cutover. It does not admit arbitrary
polygon cardinality, whole-world polygon bucket occupancy, 20 km playable battles with bounded surroundings,
new road speeds or the later round-centerline rule. GPU coordinates remain f32;
this receipt does not claim bitwise equality between arbitrary f64 world queries
and GPU boundary classification. The bounded field and full extent proofs remain
owned by their later slices.

The final native regressions remove shared edges and clip partially covered edges.
The overlap mutant deliberately retains covered fragments: the measured boundary
distance becomes 2.5 metres instead of 5, and restoration passes. The combined
public-WASM/GPU probe passes eleven single/overlap/touching samples and confirms
the real deep-interior material roughness, with no validation or disposal errors.
The unchanged village tour passes the original grass/camera/road-edge checks.
The final union build measures 214,991,487 median nav-build instructions (1.380% below the same baseline), with 45,111 cells.
[Its identity](final-union/identity.json) and falsification logs pin that final source.
[Renderer receipts](renderer/README.md) retain the final public API/material
checks, paired images and review. These are retained separately
from the rejected per-polygon export evidence.

Mixed stroke/polygon joins still combine the original stroke distance with polygon
union distance. Complete mixed-join admission belongs to C63; this slice does not
claim the complete surface distance field or city-scale frame performance.
