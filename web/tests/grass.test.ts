// @vitest-environment node
// Grass kinds are scenery appearances (generated clumps
// of blade strips), and the grass field grows them where the biome says. The
// field's GPU contract (seating, masks, residency, cost) is the village
// scene's; this pins the CPU seams it stands on.
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { mat4, vec3 } from "math";
import { frustum } from "math/shapes";
import { contentSha256 } from "@packages/scene-assets/src/glb.ts";
import {
  GRASS_MAX_HEIGHT_M,
  GRASS_SEGMENTS,
  grassBladeVertices,
  grassClumpGlb,
  grassMeshHeight,
  grassSpecErrors,
  grassStripIndices,
  grassStripFindings,
} from "@packages/scene-assets/src/grass.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import type { Catalog, StaticBundle } from "@packages/scene-assets/src/schema.ts";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome.ts";
import {
  createGrassWindow,
  GRASS_TILE_M,
  grassKinds,
  grassSides,
  grassWindow,
  metresPerPixel,
  packGrassShapes,
  SHAPE_FLOATS,
} from "@packages/battle-renderer/src/terrain/grassField.ts";
import { eyePosition, viewProjMatrix } from "@packages/renderer-core/src/camera3d.ts";
import summer from "@fixtures/biomes/summer.json";
import game from "@fixtures/game.json";
import { AUTHORITY, GRASS_SPEC, TOLERANCES } from "./sceneAssets/synthetic";

