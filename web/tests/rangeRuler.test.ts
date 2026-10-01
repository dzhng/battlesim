// @vitest-environment node
// The range ruler's pure parts: which selected unit it measures from, the
// distance as the range check measures it, and where each reach ends.
import { expect, test } from "vitest";
import village from "@fixtures/village.json";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import {
  buildRangeRuler,
  validateRulerStyle,
  type RulerStyle,
} from "@packages/battle-renderer/src/rangeRulerOverlay";
import { circleReach } from "@packages/battle-renderer/src/orderOverlay";
import { strokeWidth, validateStrokeRule } from "@packages/battle-renderer/src/strokeWidth";
import { rulerLine } from "@apps/battle-lab/src/pointerPaint";
import { villageRulerStyle } from "@apps/battle-lab/src/villageOverlay";
import { closestUnit, rangeRuler, type RulerRules } from "../src/battle/present/rangeRuler";

/** The marks' stroke widths where one pixel spans `m` metres. */
const stroke = (m: number) =>
  strokeWidth(validateStrokeRule(village.presentation.overlay.stroke), m);

const rules = village as unknown as RulerRules;
const { infantry_muzzle_m: muzzle, infantry_aim_m: aim } = village.physics;
const range = (row: keyof typeof village.weapons) => village.weapons[row].range_m;
const at = (kind: string, position: [number, number, number], id = 1) => ({ id, kind, position });

test("the ruler measures from the selected unit nearest the cursor across the ground", () => {
  const near = at("rifle", [100, 0, 80], 1);
  const far = at("rifle", [0, 0, 0], 2);
  // Nearer across the ground though higher up: height does not choose.
  expect(closestUnit([far, near], [120, 0])?.id).toBe(1);
  expect(closestUnit([far, near], [-5, 0])?.id).toBe(2);
  expect(closestUnit([], [0, 0])).toBe(null);
});

test("the ruler estimates 3D reach from the first mount's height to the aim point", () => {
  const squad = at("rifle", [0, 0, 10]);
  const flat = rangeRuler(squad, [300, 400, 10], rules, UNITS);
  expect(flat.distance_m).toBeCloseTo(Math.hypot(500, muzzle - aim), 6);
  // Up a 120 m rise, the range grows with the height.
  const up = rangeRuler(squad, [300, 400, 130], rules, UNITS);
  expect(up.distance_m).toBeCloseTo(Math.hypot(500, 120 + aim - muzzle), 6);
  // A vehicle's muzzle stands at its gun's height.
  const tank = UNITS.type("tank").mounts[0];
  const gun = tank.pivot_m[2] + tank.muzzle_m![2];
  expect(rangeRuler(at("tank", [0, 0, 0]), [0, 900, 0], rules, UNITS).distance_m).toBeCloseTo(
    Math.hypot(900, gun - aim),
    6,
  );
});

test("each reach of the unit's weapons is one mark, nearest first; rows sharing a range share it", () => {
  const ruler = rangeRuler(at("tank", [0, 0, 0]), [1000, 0, 0], rules, UNITS);
  expect(ruler.marks.map((m) => [m.names, m.range_m])).toEqual([
    [[village.weapons.hmg.name], range("hmg")],
    [[UNITS.type("tank").mounts[0].name], range("tank_ap")],
  ]);
  expect(rangeRuler(at("supply", [0, 0, 0]), [10, 0, 0], rules, UNITS).marks).toEqual([]);
});

test("a weapon the cursor is past ends on the line where its 3D range runs out; one that reaches it is in range", () => {
  const from: [number, number, number] = [0, 0, 20];
  const cursor: [number, number, number] = [700, 0, 60];
  const ruler = rangeRuler(at("rifle", from), cursor, rules, UNITS);
  const [grenade, rifle] = ruler.marks;
  expect(rifle.inRange && grenade.inRange).toBe(false);
  expect(ruler.distance_m).toBeGreaterThan(range("rifle"));
  for (const mark of [grenade, rifle]) {
    // The point on the muzzle-to-aim segment over the tick is exactly the range away.
    const t = mark.along_m! / 700;
    const dz = (cursor[2] + aim - (from[2] + muzzle)) * t;
    expect(Math.hypot(mark.along_m!, dz)).toBeCloseTo(mark.range_m, 6);
  }
  const between = (range("rifle") + range("grenade")) / 2;
  const inside = rangeRuler(at("rifle", from), [between, 0, 20], rules, UNITS);
  expect(inside.marks.map((m) => [m.inRange, m.along_m])).toEqual([
    [false, expect.closeTo((between * range("grenade")) / inside.distance_m, 6)],
    [true, null],
  ]);
});

