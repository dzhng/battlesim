// @vitest-environment node
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "../../packages/battle-renderer/src/mesh";
import {
  buildOrderOverlay as build,
  validateOrderStyle,
  type OrderStyle,
  type OrderView,
} from "../../packages/battle-renderer/src/orderOverlay";
import type { WorldMeshes } from "../../packages/battle-renderer/src/scene";
import { dragFacing } from "../src/battle/input/useUnitControl";
import village from "../../fixtures/village.json";

const flat = () => 0;
const STYLE = validateOrderStyle(village.presentation.overlay.orders as unknown as OrderStyle);
const COVER_COLORS = STYLE.cover;
const buildOrderOverlay = (units: OrderView[], z: typeof flat, o: { all?: boolean } = {}) =>
  build(units, z, STYLE, { metresPerPx: 0.05, ...o });

/** Every mesh an overlay draws with a colour (not the animated marks,
 *  whose normals carry their march). */
const drawn = (m: WorldMeshes) => [m.opaque, m.translucent, m.unoccluded ?? new Float32Array(0)];
/** Every distinct colour `mesh` draws in. */
const colours = (m: WorldMeshes) => {
  const seen = new Set<string>();
  for (const mesh of drawn(m))
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
      seen.add([0, 1, 2, 3].map((k) => mesh[i + 6 + k].toFixed(4)).join(","));
  return seen;
};
const key = (c: Rgba) => c.map((v) => v.toFixed(4)).join(",");

/** Vertices of `color` in `mesh`. */
const count = (mesh: Mesh, color: Rgba) => {
  let n = 0;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    if ([0, 1, 2, 3].every((k) => Math.abs(mesh[i + 6 + k] - color[k]) < 1e-6)) n++;
  return n;
};
const both = (m: WorldMeshes, color: Rgba) =>
  drawn(m).reduce((n, mesh) => n + count(mesh, color), 0);

const squad = (over: Partial<OrderView> = {}): OrderView => ({
  position: [0, 0, 0],
  goal: [40, 0],
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
  area: { anchor: [40, 0], radius: 3 },
  ...over,
});
/** A vehicle's view: no soldiers, no area. */
const vehicle = { members: [], memberOrders: [], area: null };

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

/** Where a mesh's travel chevrons point along x: +1 when their tips lie
 *  toward +x of their arms' ends, −1 toward −x. */
const chevronsPoint = (animated: Mesh) => {
  const tips: number[] = [],
    ends: number[] = [];
  for (let i = 0; i < animated.length; i += VERTEX_FLOATS)
    (Math.abs(animated[i + 1]) < 0.2 ? tips : Math.abs(animated[i + 1]) > 0.6 ? ends : []).push(
      animated[i],
    );
  const mean = (v: number[]) => v.reduce((a, b) => a + b, 0) / v.length;
  return Math.sign(mean(tips) - mean(ends));
};

test("a moving vehicle's travel chevrons go under it: with its facing forward, against it in reverse", () => {
  // The tank at (0, 0) moves to (40, 0), +x. Forward it faces +x; reversing
  // there, its hull faces −x (π) and it backs toward +x.
  const tank = (direction: string, finalFacing: number) =>
    buildOrderOverlay([squad({ ...vehicle, direction, finalFacing, selected: true })], flat);
  const forward = tank("forward", 0),
    reverse = tank("reverse", Math.PI);
  expect(chevronsPoint(forward.animated!)).toBe(Math.sign(Math.cos(0))); // with the facing
  expect(chevronsPoint(reverse.animated!)).toBe(-Math.sign(Math.cos(Math.PI))); // against it
  for (const m of [forward, reverse])
    for (let i = 0; i < m.animated!.length; i += VERTEX_FLOATS) {
      expect(Math.abs(m.animated![i])).toBeLessThan(10); // under the moving unit, not at (40, 0)
      expect(m.animated![i + 4]).toBe(STYLE.march.cycles_per_s);
      expect(m.animated![i + 5]).toBeCloseTo(STYLE.march.amplitude, 6);
    }
});

test("a vehicle's destination marker points its facing and shows no travel chevrons", () => {
  const dest = buildOrderOverlay(
    [squad({ ...vehicle, direction: "reverse", finalFacing: Math.PI })],
    flat,
  );
  expect(dest.animated!.length).toBe(0);
  expect([...colours(dest)]).toEqual([key(STYLE.color)]);
  // Its facing mark reaches out from the circle toward −x.
  let tip = Infinity;
  for (let i = 0; i < dest.opaque.length; i += VERTEX_FLOATS)
    if (dest.opaque[i] > 30 && Math.abs(dest.opaque[i + 1]) < 0.2)
      tip = Math.min(tip, dest.opaque[i]);
  expect(tip).toBeLessThan(40 - 2);
});

test("only a moving vehicle shows travel chevrons under it", () => {
  const picked = { ...vehicle, selected: true };
  const resting = buildOrderOverlay([squad({ ...picked, goal: null, route: [] })], flat);
  expect(resting.animated!.length).toBe(0);
  expect(both(resting, STYLE.selected)).toBeGreaterThan(0);
  const moving = buildOrderOverlay([squad(picked)], flat);
  expect(moving.animated!.length).toBeGreaterThan(0);
});

