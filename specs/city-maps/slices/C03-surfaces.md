# C03: surfaces

**Depends on:** C02. **Kind:** slice.

## Question
Is a road the same surface for movement and for drawing, at city scale?

## Contract it unlocks
```rust
MapDefinition.surfaces: Vec<SurfaceArea {
  kind: Road | Sidewalk,
  shape: Polygon { ring } | Stroke { points, width_m },
}>
```
- This replaces `roads` on **every** map. The village's polylines become `Stroke`s under today's exact distance rule, so its digest doesn't move here (S-surf). C65's round centerlines later move it, as a named change.
- `world::SurfaceIndex::at(x, y)` is a bucket index over exact shapes, replacing the linear scan (`world/mod.rs:217-231`). Nav classifies through it (`navigation.rs:190-205`).
- A sidewalk moves as ground until a named rule says otherwise.
- The export carries polygon membership triangles, exposed polygon-union boundary segments and strokes. The renderer's terrain reads them; it never re-derives the rule.

## API seam
`contract::map`, `sim::world`, `sim::navigation`, `world/export.rs`, `terrain/terrainSurface.ts`.

## What the human can run or see
A nav-classification PNG of S1's generated roads crossing the urban/plain boundary.

## Verification
- Village parity (0 battles).
- Native tests: nav reads polygons; a cell centre on a sidewalk edge is classified exactly; overlap precedence.
- Paired `NavGrid::build` instructions: equal or lower on the village.
- Village terrain frame compared before and after with compare-screenshots (must be unchanged).

## Delegated to the implementer
Index bucket size. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village terrain look and digests.

## Feedback that would change this slice
A road/sidewalk boundary that players cannot predict reopens the shared surface shape, not a renderer-only mask.

## Systems receipt

The shared map schema and all native/browser consumers use this cutover. Existing
village strokes preserve their physical samples, complete observations and digests.
Focused native tests cover simple rings, closed bucket boundaries, sidewalk/road
precedence and polygon union boundaries. The public-WASM/GPU probe covers single,
touching and overlapping polygons, including the actual material's internal seam.
The original village terrain tour remains green. See
[the retained receipt](../assets/surface-contract/README.md).

Generated urban/plain classification and the full bounded field at required extents
remain later generator/C63 admission. Mixed stroke/polygon joins have not acquired
true union distance; this receipt does not admit that capability.
