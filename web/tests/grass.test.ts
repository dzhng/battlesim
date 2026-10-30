import {
  GRASS_MIX_MAX,
  GRASS_HEIGHT_MAX,
  GRASS_MAX_HEIGHT_M,
} from "@packages/battle-renderer/src/records.gen";
// @vitest-environment node
// Grass kinds are scenery appearances (generated clumps
// of blade strips), and the grass field grows them where the biome says. The
// field's GPU contract (seating, masks, residency, cost) is the street
// scene's; this pins the CPU seams it stands on.
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { contentSha256 } from "@packages/scene-assets/src/glb.ts";
import {
  GRASS_SEGMENTS,
  grassBladeVertices,
  grassClumpGlb,
  grassMeshHeight,
  grassStripIndices,
  grassStripFindings,
} from "@packages/scene-assets/src/grass.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import type { Catalog, StaticBundle } from "@packages/scene-assets/src/schema.ts";
import {
  bareTerrain,
  presentGround,
  type Biome,
} from "@packages/battle-renderer/src/terrain/terrainSurface.ts";
import { loadPresentation } from "@web/battle/sim/presentation";
import { grass_limits } from "@wasm/renderer_browser";
import summer from "@fixtures/biomes/summer.json";
import { AUTHORITY, GRASS_SPEC, TOLERANCES } from "./sceneAssets/synthetic";

const ROOT = new URL("../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, ROOT));
const catalog = JSON.parse(read("assets/catalog.json").toString()) as Catalog;
const biome: Biome = summer;
await loadPresentation();

/** `of`'s ground (a bare one): the biome is admitted, or refused, in Rust. */
const groundOf = (of: Biome) =>
  presentGround(bareTerrain(new Float32Array(0), [0, 0, 64, 64], 4, of).input);

function meadowOf(mix: Biome["grass"]["growth"]["meadow"]["mix"], patches = {}): Biome {
  const out = structuredClone(biome);
  const meadow = out.grass.growth.meadow;
  const growth = { meadow: { ...meadow, mix, patches: { ...meadow.patches, ...patches } } };
  return { ...out, grass: { ...out.grass, growth: growth as typeof out.grass.growth } };
}

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
  for (const [key, growth] of Object.entries(biome.grass.growth))
    for (const species of growth.mix) {
      const entry = catalog.appearances[species.appearance];
      expect(entry?.unit === "scenery" && entry.scenery === "grass", `${key}`).toBe(true);
    }
  expect(() =>
    groundOf({
      ...biome,
      grass: { ...biome.grass, growth: { orchard: biome.grass.growth.meadow } as never },
    }),
  ).toThrow(/grass.growth.orchard/);
});

test("a row's mix is one to four catalog kinds, each with a share", () => {
  const species = { appearance: "grass_meadow", share: 1, drift: 0, dry: 0 };
  expect(() => groundOf(meadowOf([]))).toThrow(/grass.growth.meadow.mix/);
  expect(() =>
    groundOf(meadowOf(Array.from({ length: GRASS_MIX_MAX + 1 }, () => species))),
  ).toThrow(/grass.growth.meadow.mix/);
  expect(() => groundOf(meadowOf([{ ...species, share: 0 }]))).toThrow(/share/);
  expect(() => groundOf(meadowOf([species], { height: [0.9, 0.5] }))).toThrow(/patches.height/);
});

test("a strip cannot gain height when the field widens its blades", async () => {
  const source = await tuft();
  source.states[0].tiers[0].positions[2 * 3 + 2] += 0.1;
  expect(grassStripFindings("slanted-side", source.states[0].tiers)).not.toEqual([]);
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

test("the field's limits are the scene assets' own: the clump's height variation and the tallest grass", () => {
  // The bake and the shader read them here; the field's check in Rust
  // (`ground::grass::LIMITS`).
  expect([...grass_limits()]).toEqual([GRASS_HEIGHT_MAX, GRASS_MAX_HEIGHT_M]);
});

test("a head shows in every tier with a second pair: the far field keeps its ears and flowers", async () => {
  const eared = await tuft({
    ...GRASS_SPEC,
    jitter: 0,
    head: { from: 0.74, width: 3, chance: 1, color: [0.9, 0.8, 0.1] },
  });
  for (const t of [0, 1, 2]) {
    const mesh = eared.states[0].tiers[t];
    const per = grassBladeVertices(GRASS_SEGMENTS[t]);
    const width = (v: number) =>
      Math.hypot(
        mesh.positions[v * 3] - mesh.positions[v * 3 + 3],
        mesh.positions[v * 3 + 1] - mesh.positions[v * 3 + 4],
      );
    for (let b = 0; b < GRASS_SPEC.blades; b++) {
      // The pair under the tip: the head's colour, and wider than the pair below it.
      const last = b * per + per - 3;
      [230, 204, 26].forEach((v, c) =>
        expect(Math.abs(mesh.colors[last * 4 + c] - v), `tier ${t}`).toBeLessThanOrEqual(1),
      );
      expect(width(last), `tier ${t}`).toBeGreaterThan(width(last - 2));
    }
  }
});

test("wind outside its authored range is refused by the grass baker", () => {
  expect(() => grassClumpGlb("tuft", { ...GRASS_SPEC, wind: 1.5 })).toThrow(/wind/);
});
