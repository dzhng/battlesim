// The playable area's border: a line along the map rectangle's edge, draped
// on the walkable surface just inside it, drawn as an overlay (never fogged:
// drawn sight runs on past the edge, so this line is what marks it). Its
// width is given in pixels at the camera's target and turned into metres
// by the caller's zoom step, so it reads at the strategic height without
// becoming a road-wide band close in.
import { MeshBuilder, type Mesh, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";

/** `presentation.map_border`. */
export interface MapBorderStyle {
  color: Rgba;
  /** Width on screen at the camera's target. */
  width_px: number;
  /** Never narrower than this on the ground. */
  min_width_m: number;
  lift_m: number;
}

/** Drape step along the edge: the terrain grid's spacing or finer. */
const DRAPE_STEP_M = 4;

/** The width in metres that spans `style.width_px` where one pixel spans
 *  `metresPerPx` (`metresPerPxAt`, at the camera's target). */
export function borderWidthM(style: MapBorderStyle, metresPerPx: number): number {
  return Math.max(style.min_width_m, style.width_px * metresPerPx);
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
  const at = (x: number, y: number) => [x, y, z(x, y) + style.lift_m] as const;
  // Each side as a strip from its outer edge `o` inward by `width`, running
  // along `a` from 0 to `length`: (x, y) of the outer and inner points.
  const side = (length: number, point: (along: number, inward: number) => [number, number]) => {
    const steps = Math.max(1, Math.ceil(length / DRAPE_STEP_M));
    for (let k = 0; k < steps; k++) {
      const [a0, a1] = [(k / steps) * length, ((k + 1) / steps) * length];
      mesh.quad(
        at(...point(a0, 0)),
        at(...point(a1, 0)),
        at(...point(a1, width)),
        at(...point(a0, width)),
        style.color,
      );
    }
  };
  side(w, (a, i) => [a, i]); // south
  side(w, (a, i) => [a, h - i]); // north
  side(h, (a, i) => [i, a]); // west
  side(h, (a, i) => [w - i, a]); // east
  return mesh.build();
}
