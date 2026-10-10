// @vitest-environment node
// One outlined, filled and hatched glyph for every source, using only the
// report's area and shared presented opacity.
import { vec3, type Vec3 } from "math";
import { expect, test } from "vitest";
import { buildContactGlyphs, type ContactShape } from "@packages/battle-renderer/src/contactGlyph";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import type { PresentedContact } from "@web/battle/present/contactPresentation";
import { contactLayer } from "@apps/battle-lab/src/battleOverlay";
import { gameContactStyle } from "@apps/battle-lab/src/gameFog";
import { gameStroke } from "@apps/battle-lab/src/gameOverlay";

const flat = () => 0;
const style = gameContactStyle;

function vertices(mesh: Float32Array) {
  const out: { x: number; y: number; rgba: number[] }[] = [];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push({ x: mesh[i], y: mesh[i + 1], rgba: [...mesh.subarray(i + 6, i + 10)] });
  return out;
}

const shape = (over: Partial<ContactShape> = {}): ContactShape => ({
  center: [400, 300],
  radius: 100,
  opacity: 1,
  aloft: false,
  ...over,
});

const maxAlpha = (c: ContactShape) =>
  Math.max(
    ...vertices(buildContactGlyphs([c], flat, style, facing()).translucent).map((v) => v.rgba[3]),
  );

test("a glyph uses the shared presented opacity and disappears at zero", () => {
  const full = maxAlpha(shape());
  expect(maxAlpha(shape({ opacity: 0.5 }))).toBeCloseTo(full / 2);
  expect(maxAlpha(shape({ opacity: 0.01 }))).toBeCloseTo(full / 100);
  const gone = buildContactGlyphs([shape({ opacity: 0 })], flat, style, facing());
  expect(gone.translucent.length).toBe(0);
  expect(gone.opaque.length).toBe(0);
});

test("the pale outline follows report life independently of hatch ink", () => {
  for (const opacity of [1, 0.5, 0.05]) {
    for (const hatch_alpha of [0, 0.15, 0.8]) {
      const fixed = { ...style, outline_color: [1, 1, 1] as [number, number, number], hatch_alpha };
      const outline = vertices(
        buildContactGlyphs([shape({ opacity })], flat, fixed, facing()).translucent,
      ).filter((v) => v.rgba.slice(0, 3).every((channel) => channel > 0.99));
      expect(outline.length).toBeGreaterThan(0);
      for (const vertex of outline) expect(vertex.rgba[3]).toBeCloseTo(opacity);
    }
  }
});

test("every glyph is red through its middle and hatched; every report keeps a pale outline", () => {
  const isRed = (c: number[]) => c[0] > c[1] + 0.3 && c[0] > c[2] + 0.3 && c[3] > 0;
  const isPale = (c: number[]) => Math.min(c[0], c[1], c[2]) > 0.8 && c[3] > 0;
  const from = (v: { x: number; y: number }) => Math.hypot(v.x - 400, v.y - 300);
  const glyphFor = (source: string) => {
    return vertices(contactLayer([{ ...report, source }], flat, facing()).translucent);
  };
  const lastSeen = glyphFor("last_seen"),
    firing = glyphFor("firing");
  for (const glyph of [lastSeen, firing]) {
    // Red across the disc, its centre too: a fill, not only a rim.
    expect(glyph.some((v) => isRed(v.rgba) && from(v) < 1)).toBe(true);
    // Nothing pale inside: the hatch is red too.
    expect(glyph.some((v) => isPale(v.rgba) && from(v) < 100 - style.outline_width_m - 1e-3)).toBe(
      false,
    );
  }
  // Every source uses the same pale outline.
  expect(lastSeen.some((v) => isPale(v.rgba) && from(v) > 99)).toBe(true);
  expect(firing.some((v) => isPale(v.rgba) && from(v) > 99)).toBe(true);
  // The hatch: many parallel strips across the disc, not only rings.
  const across = (v: { x: number; y: number }) => (v.y - v.x) / Math.SQRT2;
  // The hatch shares the glow's colour; it is told by its own alpha.
  const hatched = lastSeen.filter(
    (v) =>
      v.rgba.slice(0, 3).every((c, k) => Math.abs(c - style.color[k]) < 1e-6) &&
      Math.abs(v.rgba[3] - style.hatch_alpha) < 1e-6 &&
      from(v) < 80,
  );
  const lines = new Set(hatched.map((v) => Math.round(across(v) / style.hatch_spacing_m)));
  expect(lines.size).toBeGreaterThanOrEqual(Math.floor((0.8 * 160) / style.hatch_spacing_m));
});

