// A reinforcement lane beacon. It answers one question, "which lane is
// this?" with a lane number and a short trail of matching beacons.
import {
  concatMeshes,
  groundAnnulus,
  groundStrip,
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

const DIGITS: Readonly<Record<number, readonly (readonly P2[])[]>> = {
  1: [
    [
      [-0.2, 0.78],
      [0.04, 1],
      [0.04, 0],
    ],
    [
      [-0.22, 0],
      [0.3, 0],
    ],
  ],
  2: [
    [
      [-0.3, 0.65],
      [-0.2, 0.84],
      [0, 1],
      [0.23, 0.88],
      [0.3, 0.68],
      [-0.3, 0],
      [0.32, 0],
    ],
  ],
  3: [
    [
      [-0.24, 0.82],
      [-0.08, 1],
      [0.22, 0.9],
      [0.22, 0.62],
      [0.02, 0.5],
      [0.22, 0.38],
      [0.22, 0.1],
      [-0.08, 0],
      [-0.25, 0.17],
    ],
  ],
};

// The glyph's forward extent is DIGIT_HEIGHT_M. Starting it at the road edge
// leaves the same clear gap before the first beacon as between beacon edges.
const NUMBER_OFFSET_M = -3;
const BEACON_STEP_M = 10.8;
const BEACON_RADII_M = [3.525, 2.625, 1.8] as const;
const DIGIT_HEIGHT_M = 4.65;
const DIGIT_STROKE_M = 1.2;

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

function digit(
  mesh: MeshBuilder,
  z: SurfaceHeight,
  center: P2,
  facing: number,
  lane: number,
  color: Rgba,
) {
  const [fx, fy] = [Math.cos(facing), Math.sin(facing)];
  const at = ([x, y]: P2): P2 => [
    center[0] + (fy * x + fx * y) * DIGIT_HEIGHT_M,
    center[1] + (-fx * x + fy * y) * DIGIT_HEIGHT_M,
  ];
  for (const line of DIGITS[lane] ?? DIGITS[1]) {
    const points = line.map(at);
    for (let i = 1; i < points.length; i++)
      groundStrip(mesh, points[i - 1], points[i], DIGIT_STROKE_M, color, { z, step: 3 });
  }
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