test("the painted line is lit up to the farthest reach short of the cursor, a tick where each ends", () => {
  // Between the two ranges, the cannon reaches and the HMG does not.
  const between = (range("hmg") + range("tank_ap")) / 2;
  const tank = rulerLine(rangeRuler(at("tank", [0, 0, 0]), [between, 0, 0], rules, UNITS), null);
  expect(tank.reach_m).toBeCloseTo(between, 6);
  expect(tank.ticks).toHaveLength(1);
  expect(tank.ticks[0]).toBeGreaterThan(range("hmg") - 1);
  expect(tank.ticks[0]).toBeLessThan(range("hmg"));
  // No rifle reaches 700 m: lit to the rifle's tick, the rest beyond.
  const squad = rangeRuler(at("rifle", [0, 0, 0]), [700, 0, 0], rules, UNITS);
  expect(rulerLine(squad, null).reach_m).toBe(squad.marks[1].along_m);
});

test("the ruler's paint lies on the ground, in the reach colour up to the reach and beyond it after", () => {
  const style = villageRulerStyle;
  expect(() =>
    validateRulerStyle({ ...style, beyond: [1, 0, 0] } as unknown as RulerStyle),
  ).toThrow(/beyond/);
  const slope = (x: number, y: number) => 3 + 0.1 * x - 0.05 * y;
  const line = {
    from: [0, 0] as const,
    to: [100, 0] as const,
    start_m: 0,
    reach_m: 60,
    ticks: [60, 30],
  };
  const mesh = buildRangeRuler(line, slope, style, 0.05, stroke(0.05));
  const is = (i: number, c: readonly number[]) =>
    c.every((v, k) => Math.abs(mesh[i + 6 + k] - v) < 1e-6);
  let reach = 0,
    beyond = 0;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
    expect(mesh[i + 2]).toBeCloseTo(slope(mesh[i], mesh[i + 1]), 4);
    // The end ring aside, the colour changes at the reach.
    if (Math.hypot(mesh[i] - 100, mesh[i + 1]) < 2) continue;
    if (is(i, style.reach)) reach = Math.max(reach, mesh[i]);
    if (is(i, style.beyond)) {
      beyond++;
      expect(mesh[i]).toBeGreaterThanOrEqual(60 - 1e-6);
    }
  }
  expect(reach).toBeCloseTo(60, 6);
  expect(beyond).toBeGreaterThan(0);
});

test("the painted line leaves the unit's circle at its border, whatever the circle's centre", () => {
  const tank = rangeRuler(at("tank", [0, 0, 0]), [1000, 0, 0], rules, UNITS);
  // Leaving across its facing: at the rim; along it: at the arrowhead's tip.
  expect(rulerLine(tank, { c: [0, 0], r: 5, facing: Math.PI / 2 }).start_m).toBeCloseTo(5, 6);
  expect(rulerLine(tank, { c: [0, 0], r: 5, facing: 0 }).start_m).toBeCloseTo(
    circleReach({ r: 5, facing: 0 }, 0),
    6,
  );
  expect(circleReach({ r: 5, facing: 0 }, 0)).toBeGreaterThan(5);
  // A holding squad's area ring round its anchor, off the squad's centre.
  const squad = rangeRuler(at("rifle", [0, 0, 0]), [0, 300, 0], rules, UNITS);
  expect(rulerLine(squad, { c: [0, 2], r: 10, facing: null }).start_m).toBeCloseTo(12, 6);
  // No circle drawn round the unit: from the unit itself.
  expect(rulerLine(squad, null).start_m).toBe(0);
  // The cursor inside the circle: nothing of the line is left to draw.
  const inside = rangeRuler(at("rifle", [0, 0, 0]), [4, 0, 0], rules, UNITS);
  expect(rulerLine(inside, { c: [0, 0], r: 10, facing: null }).start_m).toBeGreaterThanOrEqual(4);
});

test("the painted line never enters the unit's circle or the ring at the cursor", () => {
  const style = villageRulerStyle;
  const flat = () => 0;
  const metresPerPx = 0.05;
  const line = {
    from: [0, 0] as const,
    to: [100, 0] as const,
    start_m: 8,
    reach_m: 60,
    ticks: [60],
  };
  const mesh = buildRangeRuler(line, flat, style, metresPerPx, stroke(metresPerPx));
  const endR = style.end_px * metresPerPx;
  const half = stroke(metresPerPx)(style.line_px) / 2;
  let nearest = Infinity;
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
    const [x, y] = [mesh[i], mesh[i + 1]];
    expect(Math.hypot(x, y)).toBeGreaterThanOrEqual(8 - 1e-4);
    nearest = Math.min(nearest, Math.hypot(x - 100, y));
  }
  // The ring's own inner edge is the nearest paint to the cursor.
  expect(nearest).toBeCloseTo(endR - half, 4);
  // Wholly inside the unit's circle: only the ring at the cursor is drawn.
  const inside = buildRangeRuler(
    { ...line, start_m: 120 },
    flat,
    style,
    metresPerPx,
    stroke(metresPerPx),
  );
  expect(inside.length).toBeGreaterThan(0);
  for (let i = 0; i < inside.length; i += VERTEX_FLOATS)
    expect(Math.hypot(inside[i] - 100, inside[i + 1])).toBeGreaterThan(endR - half - 1e-4);
});
