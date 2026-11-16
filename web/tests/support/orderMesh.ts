// A unit's order marks as the game's overlay paints them, and the probe the
// order tests read them with: which stretches of a segment the painted
// triangles lying along it cover.
import { VERTEX_FLOATS, type Mesh } from "@packages/battle-renderer/src/mesh";
import { buildOrderOverlay, type OrderView } from "@packages/battle-renderer/src/orderOverlay";
import { gameOrderStyle, gameStroke } from "@apps/battle-lab/src/gameOverlay";

type P2 = readonly [number, number];

/** A ground unit standing at the origin with nothing ordered, a 3 m hull
 *  and fully revealed marks, with `over` in place of any of that. */
export function orderView(over: Partial<OrderView>): OrderView {
  return {
    position: [0, 0, 0],
    goal: null,
    state: "idle",
    route: [],
    steps: [],
    members: [],
    memberOrders: [],
    finalFacing: 0,
    yaw: 0,
    direction: null,
    area: null,
    hullHalfLength: 3,
    building: null,
    reveal: 1,
    ...over,
  };
}

/** The game's painted order marks for `v`, at a close zoom's stroke. */
export const painted = (v: OrderView): Mesh =>
  buildOrderOverlay([v], () => 0, gameOrderStyle, { stroke: gameStroke(0.05) }).painted ??
  new Float32Array();

/** The stretches of segment a→b, in metres from a, that painted triangles
 *  lying along it (each corner within `band` metres of its line, and no
 *  further than `band` past either end) cover, merged where they touch. */
export function along(mesh: Mesh, a: P2, b: P2, band = 1): [number, number][] {
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const len = Math.hypot(dx, dy);
  const spans: [number, number][] = [];
  const TRI = 3 * VERTEX_FLOATS;
  for (let i = 0; i + TRI <= mesh.length; i += TRI) {
    const ts: number[] = [];
    for (let k = 0; k < 3; k++) {
      const [px, py] = [mesh[i + k * VERTEX_FLOATS] - a[0], mesh[i + k * VERTEX_FLOATS + 1] - a[1]];
      if (Math.abs(px * dy - py * dx) / len > band) break;
      ts.push((px * dx + py * dy) / len);
    }
    if (ts.length === 3 && Math.min(...ts) > -band && Math.max(...ts) < len + band)
      spans.push([Math.min(...ts), Math.max(...ts)]);
  }
  spans.sort((p, q) => p[0] - q[0]);
  const merged: [number, number][] = [];
  for (const s of spans) {
    const last = merged.at(-1);
    if (last && s[0] <= last[1] + 1e-6) last[1] = Math.max(last[1], s[1]);
    else merged.push([...s]);
  }
  return merged;
}