test("a glyph stays inside its area, whatever the contact's radius", () => {
  for (const radius of [20, 100]) {
    const verts = vertices(
      buildContactGlyphs([shape({ radius })], flat, style, facing()).translucent,
    );
    const reach = Math.max(...verts.map((v) => Math.hypot(v.x - 400, v.y - 300)));
    expect(reach).toBeLessThanOrEqual(radius * 1.1);
    expect(reach).toBeGreaterThan(radius * 0.95);
  }
});

// The airborne sign (D11, slice 13): an aloft contact hangs at its height as
// a disc facing the camera, the ground glyph's recipe with a dark keyline.

/** A camera looking north at 40° down: screen right is east, screen up is
 *  north tipped up toward the sky. */
const pitch = (40 * Math.PI) / 180;
const facing = (metresPerPx = 0.1) => ({
  right: [1, 0, 0] as const,
  up: [0, Math.sin(pitch), Math.cos(pitch)] as const,
  stroke: gameStroke(metresPerPx),
});
/** Toward the camera: the disc's normal. */
const toEye = [0, -Math.cos(pitch), Math.sin(pitch)];
const ALOFT = { center: [400, 300, 30] as const, radius: 20, opacity: 1, aloft: true };
const glyphVerts = (c: ContactShape, metresPerPx = 0.1) =>
  vertices3(buildContactGlyphs([c], flat, style, facing(metresPerPx)).translucent);

function vertices3(mesh: Float32Array) {
  const out: { p: number[]; n: number[]; rgba: number[] }[] = [];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push({
      p: [...mesh.subarray(i, i + 3)],
      n: [...mesh.subarray(i + 3, i + 6)],
      rgba: [...mesh.subarray(i + 6, i + 10)],
    });
  return out;
}
const off = (p: number[]) => [p[0] - 400, p[1] - 300, p[2] - 30];
type V3 = { p: number[]; n: number[]; rgba: number[] };
/** The stem is the rim's pale colour at the stem's alpha; the rest is the sign. */
const isStem = (v: V3) =>
  Math.min(...v.rgba.slice(0, 3)) > 0.8 && Math.abs(v.rgba[3] - style.air.stem_alpha) < 1e-6;
const signOf = (verts: V3[]) => verts.filter((v) => !isStem(v));
const stemOf = (verts: V3[]) => verts.filter(isStem);
/** The sign's visible circle: its pale rim's outer radius. */
const rimOf = (verts: V3[]) =>
  Math.max(
    ...signOf(verts)
      .filter((v) => Math.min(...v.rgba.slice(0, 3)) > 0.8)
      .map((v) => Math.hypot(...off(v.p))),
  );

test("an aloft contact floats at its height as a disc facing the camera", () => {
  const verts = signOf(glyphVerts(ALOFT));
  expect(verts.length).toBeGreaterThan(0);
  // Every vertex lies in the plane through the centre facing the eye: a
  // perfect circle on screen, at the contact's height, not on the ground.
  for (const v of verts)
    expect(Math.abs(vec3.dot(off(v.p) as Vec3, toEye as Vec3))).toBeLessThan(1e-3);
  expect(Math.min(...verts.map((v) => v.p[2]))).toBeGreaterThan(30 - 20 * 1.2);
  // Lit as the ground glyph is (its normal is the ground's), so the two
  // read as one red.
  for (const v of verts) expect(v.n).toEqual([0, 0, 1]);
});

test("a thin stem stands from the ground under the sign up to its rim", () => {
  // 0.1 m a pixel: the sign is held at its largest on screen.
  const radius = gameStroke(0.1)(style.air.max_radius_px);
  const stem = stemOf(glyphVerts(ALOFT, 0.1));
  expect(stem.length).toBeGreaterThan(0);
  const half = gameStroke(0.1)(style.air.stem_px) / 2;
  for (const v of stem) expect(Math.hypot(v.p[0] - 400, v.p[1] - 300)).toBeLessThan(half * 1.5);
  const zs = stem.map((v) => v.p[2]);
  expect(Math.min(...zs)).toBeCloseTo(0, 5);
  // It stops where it meets the rim on screen, rather than piercing the sign.
  expect(Math.max(...zs)).toBeCloseTo(30 - radius / Math.cos(pitch), 5);
  // Over its own ground point (a sign as wide as its height), or seen from
  // straight above, there is no stem to draw.
  expect(stemOf(glyphVerts({ ...ALOFT, radius: 30 }, 1))).toEqual([]);
  const above = { ...facing(0.1), up: [0, 1, 0] as const };
  expect(stemOf(vertices3(buildContactGlyphs([ALOFT], flat, style, above).translucent))).toEqual(
    [],
  );
});

