// @vitest-environment node
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { expect, test } from "vitest";
import {
  concatMeshes,
  VERTEX_FLOATS,
  type Mesh,
  type Rgba,
} from "../../packages/battle-renderer/src/mesh";
import {
  buildOrderOverlay as build,
  resolveOrderScheme,
  validateOrderStyle,
  type AuthoredOrderStyle,
  type OrderStyle,
  type OrderView,
} from "../../packages/battle-renderer/src/orderOverlay";
import type { WorldMeshes } from "../../packages/battle-renderer/src/scene";
import {
  validatePaintStyle,
  type PaintStyle,
} from "../../packages/battle-renderer/src/frame/paintedMarks";
import {
  buildMapBorder,
  validateMapBorder,
  type MapBorderStyle,
} from "../../packages/battle-renderer/src/playAreaOverlay";
import {
  validateSupplyStyle,
  type SupplyStyle,
} from "../../packages/battle-renderer/src/supplyOverlay";
import {
  validateConsequenceStyle,
  type ConsequenceStyle,
} from "../../packages/battle-renderer/src/consequenceOverlay";
import { strokeWidth, validateStrokeRule } from "../../packages/battle-renderer/src/strokeWidth";
import { dragFacing } from "../src/battle/input/useUnitControl";
import village from "../../fixtures/village.json";

const flat = () => 0;
const STROKE_RULE = validateStrokeRule(village.presentation.overlay.stroke);
/** The stroke widths where one pixel spans `m` metres. */
const at = (m: number) => ({ stroke: strokeWidth(STROKE_RULE, m) });
const STYLE = validateOrderStyle(
  resolveOrderScheme(village.presentation.overlay.orders as unknown as AuthoredOrderStyle),
);
/** The cover pips as painted: their colours past full value by `cover_glow`. */
const glowing = (c: Rgba, g: number): Rgba => [c[0] * g, c[1] * g, c[2] * g, c[3]];
const COVER_COLORS = {
  light: glowing(STYLE.cover.light, STYLE.cover_glow),
  medium: glowing(STYLE.cover.medium, STYLE.cover_glow),
  heavy: glowing(STYLE.cover.heavy, STYLE.cover_glow),
};
/** The selection's own circles: overlay, in the style's colour. */
const SELECTED: Rgba =
  STYLE.layers.selected === "world"
    ? [
        STYLE.selected[0] * STYLE.selected_glow,
        STYLE.selected[1] * STYLE.selected_glow,
        STYLE.selected[2] * STYLE.selected_glow,
        STYLE.selected[3],
      ]
    : STYLE.selected;
/** A selected squad's soldiers' markers, likewise. */
const SOLDIER_SELECTED: Rgba =
  STYLE.layers.soldier === "world"
    ? glowing(STYLE.soldier_selected, STYLE.selected_glow)
    : STYLE.soldier_selected;
/** The geometry these tests read, whichever layer the fixture's scheme
 *  draws each role in: overlay and paint as one mesh (`painted`). */
const merged = (m: WorldMeshes): WorldMeshes => ({
  ...m,
  painted: concatMeshes([m.opaque, m.translucent, m.painted ?? new Float32Array(0)]),
});
const buildOrderOverlay = (units: OrderView[], z: typeof flat) =>
  merged(build(units, z, STYLE, at(0.05)));

/** Every mesh an overlay draws with a colour (not the animated marks,
 *  whose normals carry their march). */
/** Everything drawn: the selection's circles (overlay) and the paint. */
const drawn = (m: WorldMeshes) => [m.painted!];
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
  yaw: 0,
  direction: "forward",
  area: { anchor: [40, 0], radius: 3 },
  hullHalfLength: 0,
  // Its order marks shown in full (Space held, or its order's flash).
  reveal: 1,
  ...over,
});
/** A tank's view: no soldiers, no area, its type's hull. */
const TANK_HALF = UNITS.hull("tank")!.half_extents_m[0];
const vehicle = { members: [], memberOrders: [], area: null, hullHalfLength: TANK_HALF };

