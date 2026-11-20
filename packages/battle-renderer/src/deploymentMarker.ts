// A reinforcement lane beacon. It answers one question, "which lane is
// this?" with a lane number and a short trail of matching beacons. Pulled
// far out, where those shrink to a few pixels, a soft glow sized on screen
// keeps the road in standing out from the fields round it.
import { remapClamp, type Vec2 } from "math";
import { triangulatePolygon2 } from "math/geometry";
import { triangle2 } from "math/shapes";
import {
  concatMeshes,
  groundAnnulus,
  MeshBuilder,
  VERTEX_FLOATS,
  type Mesh,
  type Rgba,
  unpulsed,
} from "./mesh";
import type { OrderStyle, SurfaceHeight } from "./orderOverlay";
import { along, orderPen } from "./orderPen";
import type { StrokeWidth } from "./strokeWidth";

type P2 = readonly [number, number];
type Rgb = readonly [number, number, number];

export interface EntryMarker {
  center: P2;
  facing: number;
  lane: number;
}

// Barlow Condensed 500 outlines, baked from web/src/fonts so the ground
// renderer can fill the glyphs without a screen-space text layer.
const FONT_DIGITS: Readonly<Record<number, readonly P2[]>> = {
  1: [
    [0.018, 1.0],
    [0.132, 1.0],
    [0.143, 0.996],
    [0.146, 0.986],
    [0.146, 0.014],
    [0.143, 0.004],
    [0.132, 0.0],
    [0.024, 0.0],
    [0.013, 0.004],
    [0.009, 0.014],
    [0.009, 0.85],
    [0.008, 0.853],
    [0.005, 0.857],
    [0.005, 0.857],
    [0.004, 0.856],
    [-0.124, 0.823],
    [-0.129, 0.821],
    [-0.136, 0.825],
    [-0.141, 0.834],
    [-0.146, 0.914],
    [-0.144, 0.925],
    [-0.136, 0.931],
    [0.001, 0.996],
    [0.009, 0.999],
  ],
  2: [
    [-0.085, 0.117],
    [0.239, 0.117],
    [0.249, 0.114],
    [0.253, 0.103],
    [0.253, 0.014],
    [0.249, 0.004],
    [0.239, 0.0],
    [-0.236, 0.0],
    [-0.246, 0.004],
    [-0.25, 0.014],
    [-0.25, 0.107],
    [-0.249, 0.118],
    [-0.244, 0.126],
    [-0.201, 0.191],
    [-0.082, 0.366],
    [0.007, 0.504],
    [0.071, 0.636],
    [0.092, 0.744],
    [0.077, 0.811],
    [0.034, 0.883],
    [0.022, 0.883],
    [-0.013, 0.883],
    [-0.062, 0.864],
    [-0.116, 0.809],
    [-0.115, 0.794],
    [-0.114, 0.749],
    [-0.114, 0.692],
    [-0.118, 0.681],
    [-0.129, 0.678],
    [-0.239, 0.678],
    [-0.249, 0.681],
    [-0.253, 0.692],
    [-0.253, 0.753],
    [-0.216, 0.87],
    [-0.113, 1.0],
    [-0.087, 1.0],
    [-0.007, 1.0],
    [0.105, 0.965],
    [0.229, 0.859],
    [0.229, 0.83],
    [0.229, 0.743],
    [0.216, 0.655],
    [0.179, 0.531],
    [0.167, 0.511],
    [0.129, 0.449],
    [0.074, 0.365],
    [-0.02, 0.225],
    [-0.089, 0.124],
    [-0.089, 0.121],
    [-0.088, 0.117],
    [-0.087, 0.117],
  ],
  3: [
    [0.241, 0.315],
    [0.238, 0.248],
    [0.228, 0.186],
    [0.186, 0.095],
    [0.098, 0.0],
    [0.074, 0.0],
    [0.002, 0.0],
    [-0.103, 0.027],
    [-0.222, 0.107],
    [-0.225, 0.131],
    [-0.234, 0.201],
    [-0.239, 0.252],
    [-0.241, 0.301],
    [-0.237, 0.311],
    [-0.227, 0.315],
    [-0.119, 0.315],
    [-0.109, 0.311],
    [-0.105, 0.301],
    [-0.102, 0.251],
    [-0.098, 0.219],
    [-0.079, 0.17],
    [-0.037, 0.119],
    [-0.028, 0.119],
    [0.002, 0.119],
    [0.067, 0.142],
    [0.102, 0.21],
    [0.109, 0.267],
    [0.111, 0.329],
    [0.108, 0.4],
    [0.098, 0.462],
    [0.077, 0.502],
    [0.037, 0.545],
    [0.029, 0.545],
    [0.002, 0.545],
    [-0.017, 0.542],
    [-0.036, 0.532],
    [-0.041, 0.529],
    [-0.046, 0.528],
    [-0.051, 0.53],
    [-0.056, 0.534],
    [-0.109, 0.589],
    [-0.113, 0.599],
    [-0.111, 0.609],
    [0.073, 0.876],
    [0.073, 0.878],
    [0.071, 0.883],
    [0.071, 0.883],
    [0.069, 0.883],
    [-0.217, 0.883],
    [-0.227, 0.886],
    [-0.231, 0.897],
    [-0.231, 0.986],
    [-0.227, 0.996],
    [-0.217, 1.0],
    [0.224, 1.0],
    [0.234, 0.996],
    [0.238, 0.986],
    [0.238, 0.893],
    [0.237, 0.882],
    [0.232, 0.874],
    [0.067, 0.631],
    [0.066, 0.627],
    [0.07, 0.624],
    [0.132, 0.596],
    [0.207, 0.527],
    [0.212, 0.51],
    [0.225, 0.458],
    [0.237, 0.387],
  ],
};

