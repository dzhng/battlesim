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

/** `presentation.overlay.capture_curtain`: the light a held capture area
 *  gives off along its edge. */
export interface ZoneCurtainStyle {
  /** How high it rises above the ground before it fades out. */
  height_m: number;
  /** Its opacity where it meets the ground. */
  alpha: number;
}

/** How the curtain fades as it rises: (fraction of its height, fraction of
 *  its ground opacity), quick near the ground and long towards the top. */
const CURTAIN_FADE: readonly (readonly [number, number])[] = [
  [0, 1],
  [0.2, 0.5],
  [0.55, 0.16],
  [1, 0],
];

/** A held zone's curtain: a faint wall of `color` light standing on the
 *  ring's line at `radius`, fading out as it rises. Its normals point up, so
 *  the overlay's light shades it evenly all the way round. */
export function buildZoneCurtain(
  center: readonly [number, number],
  radius: number,
  color: Rgba,
  style: ZoneCurtainStyle,
  z: SurfaceHeight,
): Mesh {
  const mesh = new MeshBuilder();
  const up: [number, number, number] = [0, 0, 1];
  const segments = 64;
  const rim = Array.from({ length: segments + 1 }, (_, k) => {
    const a = (k / segments) * Math.PI * 2;
    const x = center[0] + Math.cos(a) * radius;
    const y = center[1] + Math.sin(a) * radius;
    return [x, y, z(x, y)] as const;
  });
  const shade = (f: number): Rgba => [color[0], color[1], color[2], style.alpha * f];
  const raised = (p: readonly number[], h: number): [number, number, number] => [
    p[0],
    p[1],
    p[2] + h * style.height_m,
  ];
  for (let k = 0; k < segments; k++) {
    const [a, b] = [rim[k], rim[k + 1]];
    for (let band = 0; band + 1 < CURTAIN_FADE.length; band++) {
      const [h0, f0] = CURTAIN_FADE[band];
      const [h1, f1] = CURTAIN_FADE[band + 1];
      const [a0, b0, a1, b1] = [raised(a, h0), raised(b, h0), raised(a, h1), raised(b, h1)];
      const [low, high] = [shade(f0), shade(f1)];
      mesh.vertex(a0, up, low).vertex(b0, up, low).vertex(b1, up, high);
      mesh.vertex(a0, up, low).vertex(b1, up, high).vertex(a1, up, high);
    }
  }
  return mesh.build();
}