test("cover icons appear only with the order marks, one per tier present, none for no cover", () => {
  // A pip is a filled disc in the middle of a soldier's marker: a vertex of
  // its tier's colour at the soldier (cover now) or at his spot (cover
  // there). Light cover shares the orders' colour, so it is found there.
  const pipAt = (m: WorldMeshes, c: Rgba, at: readonly [number, number]) => {
    const mesh = m.painted!;
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
      if (
        Math.hypot(mesh[i] - at[0], mesh[i + 1] - at[1]) < 1e-3 &&
        [0, 1, 2, 3].every((k) => Math.abs(mesh[i + 6 + k] - c[k]) < 1e-6)
      )
        return true;
    return false;
  };
  // Selected, its order marks hidden: the selection's markers carry no pip.
  const selection = buildOrderOverlay([squad({ selected: true, reveal: 0 })], flat);
  expect(pipAt(selection, COVER_COLORS.heavy, [40, 2])).toBe(false);
  expect(pipAt(selection, COVER_COLORS.light, [0, -1])).toBe(false);
  const all = buildOrderOverlay([squad()], flat);
  expect(pipAt(all, COVER_COLORS.heavy, [40, 2])).toBe(true); // his spot, at the final marker
  expect(pipAt(all, COVER_COLORS.light, [0, -1])).toBe(true); // his cover now, where he stands
  expect(pipAt(all, COVER_COLORS.medium, [0, -1])).toBe(false);
  expect(pipAt(all, COVER_COLORS.light, [0, 1])).toBe(false); // no cover: no pip
});

test("a unit neither shown nor selected draws nothing; shown, a marker under each soldier", () => {
  const hidden = buildOrderOverlay([squad({ reveal: 0 })], flat);
  expect(hidden.painted!.length + hidden.paintedMarching!.length).toBe(0);
  const shown = buildOrderOverlay([squad({ route: [], goal: null, area: null })], flat).painted!;
  for (const [x, y] of [
    [0, 1],
    [0, -1],
  ]) {
    let near = 0;
    for (let i = 0; i < shown.length; i += VERTEX_FLOATS)
      if (Math.hypot(shown[i] - x, shown[i + 1] - y) < 0.6) near++;
    expect(near).toBeGreaterThan(0);
  }
});

test("a selection alone shows only the selection's own markers: no route, destination or spot", () => {
  const picked = buildOrderOverlay([squad({ selected: true, reveal: 0 })], flat).painted!;
  // The squad stands round the origin; its route, area and spots lie out
  // along +x to (40, 0).
  let far = 0;
  for (let i = 0; i < picked.length; i += VERTEX_FLOATS) if (picked[i] > 5) far++;
  expect(picked.length).toBeGreaterThan(0);
  expect(far).toBe(0);
  expect(both(buildOrderOverlay([squad({ selected: true, reveal: 0 })], flat), SELECTED)).toBe(
    both(buildOrderOverlay([squad({ selected: true })], flat), SELECTED),
  );
});

test("a selected holding squad keeps its area ring as the selection's marker, without Space", () => {
  const holding = (over: Partial<OrderView>) =>
    squad({
      goal: null,
      state: "holding",
      route: [],
      area: { anchor: [0, 0], radius: 10 },
      memberOrders: [],
      ...over,
    } as Partial<OrderView>);
  // Ink out at the ring (the soldiers stand within 1 m of the anchor).
  const ringInk = (selected: boolean) => {
    const m = buildOrderOverlay([holding({ selected, reveal: 0 })], flat).painted!;
    let n = 0;
    for (let i = 0; i < m.length; i += VERTEX_FLOATS) if (Math.hypot(m[i], m[i + 1]) > 5) n++;
    return n;
  };
  // Selected alone: the area ring round the anchor. Neither selected nor
  // revealed: none.
  expect(ringInk(true)).toBeGreaterThan(0);
  expect(ringInk(false)).toBe(0);
});

