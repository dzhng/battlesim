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
  GRASS_SEGMENTS,
  grassBladeVertices,
  grassClumpGlb,
  grassStripIndices,
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
import village from "@fixtures/village.json";
import { AUTHORITY, GRASS_SPEC, TOLERANCES } from "./sceneAssets/synthetic";

const ROOT = new URL("../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, ROOT));
const catalog = JSON.parse(read("assets/catalog.json").toString()) as Catalog;
const manifest = JSON.parse(read("reuse-manifest.json").toString()) as {
  third_party: { path: string; sha256: string }[];
};
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
    expect(manifest.third_party.find((t) => t.path === path)?.sha256, name).toBe(hash);
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
  const d = village.presentation.camera.default;
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

  const def = view(village.presentation.camera.default.distance, 0.85);
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