// The glyph's forward extent is ENTRY_DIGIT_HEIGHT_M. Starting it at the road edge
// leaves the same clear gap before the first beacon as between beacon edges.
const NUMBER_OFFSET_M = -3;
const BEACON_STEP_M = 10.8;
const BEACON_RADII_M = [3.525, 2.625, 1.8] as const;
/** The painted number's height on the ground. */
export const ENTRY_DIGIT_HEIGHT_M = 4.65;
/** From the number's back edge to the last beacon's front edge, along the
 *  road from its entry. */
const SPAN_M: readonly [number, number] = [NUMBER_OFFSET_M, BEACON_STEP_M * 3 + BEACON_RADII_M[2]];

/** The glow's radius in screen pixels, thinning with the strokes as the
 *  camera pulls out, and the ground radii (metres) it fades in over: none
 *  while the painted marker reads by itself, full once it has shrunk well
 *  inside the glow. */
const GLOW_PX = 56;
const GLOW_FADE_M: readonly [number, number] = [10, 25];
const GLOW_ALPHA = 0.9;
/** The glow's falloff: alpha (of GLOW_ALPHA) at each fraction of its radius. */
const GLOW_FALLOFF: readonly (readonly [number, number])[] = [
  [0, 1],
  [0.35, 0.6],
  [0.7, 0.2],
  [1, 0],
];

/** A road in's marker on the ground: its two ends, the number's and the
 *  last beacon's, and the point halfway between them. */
export function entryMarkerSpan(marker: EntryMarker): { ends: [P2, P2]; middle: P2 } {
  const at = (d: number) => along(marker.center, marker.facing, d);
  return { ends: [at(SPAN_M[0]), at(SPAN_M[1])], middle: at((SPAN_M[0] + SPAN_M[1]) / 2) };
}