const ROOT = new URL("../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, ROOT));
const catalog = JSON.parse(read("assets/catalog.json").toString()) as Catalog;
const biome = validateBiome(summer as unknown as Biome);

async function tuft(spec = GRASS_SPEC): Promise<StaticBundle> {
  const result = await validateAppearance(
    {
      name: "tuft",
      entry: { unit: "scenery", scenery: "grass", states: { summer: "t.glb" }, basis_yaw_deg: 0 },
      files: { "t.glb": grassClumpGlb("tuft", spec) },
    },
    { authority: AUTHORITY, tolerances: TOLERANCES },
  );
  expect(result.findings).toEqual([]);
  return result.bundle as StaticBundle;
}

test("a grass kind bakes into the same blades at every tier, each rooted on the ground", async () => {
  const bundle = await tuft();
  const tiers = bundle.states[0].tiers;
  expect(tiers).toHaveLength(GRASS_SEGMENTS.length);
  const tips = tiers.map((mesh, t) => {
    const per = grassBladeVertices(GRASS_SEGMENTS[t]);
    expect([...mesh.indices]).toEqual([...grassStripIndices(GRASS_SPEC.blades, GRASS_SEGMENTS[t])]);
    return Array.from({ length: GRASS_SPEC.blades }, (_, b) => {
      const at = (v: number) => [0, 1, 2].map((c) => mesh.positions[(b * per + v) * 3 + c]);
      // The first pair stands on the ground at v = 0; the tip is the top, at v = 1.
      expect(Math.abs(at(0)[2]) + Math.abs(at(1)[2])).toBeLessThan(1e-6);
      expect(mesh.uvs[b * per * 2 + 1]).toBe(0);
      expect(mesh.uvs[(b * per + per - 1) * 2 + 1]).toBeCloseTo(1, 6);
      return at(per - 1);
    });
  });
  // Every tier is the same blades: the tips agree.
  for (const tier of tips.slice(1))
    tier.forEach((tip, b) => tip.forEach((v, c) => expect(v).toBeCloseTo(tips[0][b][c], 5)));
});

test("every generated grass kind's source is what its catalog spec generates", async () => {
  const kinds = Object.entries(catalog.appearances).filter(([, e]) => e.grass);
  expect(kinds.length).toBeGreaterThan(0);
  for (const [name, entry] of kinds) {
    const path = Object.values(entry.states!)[0];
    const hash = await contentSha256(grassClumpGlb(name, entry.grass!));
    expect(await contentSha256(new Uint8Array(read(path))), name).toBe(hash);
  }
});

test("the biome grows only catalog grass kinds, on its own plot kinds and the verge", () => {
  for (const [key, growth] of Object.entries(biome.grass.growth)) {
    const entry = catalog.appearances[growth.appearance];
    expect(entry?.unit === "scenery" && entry.scenery === "grass", `${key}`).toBe(true);
  }
  expect(() =>
    validateBiome({
      ...biome,
      grass: { ...biome.grass, growth: { orchard: biome.grass.growth.meadow } },
    }),
  ).toThrow(/grass.growth.orchard/);
});

test("the field waits for every grass kind the biome names", async () => {
  const meadow = await tuft();
  expect(grassKinds(biome, new Map([["grass_meadow", meadow]]))).toBeNull();
  const all = new Map(
    Object.values(biome.grass.growth).map((g) => [g.appearance, meadow] as const),
  );
  const kinds = grassKinds(biome, all)!;
  // Each plot kind's row names its appearance; the verge is the last row.
  const names = kinds.appearances.map((a) => a.name);
  biome.plots.forEach((plot, k) => {
    const g = biome.grass.growth[plot.name];
    expect(kinds.growth[k * 4]).toBeCloseTo(g?.density ?? 0, 6);
    if (g) expect(names[kinds.growth[k * 4 + 2]]).toBe(g.appearance);
  });
  expect(names[kinds.growth[15 * 4 + 2]]).toBe(biome.grass.growth.verge.appearance);
});

test("a clump's tints average to one, so its mean colour is the ground's it grows on", async () => {
  const meadow = await tuft();
  const kinds = grassKinds(
    biome,
    new Map(Object.values(biome.grass.growth).map((g) => [g.appearance, meadow] as const)),
  )!;
  const { shapes, bases, rows } = packGrassShapes(kinds);
  const blades = rows[1];
  const count = blades * grassBladeVertices(GRASS_SEGMENTS[0]);
  for (let c = 0; c < 3; c++) {
    let sum = 0;
    for (let v = 0; v < count; v++) sum += shapes[(bases[0] + v) * SHAPE_FLOATS + 12 + c];
    expect(sum / count).toBeCloseTo(1, 5);
  }
});

/** The village camera's framings, from the fixture. */
function view(distance: number, pitch: number) {
  const d = game.presentation.camera.default;
  return {
    target: vec3.fromValues(d.target[0], d.target[1], 0),
    distance,
    pitch,
    yaw: d.yaw,
    fovY: 0.8,
    aspect: 16 / 9,
    near: 1,
  };
}

test("the field's window covers the view where a pixel is under the fade's end, and nothing from the strategic height", () => {
  const scale = metresPerPixel(0.8, 1080);
  const eye = vec3.create();
  const out = createGrassWindow();
  const cam = view(2000, 0.85);
  eyePosition(eye, cam);
  grassWindow(out, eye, 0, scale, biome.grass);
  expect(out.tilesX * out.tilesY).toBe(0);

  const def = view(game.presentation.camera.default.distance, 0.85);
  eyePosition(eye, def);
  grassWindow(out, eye, 0, scale, biome.grass);
  expect(out.reach).toBeCloseTo(biome.grass.fade_m_per_px[1] / scale, 6);
  expect(Math.abs(out.x0 % GRASS_TILE_M)).toBe(0);
  const [tx, ty] = def.target;
  expect(tx).toBeGreaterThan(out.x0);
  expect(tx).toBeLessThan(out.x0 + out.tilesX * GRASS_TILE_M);
  expect(ty).toBeGreaterThan(out.y0);
  expect(ty).toBeLessThan(out.y0 + out.tilesY * GRASS_TILE_M);
  // The view's side planes hold its target and not the ground behind the eye.
  const sides = grassSides(frustum.create(), viewProjMatrix(mat4.create(), def));
  const inside = (p: number[]) =>
    sides.slice(0, 4).every((pl) => vec3.dot(pl.normal, p as never) + pl.constant >= 0);
  expect(inside([tx, ty, 0])).toBe(true);
  expect(inside([2 * eye[0] - tx, 2 * eye[1] - ty, 0])).toBe(false);
});

test("the field refuses grass whose composed height exceeds 0.9 m", async () => {
  const source = await tuft({ ...GRASS_SPEC, height_m: [0.5, 0.5] });
  const taller = structuredClone(biome);
  for (const growth of Object.values(taller.grass.growth)) growth.height = 1.4;
  const appearances = new Map(
    Object.values(taller.grass.growth).map((g) => [g.appearance, source] as const),
  );
  // Source and biome alone are 0.7 m. Both independent shader variations
  // can reach 1.2, taking the composed field above the physical cap.
  expect(() => grassKinds(taller, appearances)).toThrow(/0.9/);
});

test("a strip cannot gain height when the field widens its blades", async () => {
  const source = await tuft();
  source.states[0].tiers[0].positions[2 * 3 + 2] += 0.1;
  expect(grassStripFindings("slanted-side", source.states[0].tiers)).not.toEqual([]);
});

test("the effective height check includes the far tier's drawn vertices", async () => {
  const source = await tuft({ ...GRASS_SPEC, height_m: [0.3, 0.3] });
  const far = source.states[0].tiers[2];
  far.positions[far.positions.length - 1] = 1.0;
  const appearances = new Map(
    Object.values(biome.grass.growth).map((g) => [g.appearance, source] as const),
  );
  expect(() => grassKinds(biome, appearances)).toThrow(/0.9/);
});

/** A biome growing `source` everywhere, and its kinds' packed shapes. */
function packed(source: StaticBundle) {
  const kinds = grassKinds(
    biome,
    new Map(Object.values(biome.grass.growth).map((g) => [g.appearance, source] as const)),
  )!;
  return packGrassShapes(kinds);
}

test("a kind answers the field's wind as far as its spec says, at every blade vertex", async () => {
  const responses = async (wind: number) => {
    const { shapes, bases, rows } = packed(await tuft({ ...GRASS_SPEC, wind }));
    const count = rows[1] * grassBladeVertices(GRASS_SEGMENTS[0]);
    return Array.from({ length: count }, (_, v) => shapes[(bases[0] + v) * SHAPE_FLOATS + 15]);
  };
  for (const r of await responses(1)) expect(r).toBeCloseTo(1, 6);
  // A stiff stalk: a quarter of the sway, to a byte's step.
  for (const r of await responses(0.25)) expect(Math.abs(r - 0.25)).toBeLessThan(1 / 255);
  expect(grassSpecErrors({ ...GRASS_SPEC, wind: 1.5 })).not.toEqual([]);
});

test("no blade outline, droop or head lifts a blade above its spec's height", async () => {
  const shaped = await tuft({
    ...GRASS_SPEC,
    height_m: [0.4, 0.6],
    lean: [0.3, 0.8],
    droop: 0.5,
    shape: { taper: 5, belly: 3 },
    head: { from: 0.7, width: 3, chance: 1, color: [0.9, 0.8, 0.1] },
  });
  const top = grassMeshHeight(shaped.states[0].tiers);
  expect(top).toBeGreaterThan(0.2);
  expect(top).toBeLessThanOrEqual(0.6);
  expect(grassStripFindings("shaped", shaped.states[0].tiers)).toEqual([]);
});

test("a head takes its own colour: yellow flowers over green stems", async () => {
  const flowering = await tuft({
    ...GRASS_SPEC,
    jitter: 0,
    head: { from: 0.6, width: 2, chance: 1, color: [0.9, 0.8, 0.1] },
  });
  const mesh = flowering.states[0].tiers[0];
  const per = grassBladeVertices(GRASS_SEGMENTS[0]);
  for (let b = 0; b < GRASS_SPEC.blades; b++) {
    const [root, tip] = [b * per, b * per + per - 1].map((v) => [
      ...mesh.colors.subarray(v * 4, v * 4 + 3),
    ]);
    // The head's colour, to a byte's rounding.
    [230, 204, 26].forEach((v, c) => expect(Math.abs(tip[c] - v)).toBeLessThanOrEqual(1));
    // The stem under the head keeps the spec's green: more green than red.
    expect(root[1]).toBeGreaterThan(root[0]);
  }
});

test("the summer biome's every growth row stays under the cap with the catalog's own kinds", async () => {
  const sources = new Map<string, StaticBundle>();
  for (const [name, entry] of Object.entries(catalog.appearances))
    if (entry.grass) sources.set(name, await tuft(entry.grass));
  // `grassKinds` refuses any row whose composed height passes the cap.
  const kinds = grassKinds(biome, sources)!;
  expect(kinds.appearances.length).toBeGreaterThan(0);
  expect(GRASS_MAX_HEIGHT_M).toBe(0.9);
});