test("a contact not aloft lies on the ground whatever its height or the camera", () => {
  const low = { ...ALOFT, aloft: false };
  const close = buildContactGlyphs([low], flat, style, facing()).translucent;
  const far = buildContactGlyphs([low], flat, style, { ...facing(5), right: [0, 1, 0] });
  expect(far.translucent).toEqual(close);
  for (const v of vertices3(close)) expect(v.p[2]).toBeCloseTo(style.lift_m, 5);
});

test("the sign spans the contact's area, held between its smallest and largest on screen", () => {
  // Where the area is between the two on screen, it sizes the sign, as it
  // sizes the ground glyph: 20 m at a metre a pixel.
  expect(rimOf(glyphVerts(ALOFT, 1))).toBeCloseTo(20, 3);
  // Far out, where the area would be a speck, it holds its minimum.
  expect(rimOf(glyphVerts(ALOFT, 5))).toBeCloseTo(gameStroke(5)(style.air.min_radius_px), 3);
  // Close in, it never grows to hide the ground it marks.
  expect(rimOf(glyphVerts(ALOFT, 0.05))).toBeCloseTo(gameStroke(0.05)(style.air.max_radius_px), 3);
  // Like every mark's stroke, it is smaller on screen from the map's zoom.
  expect(rimOf(glyphVerts(ALOFT, 0.4)) / 0.4).toBeLessThan(rimOf(glyphVerts(ALOFT, 0.05)) / 0.05);
  // Nothing of it (keyline, glow) reaches far past its rim.
  const reach = Math.max(...signOf(glyphVerts(ALOFT, 1)).map((v) => Math.hypot(...off(v.p))));
  expect(reach).toBeLessThanOrEqual(20 * 1.15);
});

test("the sign keeps the ground glyph's recipe and adds a dark keyline outside its pale rim", () => {
  // At a metre a pixel the sign is its area's size, so its hatch is the
  // ground glyph's own.
  const verts = signOf(glyphVerts(ALOFT, 1));
  const r = (v: { p: number[] }) => Math.hypot(...off(v.p));
  const isRed = (c: number[]) => c[0] > c[1] + 0.3 && c[0] > c[2] + 0.3 && c[3] > 0;
  const isPale = (c: number[]) => Math.min(c[0], c[1], c[2]) > 0.8 && c[3] > 0;
  const isDark = (c: number[]) => Math.max(c[0], c[1], c[2]) < 0.05 && c[3] > 0;
  expect(verts.some((v) => isRed(v.rgba) && r(v) < 1)).toBe(true);
  const pale = verts.filter((v) => isPale(v.rgba)).map(r);
  const dark = verts.filter((v) => isDark(v.rgba)).map(r);
  expect(pale.length).toBeGreaterThan(0);
  expect(dark.length).toBeGreaterThan(0);
  expect(Math.min(...dark)).toBeGreaterThanOrEqual(Math.max(...pale) - 1e-3);
  // The hatch: parallel strips at 45° on screen, red, across the disc.
  const hatched = verts.filter(
    (v) => Math.abs(v.rgba[3] - style.air.hatch_alpha) < 1e-6 && isRed(v.rgba),
  );
  const across = (v: { p: number[] }) => {
    const o = off(v.p);
    const [x, y] = [
      vec3.dot(o as Vec3, facing().right as Vec3),
      vec3.dot(o as Vec3, facing().up as Vec3),
    ];
    return (y - x) / Math.SQRT2;
  };
  const lines = new Set(hatched.map((v) => Math.round(across(v) / style.hatch_spacing_m)));
  expect(lines.size).toBeGreaterThanOrEqual(5);
});

const report: PresentedContact = {
  id: 7,
  source: "last_seen",
  center: [400, 300, 0],
  layer: "ground",
  aloft: false,
  radius: 100,
  evidenceTick: 150,
  expiresTick: 390,
  opacity: 1,
  retiring: false,
  kind: "test_tank",
  heard: [],
};

test("a glyph draws only the reported area and opacity, never identity or remembered type", () => {
  const plain = contactLayer([report], flat, facing());
  const renamed = { ...report, id: 99, kind: null, heard: ["rifle"] };
  expect(contactLayer([renamed], flat, facing()).translucent).toEqual(plain.translucent);
});
