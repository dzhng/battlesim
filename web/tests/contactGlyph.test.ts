// @vitest-environment node
// One outlined, filled and hatched glyph for every source, using only the
// report's area and shared presented opacity.
import { expect, test } from "vitest";
import { buildContactGlyphs, type ContactShape } from "@packages/battle-renderer/src/contactGlyph";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import type { PresentedContact } from "@web/battle/present/contactPresentation";
import { contactLayer } from "@apps/battle-lab/src/battleOverlay";
import { gameContactStyle } from "@apps/battle-lab/src/gameFog";

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
  ...over,
});

const maxAlpha = (c: ContactShape) =>
  Math.max(...vertices(buildContactGlyphs([c], flat, style).translucent).map((v) => v.rgba[3]));

test("a glyph uses the shared presented opacity and disappears at zero", () => {
  const full = maxAlpha(shape());
  expect(maxAlpha(shape({ opacity: 0.5 }))).toBeCloseTo(full / 2);
  expect(maxAlpha(shape({ opacity: 0.01 }))).toBeCloseTo(full / 100);
  const gone = buildContactGlyphs([shape({ opacity: 0 })], flat, style);
  expect(gone.translucent.length).toBe(0);
  expect(gone.opaque.length).toBe(0);
});

test("the pale outline follows report life independently of hatch ink", () => {
  for (const opacity of [1, 0.5, 0.05]) {
    for (const hatch_alpha of [0, 0.15, 0.8]) {
      const fixed = { ...style, outline_color: [1, 1, 1] as [number, number, number], hatch_alpha };
      const outline = vertices(
        buildContactGlyphs([shape({ opacity })], flat, fixed).translucent,
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
    return vertices(contactLayer([{ ...report, source }], flat).translucent);
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
    const verts = vertices(buildContactGlyphs([shape({ radius })], flat, style).translucent);
    const reach = Math.max(...verts.map((v) => Math.hypot(v.x - 400, v.y - 300)));
    expect(reach).toBeLessThanOrEqual(radius * 1.1);
    expect(reach).toBeGreaterThan(radius * 0.95);
  }
});

const report: PresentedContact = {
  id: 7,
  source: "last_seen",
  center: [400, 300, 0],
  layer: "ground",
  radius: 100,
  evidenceTick: 150,
  expiresTick: 390,
  opacity: 1,
  retiring: false,
  kind: "test_tank",
  heard: [],
};

test("a glyph draws only the reported area and opacity, never identity or remembered type", () => {
  const plain = contactLayer([report], flat);
  const renamed = { ...report, id: 99, kind: null, heard: ["rifle"] };
  expect(contactLayer([renamed], flat).translucent).toEqual(plain.translucent);
});
