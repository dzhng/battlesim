// @vitest-environment node
// A unit's queued legs chain from where its current order ends. A blocked
// unit has no destination circle, only its blocked mark at the goal, so its
// first queued leg leaves that mark; a winged aircraft circles its goal and
// keeps today's drawing (no ground leg to its first queued point).
import { expect, test } from "vitest";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import { buildOrderOverlay, type OrderView } from "@packages/battle-renderer/src/orderOverlay";
import { gameOrderStyle, gameStroke } from "@apps/battle-lab/src/gameOverlay";

type P2 = readonly [number, number];
const GOAL: P2 = [40, 0];
const QUEUED: P2 = [40, 60];

function view(over: Partial<OrderView>): OrderView {
  return {
    position: [0, 0, 0],
    goal: GOAL,
    state: "moving",
    route: [GOAL],
    queue: [QUEUED],
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

/** The stretches of the segment a→b the painted triangles lying along it
 *  (within `band` metres) cover, as [from, to] fractions of its length,
 *  merged where they touch. */
function along(v: OrderView, a: P2, b: P2, band = 1): [number, number][] {
  const mesh =
    buildOrderOverlay([v], () => 0, gameOrderStyle, { stroke: gameStroke(0.05) }).painted ??
    new Float32Array();
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const len = Math.hypot(dx, dy);
  const spans: [number, number][] = [];
  const TRI = 3 * VERTEX_FLOATS;
  for (let i = 0; i + TRI <= mesh.length; i += TRI) {
    const ts: number[] = [];
    for (let k = 0; k < 3; k++) {
      const [px, py] = [mesh[i + k * VERTEX_FLOATS] - a[0], mesh[i + k * VERTEX_FLOATS + 1] - a[1]];
      if (Math.abs(px * dy - py * dx) / len > band) break;
      ts.push((px * dx + py * dy) / (len * len));
    }
    if (ts.length === 3) spans.push([Math.min(...ts), Math.max(...ts)]);
  }
  spans.sort((p, q) => p[0] - q[0]);
  const merged: [number, number][] = [];
  for (const s of spans) {
    const last = merged.at(-1);
    if (last && s[0] <= last[1] + 1e-6) last[1] = Math.max(last[1], s[1]);
    else merged.push([...s]);
  }
  return merged.filter(([p, q]) => q > 0.15 && p < 0.85);
}

test("a blocked unit's first queued leg runs dashed from its blocked mark to the queued ring", () => {
  const leg = along(view({ state: "route_blocked" }), GOAL, QUEUED);
  // Paint runs most of the way between the two marks, in several dashes.
  expect(leg.length).toBeGreaterThan(2);
  expect(leg[0][0]).toBeLessThan(0.2);
  expect(leg.at(-1)![1]).toBeGreaterThan(0.8);
});

test("a winged aircraft draws no ground leg to its first queued point", () => {
  const jet = view({ fixedWing: true, airborne: true });
  expect(along(jet, GOAL, QUEUED)).toEqual([]);
});
