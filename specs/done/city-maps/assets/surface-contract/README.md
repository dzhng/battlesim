> Frozen historical evidence. Original identities and rejected arms remain scoped to their source; current completion and user-authorized deferrals are recorded in [completion evidence](../../evidence.md).

# Shared physical surfaces

Road strokes keep the original capsule distance arithmetic along their length and at their bends; since 2026-10-01 each is cut square across its first and last point (`choices.md`, "Road ends"). Polygon roads and sidewalks are one closed authored ring each in the simulation, with native triangles exported for drawing. Sidewalks keep ground movement cost. A road wins a sidewalk overlap; water and bridge rules keep precedence.

The export has three streams: stroke rows, membership triangles, and exposed boundary segments of the polygon union. Native splitting removes covered fragments and shared interior edges; coincident exterior edges keep one copy, preferring the road. Drawing classifies any membership triangle, then signs distance to the exposed union boundary once, so triangulation diagonals never become material edges. Simple rings accept either winding and collinear vertices, without holes or a repeated endpoint.

A per-polygon distance export was tried first and rejected: it left seams inside touching and overlapping polygons.

## What was proven

- Navigation build on the unchanged village fell from about 218.0 million to 215.0 million median instructions (1.38%), with the same 45,111 cells.
- Geometry, navigation, aggregate-building and frozen publication checks pass. Old frozen inputs are translated from the old road schema inside test loaders; expected observations, samples and digests are unchanged.
- Removing the closed-edge guard fails at a concave ring's edge. A second regression caught a polygon maximum on a bucket boundary across the map origin; storing actual minima and maxima fixed it, where minimum-plus-width had rounded below the boundary.
- Retaining covered fragments makes a boundary distance read 2.5 m instead of 5; the restored source passes.
- The public-wasm and GPU probe passes eleven single, overlap and touching samples with the real interior material roughness, and no validation or disposal errors. The village tour passes its original grass, camera and road-edge checks; see [the renderer proof](renderer/README.md).
- The map decoder refuses unknown fields, so a leftover `roads` key is a diagnostic instead of a silently empty surface list.

## Open

The village proves this cutover only. Arbitrary polygon counts, whole-world bucket occupancy, 20 km battles, new road speeds and round centerlines are not admitted. GPU coordinates stay f32, so arbitrary f64 queries are not claimed bitwise equal to GPU classification. Mixed stroke and polygon joins still combine the two distances; their admission belongs to [C63](../../evidence.md).

Raw evidence: tag `city-maps-evidence-2026-09-30`.
