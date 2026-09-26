// Presentation of projectile flight: flown chords as thin tubes, impact and
// near-miss marks, and body colliders at their current pose. Colours are
// presentation only; every point comes from the simulation.
import { MeshBuilder, type Rgba } from "./mesh";
import type { WorldMeshes } from "./scene";

type P3 = readonly [number, number, number];

export type TraceOutcome = "flying" | "struck-body" | "struck-world" | "expired" | "blocked";

export interface FlightTrace {
  /** Chord endpoints in flight order, ending at the impact point once struck. */
  points: readonly P3[];
  outcome: TraceOutcome;
  /** Overrides the outcome colour (e.g. whose round it is). */
  color?: Rgba;
}

export type FlightMarkKind =
  | "impact-body"
  | "impact-world"
  | "ricochet"
  | "blocked"
  | "near-miss"
  | "aim";

export interface FlightMark {
  at: P3;
  kind: FlightMarkKind;
  /** Outward surface normal: the mark sits on the surface instead of inside it. */
  normal?: P3;
}

export interface FlightBody {
  shape: "capsule" | "box";
  /** Ground contact centre. */
  base: P3;
  yaw: number;
  /** Capsule: radius, height. Box: half extents x, y, z. */
  dims: readonly number[];
  struck: boolean;
}

const TRACE: Record<TraceOutcome, Rgba> = {
  flying: [0.98, 0.97, 0.9, 1],
  "struck-body": [1, 0.42, 0.16, 1],
  "struck-world": [1, 0.84, 0.2, 1],
  expired: [0.7, 0.72, 0.76, 1],
  blocked: [0.95, 0.3, 0.3, 0.45],
};
const MARK: Record<FlightMarkKind, Rgba> = {
  "impact-body": [1, 0.2, 0.08, 1],
  "impact-world": [1, 0.84, 0.2, 1],
  // Where a round glanced off a hull and flew on: lime, apart from every hit.
  ricochet: [0.55, 1, 0.35, 1],
  // A shot that never flew must not read as a hit: near-black, not red.
  blocked: [0.12, 0.1, 0.12, 1],
  "near-miss": [0.3, 0.9, 1, 1],
  aim: [0.16, 0.2, 0.42, 1],
};
const BODY: Rgba = [0.34, 0.5, 0.86, 1];
// Distinct from the red impact marks so a mark on a struck body stays visible.
const BODY_STRUCK: Rgba = [0.6, 0.4, 0.68, 1];

/** `half` is the trace tube half width in metres; marks scale with it. */
export function buildFlightOverlay(
  traces: readonly FlightTrace[],
  marks: readonly FlightMark[],
  bodies: readonly FlightBody[],
  half: number,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const trace of traces) {
    const out = trace.outcome === "blocked" ? translucent : opaque;
    for (let i = 1; i < trace.points.length; i++) {
      out.segment(trace.points[i - 1], trace.points[i], half, trace.color ?? TRACE[trace.outcome]);
    }
  }
  const size = half * 4;
  for (const { at: point, kind, normal } of marks) {
    const at: P3 = normal
      ? [point[0] + normal[0] * size, point[1] + normal[1] * size, point[2] + normal[2] * size]
      : point;
    if (kind === "aim") {
      // A flat plate on the aimed ground point.
      opaque.box(at[0], at[1], at[2] + 0.02, size * 1.5, size * 1.5, 0.02, MARK.aim);
    } else {
      const s = kind === "near-miss" ? size * 0.7 : size;
      opaque.box(at[0], at[1], at[2], s, s, s, MARK[kind]);
    }
  }
  for (const body of bodies) {
    const color = body.struck ? BODY_STRUCK : BODY;
    if (body.shape === "capsule") {
      opaque.prism(body.base[0], body.base[1], body.base[2], body.dims[0], body.dims[1], 10, color);
    } else {
      const [hx, hy, hz] = body.dims;
      opaque.orientedBox(body.base[0], body.base[1], body.yaw, [hx, hy, hz], body.base[2], color);
    }
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
