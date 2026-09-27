// @vitest-environment node
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "../../packages/battle-renderer/src/mesh";
import {
  buildOrderOverlay,
  COVER_COLORS,
  type OrderView,
} from "../../packages/battle-renderer/src/orderOverlay";
import { dragFacing } from "../src/battle/input/useUnitControl";

const flat = () => 0;

/** Vertices of `color` in `mesh`. */
const count = (mesh: Mesh, color: Rgba) => {
  let n = 0;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    if ([0, 1, 2, 3].every((k) => Math.abs(mesh[i + 6 + k] - color[k]) < 1e-6)) n++;
  return n;
};
const both = (m: { opaque: Mesh; translucent: Mesh }, color: Rgba) =>
  count(m.opaque, color) + count(m.translucent, color);

const squad = (over: Partial<OrderView> = {}): OrderView => ({
  position: [0, 0, 0],
  goal: [40, 0],
  policy: "shortest",
  state: "moving",
  route: [[40, 0]],
  queue: [],
  members: [
    [0, 1, 0],
    [0, -1, 0],
  ],
  memberOrders: [
    { spot: [40, 2], coverNow: null, coverThere: "heavy" },
    { spot: [40, -2], coverNow: "light", coverThere: null },
  ],
  finalFacing: 0,
  direction: "forward",
  ...over,
});

test("cover icons appear only with Space, one per tier present, none for no cover", () => {
  const plain = buildOrderOverlay([squad()], flat);
  for (const c of Object.values(COVER_COLORS)) expect(both(plain, c)).toBe(0);
  const all = buildOrderOverlay([squad()], flat, { all: true });
  expect(both(all, COVER_COLORS.heavy)).toBeGreaterThan(0); // his spot, at the final marker
  expect(both(all, COVER_COLORS.light)).toBeGreaterThan(0); // his cover now, at the current one
  expect(both(all, COVER_COLORS.medium)).toBe(0);
});

test("Space adds a marker under each soldier's current position", () => {
  const plain = buildOrderOverlay([squad()], flat);
  const all = buildOrderOverlay([squad()], flat, { all: true });
  expect(all.translucent.length).toBeGreaterThan(plain.translucent.length);
});

test("a reverse move's final marker carries the reverse indicator", () => {
  const tank = (direction: string) =>
    buildOrderOverlay(
      [squad({ members: [], memberOrders: [], direction, finalFacing: Math.PI })],
      flat,
    );
  const amber: Rgba = [1.0, 0.72, 0.2, 1];
  expect(both(tank("reverse"), amber)).toBeGreaterThan(0);
  expect(both(tank("forward"), amber)).toBe(0);
});

test("a right-drag faces from the goal toward the release; a short drag sets none", () => {
  expect(dragFacing({ ground: [10, 10], facingTo: [10, 20] })).toBeCloseTo(Math.PI / 2);
  expect(dragFacing({ ground: [10, 10], facingTo: [10.3, 10] })).toBeUndefined();
  expect(dragFacing({ ground: [10, 10], facingTo: null })).toBeUndefined();
});