test("a flash draws the Space view's very marks, its order colours at the flash's opacity", () => {
  // Same geometry at any opacity, and every order colour's alpha scaled;
  // the selection's own markers stay whole.
  const full = buildOrderOverlay([squad({ selected: true })], flat).painted!;
  const half = buildOrderOverlay([squad({ selected: true, reveal: 0.5 })], flat).painted!;
  expect(half.length).toBe(full.length);
  for (let i = 0; i < full.length; i += VERTEX_FLOATS) {
    for (let k = 0; k < 6; k++) expect(half[i + k]).toBe(full[i + k]);
    const selection = [SELECTED, SOLDIER_SELECTED].some((c) =>
      [0, 1, 2, 3].every((k) => Math.abs(full[i + 6 + k] - c[k]) < 1e-6),
    );
    expect(half[i + 9]).toBeCloseTo(selection ? full[i + 9] : full[i + 9] * 0.5, 6);
  }
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
  expect(chevronsPoint(forward.paintedMarching!)).toBe(Math.sign(Math.cos(0))); // with the facing
  expect(chevronsPoint(reverse.paintedMarching!)).toBe(-Math.sign(Math.cos(Math.PI))); // against it
  for (const m of [forward, reverse])
    for (let i = 0; i < m.paintedMarching!.length; i += VERTEX_FLOATS) {
      expect(Math.abs(m.paintedMarching![i])).toBeLessThan(10); // under the moving unit, not at (40, 0)
      expect(m.paintedMarching![i + 4]).toBe(STYLE.march.cycles_per_s);
      expect(m.paintedMarching![i + 5]).toBeCloseTo(STYLE.march.amplitude, 6);
    }
});

test("a vehicle's destination marker points its facing and shows no travel chevrons", () => {
  const dest = buildOrderOverlay(
    [squad({ ...vehicle, direction: "reverse", finalFacing: Math.PI })],
    flat,
  );
  // Its chevrons (it is moving) lie under it, none at its destination.
  for (let i = 0; i < dest.paintedMarching!.length; i += VERTEX_FLOATS)
    expect(dest.paintedMarching![i]).toBeLessThan(10);
  expect([...colours(dest)]).toEqual([key(STYLE.color)]);
  // Its facing mark reaches out from the circle toward −x.
  let tip = Infinity;
  for (let i = 0; i < dest.painted!.length; i += VERTEX_FLOATS)
    if (dest.painted![i] > 30 && Math.abs(dest.painted![i + 1]) < 0.2)
      tip = Math.min(tip, dest.painted![i]);
  expect(tip).toBeLessThan(40 - 2);
});

test("only a moving vehicle shows travel chevrons under it", () => {
  const picked = { ...vehicle, selected: true };
  const resting = buildOrderOverlay([squad({ ...picked, goal: null, route: [] })], flat);
  expect(resting.paintedMarching!.length).toBe(0);
  expect(both(resting, SELECTED)).toBeGreaterThan(0);
  const moving = buildOrderOverlay([squad(picked)], flat);
  expect(moving.paintedMarching!.length).toBeGreaterThan(0);
});

test("an order draws in one colour whatever its kind; the selection's marker in its own", () => {
  // Without cover (no pips): the order's colour, at its full and current alphas.
  const plain = buildOrderOverlay(
    [
      squad({
        memberOrders: [
          { spot: [40, 2], coverNow: null, coverThere: null },
          { spot: [40, -2], coverNow: null, coverThere: null },
        ],
      }),
    ],
    flat,
  );
  const [r, g, b, a] = STYLE.color;
  const current: Rgba = [r, g, b, a * STYLE.current_alpha];
  expect([...colours(plain)].sort()).toEqual([key(STYLE.color), key(current)].sort());
  const picked = buildOrderOverlay([squad({ selected: true })], flat);
  expect(both(picked, SELECTED)).toBeGreaterThan(0);
  expect(both(plain, SELECTED)).toBe(0);
  // A vehicle's selection marker too.
  const tank = buildOrderOverlay([squad({ ...vehicle, selected: true })], flat);
  expect(both(tank, SELECTED)).toBeGreaterThan(0);
});

