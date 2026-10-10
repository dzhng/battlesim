// @vitest-environment node
// Where an aircraft is over the ground (D18): its marker lies on the ground
// under it as paint, and a drop line joins it to the airframe in the
// overlay. Every aircraft the side sees carries them, its own and an
// identified enemy's, whatever it is doing; a ground unit's marker stays
// contextual.
import { expect, test } from "vitest";
import { VERTEX_FLOATS, type Mesh, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { ObservationView, OwnUnitView, IdentifiedView } from "@web/battle/sim/observation";
import { aircraftLayer, orderLayer } from "@apps/battle-lab/src/battleOverlay";
import { lightened } from "@packages/battle-renderer/src/orderOverlay";
import {
  gameOrderStyle as STYLE,
  gameStroke,
  OPENING_METRES_PER_PX,
} from "@apps/battle-lab/src/gameOverlay";
import { gameHud } from "@web/battle/present/hudTheme";
import { UNITS } from "./catalog";
import game from "../../fixtures/game.json";

const flat = () => 0;
const HEIGHT = 20;
const AT: [number, number, number] = [100, 50, HEIGHT];
const glow = (c: Rgba, g: number): Rgba => [c[0] * g, c[1] * g, c[2] * g, c[3]];
const SELECTED = glow(STYLE.selected, STYLE.glow.selected);
const ENEMY = glow([...gameHud.enemy, 1], STYLE.glow.order);
const HULL = UNITS.hull("test_heli")!.half_extents_m[0];

const own = (u: Partial<OwnUnitView>) =>
  ({
    id: 1,
    kind: "test_heli",
    position: AT,
    yaw: 0,
    goal: null,
    state: "idle",
    route: [],
    queue: [],
    members: [],
    memberOrders: [],
    area: null,
    garrison: null,
    finalFacing: 0,
    direction: null,
    ...u,
  }) as OwnUnitView;
const seen = (u: Partial<IdentifiedView>) =>
  ({ id: 9, kind: "test_heli", position: AT, yaw: 0, members: [], ...u }) as IdentifiedView;
const view = (o: Partial<ObservationView>) =>
  ({ own: [], identified: [], ...o }) as unknown as ObservationView;

const vertices = (mesh: Mesh | undefined) => {
  const out: { p: number[]; c: number[] }[] = [];
  if (!mesh) return out;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push({
      p: Array.from(mesh.slice(i, i + 3)),
      c: Array.from(mesh.slice(i + 6, i + 10)),
    });
  return out;
};
/** `c` lightened as the dots are, by the overlay's own rule. */
const asDots = (c: Rgba): Rgba => [...lightened(c, STYLE.drop_line_lighten), c[3]];
const sameRgb = (a: readonly number[], b: readonly number[]) =>
  [0, 1, 2].every((k) => Math.abs(a[k] - b[k]) < 1e-5);

/** The drop line's dots, bottom up: each one's height span and opacity. */
function dotsOf(line: { p: number[]; c: number[] }[]) {
  const dots: { from: number; to: number; alpha: number }[] = [];
  // A dot is a closed tube: six quads, two triangles each.
  for (let i = 0; i < line.length; i += 36) {
    const z = line.slice(i, i + 36).map((v) => v.p[2]);
    dots.push({ from: Math.min(...z), to: Math.max(...z), alpha: line[i].c[3] });
  }
  return dots.sort((a, b) => a.from - b.from);
}

/** The ground ring's radius span round `c`, and the drop line's extent. */
function marks(layer: { painted?: Mesh; opaque: Mesh }, c = AT) {
  const ring = vertices(layer.painted).map((v) => Math.hypot(v.p[0] - c[0], v.p[1] - c[1]));
  const line = vertices(layer.opaque);
  return {
    ring,
    ringZ: vertices(layer.painted).map((v) => v.p[2]),
    line,
    lineZ: line.map((v) => v.p[2]),
    lineOff: line.map((v) => Math.hypot(v.p[0] - c[0], v.p[1] - c[1])),
  };
}

test("an own aircraft, neither selected nor ordered, still shows its ground marker and a drop line to it", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat));
  // The marker lies on the ground, round the point under the airframe, the
  // size a vehicle's marker is from its hull.
  expect(m.ring.length).toBeGreaterThan(0);
  expect(Math.max(...m.ringZ)).toBeLessThan(0.05);
  expect(Math.max(...m.ring)).toBeGreaterThan(HULL);
  // The drop line rises from the ring's centre, straight up, and fades out
  // well short of the airframe: it points up at it, it doesn't tie on.
  expect(Math.min(...m.lineZ)).toBeCloseTo(0, 5);
  expect(Math.max(...m.lineZ)).toBeLessThanOrEqual(HEIGHT * STYLE.drop_line_reach + 1e-6);
  expect(Math.max(...m.lineZ)).toBeGreaterThan(HEIGHT * STYLE.drop_line_reach * 0.8);
  expect(Math.max(...m.lineOff)).toBeLessThan(0.5);
  // It is dotted: separate dots with gaps between them, not one stroke.
  const dots = dotsOf(m.line);
  expect(dots.length).toBeGreaterThan(3);
  for (let i = 1; i < dots.length; i++) expect(dots[i].from).toBeGreaterThan(dots[i - 1].to);
  // Full at the ground, fading out as it rises: the marker is quiet, the
  // dots keep their own opacity over pale ground.
  expect(dots[0].alpha).toBeCloseTo(STYLE.drop_line_alpha, 5);
  expect(dots.at(-1)!.alpha).toBeLessThan(STYLE.drop_line_alpha * 0.25);
  for (let i = 1; i < dots.length; i++) expect(dots[i].alpha).toBeLessThan(dots[i - 1].alpha);
});