test("an order draws in one colour whatever its kind; the selection's marker in its own", () => {
  const plain = buildOrderOverlay([squad()], flat);
  expect([...colours(plain)]).toEqual([key(STYLE.color)]);
  const picked = buildOrderOverlay([squad({ selected: true })], flat);
  expect(both(picked, STYLE.selected)).toBeGreaterThan(0);
  expect(both(plain, STYLE.selected)).toBe(0);
  // A vehicle's selection marker too.
  const tank = buildOrderOverlay([squad({ ...vehicle, selected: true })], flat);
  expect(both(tank, STYLE.selected)).toBeGreaterThan(0);
});

test("a squad's route runs from the edge of the circle it stands in to the edge of its area ring", () => {
  const mesh = buildOrderOverlay([squad()], flat).opaque;
  // The route runs along x from the squad (0, 0; soldiers 1 m either side)
  // to its goal (40, 0; spots 2 m either side). Its edges lie half a line
  // off the axis.
  let [start, end] = [Infinity, -Infinity];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    if (Math.abs(Math.abs(mesh[i + 1]) - (STYLE.line_px * 0.05) / 2) < 1e-6) {
      start = Math.min(start, mesh[i]);
      end = Math.max(end, mesh[i]);
    }
  expect(start).toBeCloseTo((1 + 0.85) * STYLE.area_draw_scale, 1); // round the soldiers
  expect(end).toBeCloseTo(40 - 3 * STYLE.area_draw_scale, 1); // 27d's area, drawn smaller
});

test("with Space, a holding squad's area is drawn once, round its anchor", () => {
  const holding = squad({
    goal: null,
    state: "idle",
    route: [],
    members: [
      [22, 3, 0],
      [18, -6, 0],
    ],
    // At their posts: no legs to walk.
    memberOrders: [
      { spot: [22, 3], coverNow: null, coverThere: null },
      { spot: [18, -6], coverNow: null, coverThere: null },
    ],
    area: { anchor: [10, 0], radius: 14 },
  });
  const current = [
    STYLE.color[0],
    STYLE.color[1],
    STYLE.color[2],
    STYLE.color[3] * STYLE.current_alpha,
  ] as Rgba;
  const ringAt = (m: WorldMeshes) => {
    const radii: number[] = [];
    for (let i = 0; i < m.translucent.length; i += VERTEX_FLOATS)
      if ([0, 1, 2, 3].every((k) => Math.abs(m.translucent[i + 6 + k] - current[k]) < 1e-6)) {
        const [x, y] = [m.translucent[i], m.translucent[i + 1]];
        // Not the markers under the soldiers themselves.
        if (holding.members.some((p) => Math.hypot(x - p[0], y - p[1]) < 2)) continue;
        radii.push(Math.hypot(x - 10, y));
      }
    return radii;
  };
  expect(ringAt(buildOrderOverlay([holding], flat))).toEqual([]);
  const radii = ringAt(buildOrderOverlay([holding], flat, { all: true }));
  expect(radii.length).toBeGreaterThan(0);
  for (const r of radii) expect(r).toBeCloseTo(14 * STYLE.area_draw_scale, 0);
});

test("a right-drag faces from the goal toward the release; a short drag sets none", () => {
  expect(dragFacing({ ground: [10, 10], facingTo: [10, 20] })).toBeCloseTo(Math.PI / 2);
  expect(dragFacing({ ground: [10, 10], facingTo: [10.3, 10] })).toBeUndefined();
  expect(dragFacing({ ground: [10, 10], facingTo: null })).toBeUndefined();
});

test("routes are drawn a fixed width on screen, never under the floor in metres", () => {
  const route = (metresPerPx: number) => {
    const mesh = build([squad({ memberOrders: [], members: [] })], flat, STYLE, {
      metresPerPx,
    }).opaque;
    // The route's first quad: y spans its width (the leg runs along x).
    let lo = Infinity,
      hi = -Infinity;
    for (let i = 0; i < 6 * VERTEX_FLOATS; i += VERTEX_FLOATS) {
      lo = Math.min(lo, mesh[i + 1]);
      hi = Math.max(hi, mesh[i + 1]);
    }
    return hi - lo;
  };
  expect(route(0.05)).toBeCloseTo(STYLE.line_px * 0.05, 5);
  expect(route(1)).toBeCloseTo(STYLE.line_px, 5);
  expect(route(1e-5)).toBeCloseTo(STYLE.min_line_m, 5);
});

test("the fixture's order style is complete, and a missing colour is refused", () => {
  const broken = { ...STYLE, selected: undefined };
  expect(() => validateOrderStyle(broken as unknown as OrderStyle)).toThrow(/selected/);
});

test("a squad's area ring points its final facing with an arrowhead on its rim, no arrow beyond", () => {
  // Area round (40, 0), radius 3, drawn at the fixture's scale; facing +y.
  const mesh = buildOrderOverlay([squad({ finalFacing: Math.PI / 2 })], flat).opaque;
  const rim = 3 * STYLE.area_draw_scale;
  let far = { d: 0, x: 0 };
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
    const d = Math.hypot(mesh[i] - 40, mesh[i + 1]);
    // Round the area (not the squad at the origin), above the route in along y = 0.
    if (mesh[i] > 30 && mesh[i + 1] > 0.5 && d > far.d) far = { d, x: mesh[i] - 40 };
  }
  // Its farthest point is the arrowhead's tip, straight along the facing,
  // a little past the rim: nothing reaches further out.
  expect(far.x).toBeCloseTo(0, 3);
  expect(far.d).toBeGreaterThan(rim + 0.5);
  expect(far.d).toBeLessThan(rim + 2.5);
});