test("a selected squad's circle where it stands is the selection's colour; its route and area stay the order's", () => {
  const picked = buildOrderOverlay([squad({ selected: true })], flat).painted!;
  // The squad stands at the origin, its area round (40, 0).
  const near = (x: number, y: number) => Math.hypot(x, y) < 5;
  let [ringSelected, farSelected] = [0, 0];
  for (let i = 0; i < picked.length; i += VERTEX_FLOATS) {
    const yellow = [0, 1, 2, 3].every((k) => Math.abs(picked[i + 6 + k] - SELECTED[k]) < 1e-6);
    if (!yellow) continue;
    // Off the soldiers (at y = ±1): on the circle round them.
    if (
      near(picked[i], picked[i + 1]) &&
      Math.hypot(picked[i], picked[i + 1] - 1) > 0.7 &&
      Math.hypot(picked[i], picked[i + 1] + 1) > 0.7
    )
      ringSelected++;
    if (picked[i] > 5) farSelected++;
  }
  expect(ringSelected).toBeGreaterThan(0);
  expect(farSelected).toBe(0);
});

test("a vehicle's marker ring clears its own hull, a jeep's and a tank's alike", () => {
  for (const kind of ["jeep", "tank"] as const) {
    const hull = UNITS.hull(kind)!.half_extents_m[0];
    const view = squad({ ...vehicle, hullHalfLength: hull, goal: null, route: [], selected: true });
    const mesh = buildOrderOverlay([view], flat).painted!;
    // The ring's inner edge (the nearest vertex to the centre, the
    // arrowhead's base aside) lies outside the hull's half-length.
    let inner = Infinity;
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
      inner = Math.min(inner, Math.hypot(mesh[i], mesh[i + 1]));
      expect(mesh[i + 2]).toBeLessThan(1); // on the ground, not over the hull
    }
    expect(inner).toBeGreaterThan(hull);
    expect(inner).toBeLessThan(hull + STYLE.vehicle_marker_margin_m);
  }
});

test("a squad's circle where it stands points its current facing with an arrowhead on its rim", () => {
  // The squad at the origin faces −y now; its destination lies along +x.
  const mesh = buildOrderOverlay([squad({ yaw: -Math.PI / 2 })], flat).painted!;
  const rim = (1 + 0.85) * STYLE.area_draw_scale;
  let tip = 0;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    if (Math.abs(mesh[i]) < 0.2 && mesh[i + 1] < 0) tip = Math.min(tip, mesh[i + 1]);
  expect(-tip).toBeGreaterThan(rim + 0.5); // past the rim, along −y
});

test("no route runs inside a unit's circle: a vehicle's leaves its marker's rim and ends at its destination's", () => {
  // A selected tank at the origin, facing +x, driving to (40, 0).
  const tank = buildOrderOverlay([squad({ ...vehicle, selected: true })], flat);
  const r = TANK_HALF + STYLE.vehicle_marker_margin_m;
  const color = [...STYLE.color];
  let [start, end] = [Infinity, -Infinity];
  for (let i = 0; i < tank.painted!.length; i += VERTEX_FLOATS) {
    const route = [0, 1, 2, 3].every((k) => Math.abs(tank.painted![i + 6 + k] - color[k]) < 1e-6);
    // The route's ribbon: along y = 0, half a line off it.
    if (route && Math.abs(Math.abs(tank.painted![i + 1]) - (STYLE.line_px * 0.05) / 2) < 1e-6) {
      start = Math.min(start, tank.painted![i]);
      end = Math.max(end, tank.painted![i]);
    }
  }
  // It leaves along the tank's facing (+x), so from past its marker's
  // arrowhead (its tip), never over it; not from the centre.
  expect(start).toBeCloseTo(r + Math.min(0.6 * r, 1.5), 1);
  expect(end).toBeCloseTo(40 - r, 1); // to its destination marker's rim
});