export function buildDeploymentMarker(
  marker: EntryMarker,
  z: SurfaceHeight,
  style: OrderStyle,
  markerColor: Rgb,
  { stroke }: { stroke: StrokeWidth },
): { paintedMarching: Mesh } {
  const pen = orderPen(z, style, stroke);
  const color: Rgba = [markerColor[0], markerColor[1], markerColor[2], 0.86];
  const normal: readonly [number, number, number] = [
    0,
    style.march.cycles_per_s,
    Math.min(style.march.amplitude, 0.75),
  ];
  const painted = new MeshBuilder();
  const marching = new MeshBuilder();
  glow(painted, pen.z, entryMarkerSpan(marker).middle, stroke(GLOW_PX), markerColor);
  const number = along(marker.center, marker.facing, pen.line * 2 + NUMBER_OFFSET_M);
  digit(painted, pen.z, number, marker.facing, marker.lane, color);

  const first = pen.line * 2 + BEACON_STEP_M;
  for (let i = 0; i < 3; i++) {
    const at = along(marker.center, marker.facing, first + i * BEACON_STEP_M);
    groundAnnulus(marching, at, 0, BEACON_RADII_M[i], {
      z: pen.z,
      segments: 20,
      colorIn: color,
    });
  }

  const still = painted.build();
  return { paintedMarching: concatMeshes([unpulsed(still), withMarch(marching.build(), normal)]) };
}

/** A soft disc of the marker's light, `radius` metres across the ground,
 *  fading in as that radius grows through GLOW_FADE_M (none close in). */
function glow(mesh: MeshBuilder, z: SurfaceHeight, center: P2, radius: number, color: Rgb) {
  const strength = GLOW_ALPHA * remapClamp(radius, GLOW_FADE_M[0], GLOW_FADE_M[1], 0, 1);
  if (strength <= 0) return;
  const tone = (a: number): Rgba => [color[0], color[1], color[2], strength * a];
  for (let i = 1; i < GLOW_FALLOFF.length; i++) {
    const [r0, a0] = GLOW_FALLOFF[i - 1];
    const [r1, a1] = GLOW_FALLOFF[i];
    groundAnnulus(mesh, center, r0 * radius, r1 * radius, {
      z,
      segments: 32,
      colorIn: tone(a0),
      colorOut: tone(a1),
    });
  }
}

function digit(
  mesh: MeshBuilder,
  z: SurfaceHeight,
  center: P2,
  facing: number,
  lane: number,
  color: Rgba,
) {
  const [fx, fy] = [Math.cos(facing), Math.sin(facing)];
  const at = ([x, y]: P2): [number, number, number] => [
    center[0] + (fy * x + fx * y) * ENTRY_DIGIT_HEIGHT_M,
    center[1] + (-fx * x + fy * y) * ENTRY_DIGIT_HEIGHT_M,
    z(
      center[0] + (fy * x + fx * y) * ENTRY_DIGIT_HEIGHT_M,
      center[1] + (-fx * x + fy * y) * ENTRY_DIGIT_HEIGHT_M,
    ),
  ];
  const points = FONT_DIGITS[lane] ?? FONT_DIGITS[1];
  for (const [a, b, c] of triangulate(points)) mesh.triangle(at(a), at(b), at(c), color);
}

/** A glyph outline cut into triangles, each wound counter-clockwise so its
 *  normal faces up once the outline is laid on the ground. */
function triangulate(points: readonly P2[]): readonly [P2, P2, P2][] {
  const indices: number[] = new Array(Math.max(0, 3 * (points.length - 2))).fill(0);
  const count = triangulatePolygon2(indices, points.flat(), points.length);
  const triangles: [P2, P2, P2][] = [];
  for (let t = 0; t < count; t++) {
    const [a, b, c] = [
      points[indices[3 * t]],
      points[indices[3 * t + 1]],
      points[indices[3 * t + 2]],
    ];
    triangles.push(
      triangle2.signedArea(a as Vec2, b as Vec2, c as Vec2) >= 0 ? [a, b, c] : [a, c, b],
    );
  }
  return triangles;
}

function withMarch(mesh: Mesh, normal: readonly [number, number, number]): Mesh {
  const out = mesh.slice();
  for (let i = 0; i < out.length; i += VERTEX_FLOATS) {
    out[i + 3] = normal[0];
    out[i + 4] = normal[1];
    out[i + 5] = normal[2];
  }
  return out;
}
