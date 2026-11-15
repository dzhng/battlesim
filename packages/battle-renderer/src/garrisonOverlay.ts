// Garrison presentation for one side: one outline painted round every
// building a garrison is known to hold, all its parts in one (its side's
// colour, traced on the ground, so the player reads at a glance which
// buildings are manned and by whom), and for the garrison lab, from its own units'
// published state, a pad under each occupant at its perimeter slot (the small
// exposed region a round can hit; anything wide of it meets the wall). Entering,
// leaving and being pinned are the squad's info panel's.
import { groundAnnulus, groundStrip, MeshBuilder, type Mesh, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";
import type { Vec3 } from "math";
import { quickhull2 } from "math/geometry";

export interface GarrisonMark {
  /** Occupant positions: their perimeter slots while inside. */
  members: readonly Readonly<Vec3>[];
  phase: string;
}

const OCCUPANT_PAD: Rgba = [0.55, 0.78, 1.0, 0.55];
const PAD_EDGE: Rgba = [0.08, 0.12, 0.2, 1];
const SEGMENTS = 32;
const LIFT_M = 0.35;
// The pad stays inside the slot's standoff from the wall, so the wall never
// hides half of it.
const PAD_M = 0.3;
const PAD_EDGE_M = 0.14;

/** A flat disc or ring round (`cx`, `cy`). */
function disc(
  mesh: MeshBuilder,
  cx: number,
  cy: number,
  inner: number,
  outer: number,
  color: Rgba,
  z: SurfaceHeight,
) {
  groundAnnulus(mesh, [cx, cy], inner, outer, {
    z,
    lift: LIFT_M,
    segments: SEGMENTS,
    colorIn: color,
  });
}

export function buildGarrisonOverlay(
  squads: readonly GarrisonMark[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of squads) {
    if (s.phase !== "inside" && s.phase !== "exiting") continue;
    for (const [x, y] of s.members) {
      disc(translucent, x, y, 0, PAD_M, OCCUPANT_PAD, z);
      disc(opaque, x, y, PAD_M, PAD_M + PAD_EDGE_M, PAD_EDGE, z);
    }
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}

/** A building a garrison holds: its parts' ground boxes (centre, yaw, half
 *  extents) and the holder's side colour. */
export interface HeldBuilding {
  parts: readonly {
    center: readonly [number, number];
    yaw: number;
    half: readonly [number, number, ...number[]];
  }[];
  color: Rgba;
}

type P2 = readonly [number, number];

/** How far a part's wall may face from a joining line's outward side and
 *  still be walked along before the line leaves the part. */
const FACING_COS = Math.SQRT1_2;

/** One outline round a building's parts, `margin` outside their walls,
 *  counter-clockwise. Round one part it is that part's box. Round several
 *  it is their convex hull, except that a line joining two parts across open
 *  ground first runs along each wall of theirs that faces out the way it
 *  does (within 45°): a small part on the hull's corner keeps its outward
 *  wall, rather than being cut across from corner to corner. */
export function heldOutline(parts: HeldBuilding["parts"], margin: number): P2[] {
  // Each part's corners and its walls' outward normals, counter-clockwise.
  const boxes = parts.map(({ center, yaw, half }) => {
    const [c, s] = [Math.cos(yaw), Math.sin(yaw)];
    const [hx, hy] = [half[0] + margin, half[1] + margin];
    const turn = ([u, v]: P2): P2 => [u * c - v * s, u * s + v * c];
    const corners = (
      [
        [-hx, -hy],
        [hx, -hy],
        [hx, hy],
        [-hx, hy],
      ] as const
    ).map(([u, v]) => {
      const [x, y] = turn([u, v]);
      return [center[0] + x, center[1] + y] as P2;
    });
    const normals = (
      [
        [0, -1],
        [1, 0],
        [0, 1],
        [-1, 0],
      ] as const
    ).map(turn);
    return { center, c, s, hx, hy, corners, normals };
  });
  const inside = ([x, y]: P2, skip: number) =>
    boxes.some((b, i) => {
      if (i === skip) return false;
      const [dx, dy] = [x - b.center[0], y - b.center[1]];
      const [u, v] = [dx * b.c + dy * b.s, -dx * b.s + dy * b.c];
      return Math.abs(u) < b.hx - 1e-6 && Math.abs(v) < b.hy - 1e-6;
    });
  const corners = boxes.flatMap((b) => b.corners);
  const hull = quickhull2(corners.flat());
  const ring: P2[] = [];
  hull.forEach((k, i) => {
    const next = hull[(i + 1) % hull.length];
    ring.push(corners[k]);
    const [p, q] = [Math.floor(k / 4), Math.floor(next / 4)];
    if (p === q) return;
    // A joining line: walk on round the part it leaves, and back round the
    // part it reaches, along walls that face out its way.
    const [a, b] = [corners[k], corners[next]];
    const length = Math.hypot(b[0] - a[0], b[1] - a[1]);
    const out: P2 = [(b[1] - a[1]) / length, -(b[0] - a[0]) / length];
    const faces = ([x, y]: P2) => x * out[0] + y * out[1] > FACING_COS;
    const leaving: P2[] = [];
    for (let j = k % 4, n = 0; n < 2 && faces(boxes[p].normals[j]); n++) {
      j = (j + 1) % 4;
      if (inside(boxes[p].corners[j], p)) break;
      leaving.push(boxes[p].corners[j]);
    }
    const reaching: P2[] = [];
    for (let j = next % 4, n = 0; n < 2 && faces(boxes[q].normals[(j + 3) % 4]); n++) {
      j = (j + 3) % 4;
      if (inside(boxes[q].corners[j], q)) break;
      reaching.unshift(boxes[q].corners[j]);
    }
    ring.push(...leaving, ...reaching);
  });
  return ring;
}

/** Each held building's outline (`heldOutline`): a line `line` wide, painted
 *  on the ground so the shell hides its far side as it hides the ground. */
export function buildGarrisonOutlines(
  held: readonly HeldBuilding[],
  line: number,
  margin: number,
  z: SurfaceHeight,
): Mesh {
  const mesh = new MeshBuilder();
  for (const { parts, color } of held) {
    const ring = heldOutline(parts, margin);
    ring.forEach((a, i) => {
      // Each side runs on past its corners by half the line, closing them.
      const b = ring[(i + 1) % ring.length];
      const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
      const k = line / 2 / Math.hypot(dx, dy);
      groundStrip(
        mesh,
        [a[0] - dx * k, a[1] - dy * k],
        [b[0] + dx * k, b[1] + dy * k],
        line,
        color,
        {
          z,
        },
      );
    });
  }
  return mesh.build();
}