test("a squad's route runs from the edge of the circle it stands in to the edge of its area ring", () => {
  const mesh = buildOrderOverlay([squad()], flat).painted!;
  // The route runs along x from the squad (0, 0; soldiers 1 m either side)
  // to its goal (40, 0; spots 2 m either side). Its edges lie half a line
  // off the axis.
  let [start, end] = [Infinity, -Infinity];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    if (Math.abs(Math.abs(mesh[i + 1]) - (STYLE.line_px * 0.05) / 2) < 1e-6) {
      start = Math.min(start, mesh[i]);
      end = Math.max(end, mesh[i]);
    }
  // Round the soldiers, and past the circle's arrowhead: the squad faces
  // (yaw 0) the way the route leaves.
  const rim = (1 + 0.85) * STYLE.area_draw_scale;
  expect(start).toBeCloseTo(rim + Math.min(0.6 * rim, 1.5), 1);
  expect(end).toBeCloseTo(40 - 3 * STYLE.area_draw_scale, 1); // the squad's area, drawn smaller
});

test("shown, a holding squad's area is drawn once, round its anchor", () => {
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
    for (let i = 0; i < m.painted!.length; i += VERTEX_FLOATS)
      if ([0, 1, 2, 3].every((k) => Math.abs(m.painted![i + 6 + k] - current[k]) < 1e-6)) {
        const [x, y] = [m.painted![i], m.painted![i + 1]];
        // Not the markers under the soldiers themselves.
        if (holding.members.some((p) => Math.hypot(x - p[0], y - p[1]) < 2)) continue;
        radii.push(Math.hypot(x - 10, y));
      }
    return radii;
  };
  expect(ringAt(buildOrderOverlay([{ ...holding, selected: true, reveal: 0 }], flat))).toEqual([]);
  const radii = ringAt(buildOrderOverlay([holding], flat));
  expect(radii.length).toBeGreaterThan(0);
  for (const r of radii) expect(r).toBeCloseTo(14 * STYLE.area_draw_scale, 0);
});

test("a right-drag faces from the goal toward the release; a short drag sets none", () => {
  expect(dragFacing({ ground: [10, 10], facingTo: [10, 20] })).toBeCloseTo(Math.PI / 2);
  expect(dragFacing({ ground: [10, 10], facingTo: [10.3, 10] })).toBeUndefined();
  expect(dragFacing({ ground: [10, 10], facingTo: null })).toBeUndefined();
});

test("routes are drawn at the stroke rule's width for the zoom", () => {
  const route = (metresPerPx: number) => {
    const mesh = merged(
      build([squad({ memberOrders: [], members: [] })], flat, STYLE, at(metresPerPx)),
    ).painted!;
    // The route between the circles (the leg runs along x, clear of both
    // circles between x = 10 and 30): y spans its width.
    let lo = Infinity,
      hi = -Infinity;
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
      if (mesh[i] < 10 || mesh[i] > 30) continue;
      lo = Math.min(lo, mesh[i + 1]);
      hi = Math.max(hi, mesh[i + 1]);
    }
    return hi - lo;
  };
  for (const m of [1e-5, 0.05, 0.2, 1])
    expect(route(m)).toBeCloseTo(strokeWidth(STROKE_RULE, m)(STYLE.line_px), 5);
});

test("the fixture's order style is complete, and a missing colour is refused", () => {
  const broken = { ...STYLE, selected: undefined };
  expect(() => validateOrderStyle(broken as unknown as OrderStyle)).toThrow(/selected/);
});

