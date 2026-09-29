// @vitest-environment node
// Contact glyphs: red through the middle, hatched, with a red glow, per
// approximate contact, a pale outline round a last sighting, fading to
// nothing at expiry, and built from the contact's own fields only.
import { expect, test } from "vitest";
import {
  buildContactGlyphs,
  contactFreshness,
  type ContactShape,
} from "@packages/battle-renderer/src/contactGlyph";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import type { ObservationView } from "@web/battle/sim/observation";
import { contactLayer } from "@apps/battle-lab/src/battleOverlay";
import { villageContactStyle } from "@apps/battle-lab/src/villageFog";

const flat = () => 0;
const style = villageContactStyle;

function vertices(mesh: Float32Array) {
  const out: { x: number; y: number; rgba: number[] }[] = [];
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS)
    out.push({ x: mesh[i], y: mesh[i + 1], rgba: [...mesh.subarray(i + 6, i + 10)] });
  return out;
}

const shape = (over: Partial<ContactShape> = {}): ContactShape => ({
  center: [400, 300],
  radius: 100,
  freshness: 1,
  source: "last_seen",
  ...over,
});

const maxAlpha = (c: ContactShape) =>
  Math.max(...vertices(buildContactGlyphs([c], flat, style).translucent).map((v) => v.rgba[3]));

test("a glyph fades as its contact ages and is gone at expiry", () => {
  const contact = { evidenceTick: 100, expiresTick: 340 };
  const ticks = [100, 160, 250, 339];
  for (const source of ["last_seen", "firing"]) {
    const alphas = ticks.map((t) =>
      maxAlpha(shape({ source, freshness: contactFreshness(contact, t) })),
    );
    for (let i = 1; i < alphas.length; i++) expect(alphas[i]).toBeLessThan(alphas[i - 1]);
    expect(alphas.at(-1)).toBeGreaterThan(0);
    const expired = buildContactGlyphs(
      [shape({ source, freshness: contactFreshness(contact, 340) })],
      flat,
      style,
    );
    expect(expired.translucent.length).toBe(0);
    expect(expired.opaque.length).toBe(0);
  }
});

test("every glyph is red through its middle and hatched; a last sighting keeps a pale outline", () => {
  const isRed = (c: number[]) => c[0] > c[1] + 0.3 && c[0] > c[2] + 0.3 && c[3] > 0;
  const isPale = (c: number[]) => Math.min(c[0], c[1], c[2]) > 0.8 && c[3] > 0;
  const from = (v: { x: number; y: number }) => Math.hypot(v.x - 400, v.y - 300);
  const ghost = vertices(buildContactGlyphs([shape()], flat, style).translucent);
  const firing = vertices(
    buildContactGlyphs([shape({ source: "firing" })], flat, style).translucent,
  );
  for (const glyph of [ghost, firing]) {
    // Red across the disc, its centre too: a fill, not only a rim.
    expect(glyph.some((v) => isRed(v.rgba) && from(v) < 1)).toBe(true);
    // Nothing pale inside: the hatch is red too.
    expect(glyph.some((v) => isPale(v.rgba) && from(v) < 100 - style.outline_width_m - 1e-3)).toBe(
      false,
    );
  }
  // A last sighting's crisp outline is pale; a firing report has none.
  expect(ghost.some((v) => isPale(v.rgba) && from(v) > 99)).toBe(true);
  expect(firing.some((v) => isPale(v.rgba))).toBe(false);
  // The hatch: many parallel strips across the disc, not only rings.
  const across = (v: { x: number; y: number }) => (v.y - v.x) / Math.SQRT2;
  // The hatch shares the glow's colour; it is told by its own alpha.
  const hatched = ghost.filter(
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
    for (const source of ["last_seen", "firing"]) {
      const verts = vertices(
        buildContactGlyphs([shape({ radius, source })], flat, style).translucent,
      );
      const reach = Math.max(...verts.map((v) => Math.hypot(v.x - 400, v.y - 300)));
      expect(reach).toBeLessThanOrEqual(radius * 1.1);
      expect(reach).toBeGreaterThan(radius * 0.95);
    }
  }
});

/** A published frame with one last-seen contact and whatever else the side knows. */
function observation(extra: Partial<ObservationView>): ObservationView {
  return {
    tick: 200,
    own: [],
    identified: [],
    contacts: [
      {
        id: 7,
        source: "last_seen",
        center: [400, 300],
        radius: 100,
        evidenceTick: 150,
        expiresTick: 390,
      },
    ],
    audible: [],
    knownProps: [],
    projectiles: [],
    blasts: [],
    corpses: [],
    guided: [],
    ...extra,
  } as unknown as ObservationView;
}

test("a glyph shows nothing beyond its contact: other knowledge and ids change nothing", () => {
  const plain = contactLayer(observation({}), flat);
  // An identified enemy near the contact, moving: the glyph must not follow it.
  const busy = contactLayer(
    observation({
      identified: [
        { id: 3, kind: "tank", position: [430, 280, 0], velocity: [5, -2], yaw: 1.2 },
      ] as unknown as ObservationView["identified"],
    }),
    flat,
  );
  expect(busy.translucent).toEqual(plain.translucent);
  // The contact's handle is not drawn either.
  const renamed = observation({});
  renamed.contacts[0].id = 99;
  expect(contactLayer(renamed, flat).translucent).toEqual(plain.translucent);
});
