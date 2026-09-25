// Garrison presentation for one side, from its own units' published state: a
// pad under each occupant at its perimeter slot (the small exposed region a
// round can hit; anything wide of it meets the wall), edged toward orange as
// the squad is pinned, and an arc around a squad entering or leaving a
// building whose length is the timer's progress. A squad refused for want of
// room gets a full red ring instead.
import { MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

type P3 = readonly [number, number, number];

export interface GarrisonMark {
  /** Squad centre (the building's centre while inside). */
  center: readonly [number, number];
  /** Occupant positions: their perimeter slots while inside. */
  members: readonly P3[];
  phase: string;
  progress: number;
  /** In [0, 1]: occupant pads edge toward orange as the squad is pinned. */
  suppression: number;
}

export const OCCUPANT_PAD: Rgba = [0.55, 0.78, 1.0, 0.55];
const PAD_EDGE: Rgba = [0.08, 0.12, 0.2, 1];
const PINNED_EDGE: Rgba = [1.0, 0.62, 0.1, 1];
const TIMER: Rgba = [0.96, 0.96, 0.9, 1];
const NO_ROOM: Rgba = [1.0, 0.32, 0.26, 1];
const SEGMENTS = 32;
const LIFT_M = 0.35;
// The pad stays inside the slot's standoff from the wall, so the wall never
// hides half of it.
const PAD_M = 0.3;
const PAD_EDGE_M = 0.14;
const RING_M = 7;
const RING_WIDTH_M = 0.6;

/** A flat annulus sector from angle 0 through `turn` of a full circle. */
function arc(
  mesh: MeshBuilder,
  cx: number,
  cy: number,
  inner: number,
  outer: number,
  turn: number,
  color: Rgba,
  z: SurfaceHeight,
) {
  const at = (a: number, r: number): P3 => {
    const x = cx + Math.cos(a) * r,
      y = cy + Math.sin(a) * r;
    return [x, y, z(x, y) + LIFT_M];
  };
  const n = Math.max(1, Math.ceil(SEGMENTS * turn));
  for (let k = 0; k < n; k++) {
    const [t0, t1] = [(k / n) * turn * Math.PI * 2, ((k + 1) / n) * turn * Math.PI * 2];
    mesh.shadedTriangle(at(t0, inner), at(t1, inner), at(t1, outer), color, color, color);
    mesh.shadedTriangle(at(t0, inner), at(t1, outer), at(t0, outer), color, color, color);
  }
}

export function buildGarrisonOverlay(
  squads: readonly GarrisonMark[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of squads) {
    if (s.phase === "inside" || s.phase === "exiting") {
      for (const [x, y] of s.members) {
        const t = Math.min(1, Math.max(0, s.suppression));
        const edge: Rgba = [
          PAD_EDGE[0] + (PINNED_EDGE[0] - PAD_EDGE[0]) * t,
          PAD_EDGE[1] + (PINNED_EDGE[1] - PAD_EDGE[1]) * t,
          PAD_EDGE[2] + (PINNED_EDGE[2] - PAD_EDGE[2]) * t,
          1,
        ];
        arc(translucent, x, y, 0, PAD_M, 1, OCCUPANT_PAD, z);
        arc(opaque, x, y, PAD_M, PAD_M + PAD_EDGE_M, 1, edge, z);
      }
    }
    const [cx, cy] = s.center;
    if (s.phase === "waiting_for_room") {
      arc(opaque, cx, cy, RING_M, RING_M + RING_WIDTH_M, 1, NO_ROOM, z);
    } else if (s.phase === "entering" || s.phase === "exiting") {
      arc(opaque, cx, cy, RING_M, RING_M + RING_WIDTH_M, s.progress, TIMER, z);
    }
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