test("a squad's area ring points its final facing with an arrowhead on its rim, no arrow beyond", () => {
  // Area round (40, 0), radius 3, drawn at the fixture's scale; facing +y.
  const mesh = buildOrderOverlay([squad({ finalFacing: Math.PI / 2 })], flat).painted!;
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

test("supply's and the consequences' colours are the fixture's, and a missing one is refused", () => {
  const overlay = village.presentation.overlay;
  const supply = validateSupplyStyle(overlay.supply as unknown as SupplyStyle);
  const consequences = validateConsequenceStyle(
    overlay.consequences as unknown as ConsequenceStyle,
  );
  expect(supply.reach).toEqual(overlay.supply.reach);
  expect(consequences.impact).toEqual(overlay.consequences.impact);
  expect(() =>
    validateSupplyStyle({ ...supply, reach: undefined } as unknown as SupplyStyle),
  ).toThrow(/reach/);
  expect(() =>
    validateConsequenceStyle({ ...consequences, impact: [1, 0, 0] } as unknown as ConsequenceStyle),
  ).toThrow(/impact/);
});

test("routes and rings take the order weight; a soldier's own markers keep their finer one", () => {
  const mpp = 0.1;
  const view = squad({ selected: true, members: [[0, 10, 0]], memberOrders: [] });
  const built = merged(build([view], flat, STYLE, at(mpp)));
  // The soldier's marker at (0, 10): its circle's inner edge is half its
  // line in from the 0.45 m radius.
  let inner = Infinity;
  for (const mesh of [built.painted!])
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
      const d = Math.hypot(mesh[i], mesh[i + 1] - 10);
      if (d < 2) inner = Math.min(inner, d);
    }
  const { stroke } = at(mpp);
  expect(inner).toBeCloseTo(0.45 - stroke(STYLE.soldier_line_px) / 2, 3);
  expect(STYLE.soldier_line_px).toBeLessThan(STYLE.line_px);
  // The route along y = 0 is the order weight wide.
  let [lo, hi] = [Infinity, -Infinity];
  for (let i = 0; i < built.painted!.length; i += VERTEX_FLOATS)
    if (built.painted![i] > 15 && built.painted![i] < 25 && Math.abs(built.painted![i + 1]) < 1) {
      lo = Math.min(lo, built.painted![i + 1]);
      hi = Math.max(hi, built.painted![i + 1]);
    }
  expect(hi - lo).toBeCloseTo(stroke(STYLE.line_px), 3);
});

test("a route leaving sideways to a squad's facing starts at its circle's rim, clear of the arrowhead", () => {
  // The squad faces +y (its arrowhead there); the route leaves along +x.
  const mesh = buildOrderOverlay([squad({ yaw: Math.PI / 2 })], flat).painted!;
  let start = Infinity;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    if (Math.abs(Math.abs(mesh[i + 1]) - (STYLE.line_px * 0.05) / 2) < 1e-6)
      start = Math.min(start, mesh[i]);
  expect(start).toBeCloseTo((1 + 0.85) * STYLE.area_draw_scale, 1);
});

test("the colour scheme is data: each role draws in its scheme's colour and layer", () => {
  const authored = village.presentation.overlay.orders as unknown as AuthoredOrderStyle;
  const yellow = validateOrderStyle(resolveOrderScheme({ ...authored, scheme: "yellow-orders" }));
  // No cover: light cover's pip shares the orders' yellow, and is paint.
  const uncovered = squad({
    selected: true,
    memberOrders: [
      { spot: [40, 2], coverNow: null, coverThere: null },
      { spot: [40, -2], coverNow: null, coverThere: null },
    ],
  });
  const built = build([uncovered], flat, yellow, at(0.05));
  const inColour = (mesh: Mesh, c: Rgba) => count(mesh, c) > 0;
  // Orders after tone mapping (overlay), in the scheme's yellow...
  expect(yellow.layers.order).toBe("overlay");
  expect(inColour(built.opaque, yellow.color)).toBe(true);
  expect(inColour(built.painted!, yellow.color)).toBe(false);
  // ...and the squad's own circle painted in the world, in its amber.
  const g = yellow.selected_glow;
  const amber: Rgba = [yellow.selected[0] * g, yellow.selected[1] * g, yellow.selected[2] * g, 1];
  expect(inColour(built.painted!, amber)).toBe(true);
  expect(() => resolveOrderScheme({ ...authored, scheme: "no-such" })).toThrow(/scheme/);
});

test("a selected vehicle's travel chevrons take the selection's colour, not the soldiers'", () => {
  // The selection is one role everywhere. The white scheme gives the two
  // roles different colours; the chevrons are always paint, so they glow.
  const authored = village.presentation.overlay.orders as unknown as AuthoredOrderStyle;
  const white = validateOrderStyle(resolveOrderScheme({ ...authored, scheme: "white-orders" }));
  expect(key(white.selected)).not.toBe(key(white.soldier_selected));
  const tank = squad({ ...vehicle, selected: true });
  const chevrons = build([tank], flat, white, at(0.05)).paintedMarching!;
  expect(chevrons.length).toBeGreaterThan(0);
  const g = white.selected_glow;
  const paint: Rgba = [white.selected[0] * g, white.selected[1] * g, white.selected[2] * g, 1];
  expect(count(chevrons, paint)).toBe(chevrons.length / VERTEX_FLOATS);
});

test("paint lies on the ground: no style carries a mark height, and every mark sits on the surface", () => {
  // One owner of how a mark clears the ground: its raster (the paint's pull,
  // the overlay's depth bias), never a lift in a style or a mesh.
  const authored = village.presentation.overlay as unknown as {
    orders: Record<string, unknown>;
    paint: Record<string, number>;
    glow: { ground: number };
  };
  expect(authored.orders).not.toHaveProperty("lift_m");
  expect(village.presentation.map_border).not.toHaveProperty("lift_m");
  expect(() =>
    validatePaintStyle({ emissive: authored.glow.ground, ...authored.paint } as PaintStyle),
  ).not.toThrow();
  expect(() =>
    validateMapBorder(village.presentation.map_border as unknown as MapBorderStyle),
  ).not.toThrow();
  // On a slope, every vertex of every mark lies on it, but for the few
  // centimetres an arrowhead or chevron stacks over the ring it meets.
  const slope = (x: number, y: number) => 3 + 0.1 * x - 0.05 * y;
  const units = [squad({ selected: true }), squad({ ...vehicle, selected: true })];
  const m = build(units, slope, STYLE, at(0.05));
  const rise: number[] = [];
  for (const mesh of [m.opaque, m.translucent, m.painted!, m.paintedMarching!])
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
      rise.push(mesh[i + 2] - slope(mesh[i], mesh[i + 1]));
  expect(rise.length).toBeGreaterThan(100);
  expect(Math.min(...rise)).toBeGreaterThan(-1e-4);
  expect(Math.max(...rise)).toBeLessThan(0.05);
  const border = buildMapBorder(
    [100, 100],
    village.presentation.map_border as unknown as MapBorderStyle,
    1,
    slope,
  );
  for (let i = 0; i < border.length; i += VERTEX_FLOATS)
    expect(border[i + 2]).toBeCloseTo(slope(border[i], border[i + 1]), 4);
});

test("every scheme is checked, not only the active one", () => {
  // A broken scheme the fixture doesn't select today still fails at load,
  // not on the day someone switches to it.
  const authored = village.presentation.overlay.orders as unknown as AuthoredOrderStyle;
  const ok = authored.schemes[authored.scheme];
  const broken = (role: Partial<(typeof ok)["order"]>) => ({
    ...authored,
    schemes: { ...authored.schemes, spare: { ...ok, soldier: { ...ok.soldier, ...role } } },
  });
  expect(() => resolveOrderScheme(broken({ layer: "sky" as never }))).toThrow(/spare/);
  expect(() => resolveOrderScheme(broken({ color: [1, 0.5] as never }))).toThrow(/spare/);
  expect(() => resolveOrderScheme(broken({ color: [2, 0, 0, 1] }))).toThrow(/spare/);
});

test("an order is the unit's: no line ever runs from a soldier to his spot", () => {
  // A holding squad whose soldiers walk to posts 6 m off: Space draws the
  // posts as markers, and no ribbon between a soldier and his post.
  const holding = squad({
    goal: null,
    state: "idle",
    route: [],
    members: [
      [0, 0, 0],
      [0, 8, 0],
    ],
    memberOrders: [
      { spot: [6, 0], coverNow: null, coverThere: null },
      { spot: [6, 8], coverNow: null, coverThere: null },
    ],
    area: { anchor: [3, 4], radius: 8 },
  });
  const mesh = buildOrderOverlay([holding], flat).painted!;
  // Nothing drawn half way between a soldier and his post.
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    for (const mid of [
      [3, 0],
      [3, 8],
    ])
      expect(Math.hypot(mesh[i] - mid[0], mesh[i + 1] - mid[1])).toBeGreaterThan(1);
});

test("with Space, the route polylines are exactly the units with an order, one each", () => {
  // Two units with orders (a squad, a vehicle), a holding squad whose
  // soldiers walk to posts 6 m off, and a vehicle at rest.
  const units = [
    squad(),
    squad({ ...vehicle, position: [0, 40, 0], goal: [40, 40], route: [[40, 40]] }),
    squad({
      position: [0, 80, 0],
      goal: null,
      state: "idle",
      route: [],
      members: [
        [0, 80, 0],
        [0, 88, 0],
      ],
      memberOrders: [
        { spot: [6, 80], coverNow: null, coverThere: null },
        { spot: [6, 88], coverNow: null, coverThere: null },
      ],
      area: { anchor: [3, 84], radius: 8 },
    }),
    squad({ ...vehicle, position: [0, 120, 0], goal: null, route: [] }),
  ];
  const mesh = buildOrderOverlay(units, flat).painted!;
  // The order colour's triangles, joined where they share a vertex: each
  // route is one chain of ribbon quads; a ring closes on itself round its
  // centre, a marker or an arrowhead is small.
  const colour = STYLE.color;
  const key = (i: number) => `${mesh[i].toFixed(4)},${mesh[i + 1].toFixed(4)}`;
  const parent = new Map<string, string>();
  const find = (a: string): string => {
    while (parent.get(a) !== a) a = parent.get(a)!;
    return a;
  };
  const join = (a: string, b: string) => parent.set(find(a), find(b));
  for (let t = 0; t < mesh.length; t += 3 * VERTEX_FLOATS) {
    // The order colour at any of its alphas (current, queued).
    const ours = [0, 1, 2].every((k) => Math.abs(mesh[t + 6 + k] - colour[k]) < 1e-6);
    if (!ours) continue;
    const ks = [0, 1, 2].map((v) => key(t + v * VERTEX_FLOATS));
    for (const k of ks) if (!parent.has(k)) parent.set(k, k);
    join(ks[0], ks[1]);
    join(ks[1], ks[2]);
  }
  const parts = new Map<string, [number, number][]>();
  for (const k of parent.keys()) {
    const root = find(k);
    const [x, y] = k.split(",").map(Number);
    parts.set(root, [...(parts.get(root) ?? []), [x, y]]);
  }
  // A route: long (over 5 m end to end) and not a ring (its points don't all
  // lie near one distance from their centre).
  const routes = [...parts.values()].filter((pts) => {
    const c = pts.reduce((a, p) => [a[0] + p[0] / pts.length, a[1] + p[1] / pts.length], [0, 0]);
    const d = pts.map((p) => Math.hypot(p[0] - c[0], p[1] - c[1]));
    return Math.max(...d) > 2.5 && Math.min(...d) < 0.5 * Math.max(...d);
  });
  const withOrders = units.filter((u) => u.goal).length;
  expect(routes.length).toBe(withOrders);
});
