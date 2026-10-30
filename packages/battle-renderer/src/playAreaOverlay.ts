// The playable area's border: a line along the map rectangle's edge, draped
// on the walkable surface just inside it, lit and fogged with other ground
// paint. Its width is given in pixels at the camera's target and turned into metres
// by the one stroke rule at the caller's zoom step (`strokeWidth.ts`), so it
// reads at the strategic height without becoming a road-wide band close in.
import { groundRing, groundStrip, MeshBuilder, type Mesh, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { StrokeWidth } from "./strokeWidth";

/** `presentation.map_border`. */
export interface MapBorderStyle {
  color: Rgba;
  /** Width on screen at the camera's target. */
  width_px: number;
  /** Never narrower than this on the ground. */
  min_width_m: number;
}

/** The border's width in metres at the camera's zoom (`stroke`). */
export function borderWidthM(style: MapBorderStyle, stroke: StrokeWidth): number {
  return Math.max(style.min_width_m, stroke(style.width_px));
}

/** The border of a `size[0]` × `size[1]` map with its origin at (0, 0),
 *  `width` metres wide, lying inside the edge. */
export function buildMapBorder(
  size: readonly [number, number],
  style: MapBorderStyle,
  width: number,
  z: SurfaceHeight,
): Mesh {
  const mesh = new MeshBuilder();
  const [w, h] = size;
  // Each side a strip lying just inside its edge.
  const i = width / 2;
  for (const [a, b] of [
    [
      [0, i],
      [w, i],
    ], // south
    [
      [0, h - i],
      [w, h - i],
    ], // north
    [
      [i, 0],
      [i, h],
    ], // west
    [
      [w - i, 0],
      [w - i, h],
    ], // east
  ] as const)
    groundStrip(mesh, a, b, width, style.color, { z, step: 4 });
  return mesh.build();
}

/** The objective zone's edge: a ring `line` wide lying just inside `radius`,
 *  dashed while the zone is not held. */
export function buildZoneRing(
  center: readonly [number, number],
  radius: number,
  line: number,
  color: Rgba,
  held: boolean,
  z: SurfaceHeight,
): Mesh {
  const mesh = new MeshBuilder();
  groundRing(mesh, center, radius - line / 2, line, color, { z, segments: 64, dashed: !held });
  return mesh.build();
}