test("the dots are the marker's hue at full brightness, lightened toward white", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat));
  const quiet = glow(STYLE.color, STYLE.glow.order);
  const [r, g, b] = m.line[0].c;
  // Paler than the quiet marker's colour, never another hue: each channel
  // sits the style's share of the way from the hue at full brightness to
  // white, so the channels keep their order.
  const k = 1 / Math.max(quiet[0], quiet[1], quiet[2]);
  const w = STYLE.drop_line_lighten;
  for (const [i, c] of [r, g, b].entries())
    expect(c).toBeCloseTo(quiet[i] * k + (1 - quiet[i] * k) * w, 5);
  expect(Math.max(r, g, b)).toBeCloseTo(1, 5);
  expect(r + g + b).toBeGreaterThan(quiet[0] + quiet[1] + quiet[2]);
});

test("the drop line's dots are lit as marks on the ground, not as solids", () => {
  const layer = orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat);
  const normals = new Set<string>();
  for (let i = 0; i < layer.opaque.length; i += VERTEX_FLOATS)
    normals.add(Array.from(layer.opaque.slice(i + 3, i + 6)).join());
  expect([...normals]).toEqual(["0,0,1"]);
});

test("however close the camera, an aircraft's dots stay few", () => {
  const close = orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat, 0.001);
  expect(dotsOf(marks(close).line).length).toBeLessThanOrEqual(64);
});

test("a ground vehicle neither selected nor ordered still shows nothing", () => {
  const jeep = own({ kind: "test_jeep", position: [100, 50, 0] });
  const layer = orderLayer(UNITS, view({ own: [jeep] }), [], new Map(), flat);
  expect(layer.painted?.length ?? 0).toBe(0);
  expect(layer.opaque.length).toBe(0);
});

test("selected, an aircraft's marker and its drop line both take the selection's colour", () => {
  const m = marks(orderLayer(UNITS, view({ own: [own({})] }), [1], new Map(), flat));
  expect(m.line.length).toBeGreaterThan(0);
  const dotColour = asDots(SELECTED);
  for (const v of m.line) expect(sameRgb(v.c, dotColour)).toBe(true);
  const layer = orderLayer(UNITS, view({ own: [own({})] }), [1], new Map(), flat);
  for (const v of vertices(layer.painted)) expect(sameRgb(v.c, SELECTED)).toBe(true);
});

test("an identified enemy aircraft shows its marker and drop line in the enemy's colour; a ground enemy none", () => {
  const o = view({
    identified: [seen({}), seen({ id: 10, kind: "test_tank", position: [200, 50, 0] })],
  });
  const layer = aircraftLayer(UNITS, o, flat);
  const m = marks(layer);
  expect(Math.max(...m.ring)).toBeLessThan(HULL + 3);
  expect(Math.max(...m.lineZ)).toBeGreaterThan(HEIGHT * STYLE.drop_line_reach * 0.8);
  for (const v of vertices(layer.painted)) expect(sameRgb(v.c, ENEMY)).toBe(true);
  const dotColour = asDots(ENEMY);
  for (const v of m.line) expect(sameRgb(v.c, dotColour)).toBe(true);
});

test("the drop line keeps its weight on screen as the camera pulls out, thinning by the stroke rule", () => {
  const width = (metresPerPx: number) => {
    const layer = orderLayer(UNITS, view({ own: [own({})] }), [], new Map(), flat, metresPerPx);
    return 2 * Math.max(...marks(layer).lineOff);
  };
  // A square tube: its diagonal is √2 × its width. Every mark's stroke
  // comes from the one rule, the drop line's too.
  for (const metresPerPx of [OPENING_METRES_PER_PX, 0.4])
    expect(width(metresPerPx) / Math.SQRT2).toBeCloseTo(
      gameStroke(metresPerPx)(STYLE.drop_line_px),
      5,
    );
  // Pulled out to the map, it is still wider on the ground than close up.
  expect(width(0.4)).toBeGreaterThan(width(OPENING_METRES_PER_PX));
});

test("with Space held, an aircraft's ghost where its orders end is joined to that end's marker by a drop line", () => {
  const GOAL: [number, number] = [200, 50];
  const ordered = own({ goal: GOAL, state: "moving", route: [GOAL], finalFacing: 0 });
  const o = view({ own: [ordered] });
  const reveal = new Map([[1, 1]]);
  /** Drop-line vertices standing over the goal, by their heights. */
  const overGoal = (ghosts: boolean) =>
    vertices(orderLayer(UNITS, o, [], reveal, flat, undefined, ghosts).opaque)
      .filter((v) => Math.hypot(v.p[0] - GOAL[0], v.p[1] - GOAL[1]) < 0.5)
      .map((v) => v.p[2]);
  const held = overGoal(true);
  expect(Math.min(...held)).toBeCloseTo(0, 5);
  expect(Math.max(...held)).toBeGreaterThan(game.air.cruise_agl_m * STYLE.drop_line_reach * 0.8);
  expect(Math.max(...held)).toBeLessThanOrEqual(
    game.air.cruise_agl_m * STYLE.drop_line_reach + 1e-6,
  );
  // An order's flash shows no ghost, so no line stands over its end.
  expect(overGoal(false)).toEqual([]);
});
