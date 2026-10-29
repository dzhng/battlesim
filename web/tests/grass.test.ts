// @vitest-environment node
// Grass kinds are scenery appearances (generated clumps
// of blade strips), and the grass field grows them where the biome says. The
// field's GPU contract (seating, masks, residency, cost) is the street
// scene's; this pins the CPU seams it stands on.
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { contentSha256 } from "@packages/scene-assets/src/glb.ts";
import {
  GRASS_HEIGHT_VARIATION,
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
import {
  bareTerrain,
  presentGround,
  type Biome,
} from "@packages/battle-renderer/src/terrain/terrainSurface.ts";
import {
  GRASS_GROWTH_ROWS,
  GRASS_MIX_MAX,
  grassShapes,
} from "@packages/battle-renderer/src/terrain/grassField.ts";
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

/** Where `GrassParams`' growth rows and mixes start, in floats. */
const GROWTH = 56;
/** Floats a packed clump vertex (`ground::grass::SHAPE_FLOATS`); its tint
 *  from 12, its wind's weight at 15. */
const SHAPE_FLOATS = 16;
const MIXES = GROWTH + GRASS_GROWTH_ROWS * 8;

/** The biome's growth resolved against `sources` as the field resolves it:
 *  null (no grass) until every kind it names is installed, else the kinds in
 *  order, their packed shapes and the pass's parameters. Throws as the field
 *  refuses. */
function grassKinds(of: Biome, sources: ReadonlyMap<string, StaticBundle>) {
  const ground = groundOf(of);
  const names = JSON.parse(ground.grass_kinds()) as string[];
  if (!names.every((name) => sources.has(name))) return null;
  const packed = grassShapes(names, sources);
  try {
    const words = ground.grass_params(packed, 1);
    return { names, shapes: packed.shapes(), params: new Float32Array(words.buffer) };
  } finally {
    packed.free();
  }
}

/** Every grass the biome's rows name, each drawn as `bundle`. */
const growing = (of: Biome, bundle: StaticBundle) =>
  new Map(
    Object.values(of.grass.growth).flatMap((g) =>
      g.mix.map((s) => [s.appearance, bundle] as const),
    ),
  );

/** `biome` growing on its meadows alone, and `mix` there in place of their own. */
function meadowOf(mix: Biome["grass"]["growth"]["meadow"]["mix"], patches = {}): Biome {
  const out = structuredClone(biome);
  const meadow = out.grass.growth.meadow;
  const growth = { meadow: { ...meadow, mix, patches: { ...meadow.patches, ...patches } } };
  return { ...out, grass: { ...out.grass, growth: growth as typeof out.grass.growth } };
}

/** What the field packs for plot kind `name`'s row: its clumps' kinds by
 *  share, as the build reads them. */
function packedMix(of: Biome, kinds: NonNullable<ReturnType<typeof grassKinds>>, name: string) {
  const row = of.plots.findIndex((p) => p.name === name);
  const count = kinds.params[GROWTH + row * 4 + 2];
  return Array.from({ length: count }, (_, i) => {
    const at = MIXES + (row * GRASS_MIX_MAX + i) * 4;
    const [kind, share, drift, dry] = kinds.params.subarray(at, at + 4);
    return { name: kinds.names[kind], share, drift, dry };
  });
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

test("the field waits for every grass kind the biome names", async () => {
  const meadow = await tuft();
  expect(grassKinds(biome, new Map([["grass_meadow", meadow]]))).toBeNull();
  const kinds = grassKinds(biome, growing(biome, meadow))!;
  // Each plot kind's row holds its density (none where it grows nothing);
  // the verge is the last row.
  const growth = biome.grass.growth as Record<string, { density: number }>;
  biome.plots.forEach((plot, k) => {
    expect(kinds.params[GROWTH + k * 4]).toBeCloseTo(growth[plot.name]?.density ?? 0, 6);
  });
  expect(kinds.params[GROWTH + (GRASS_GROWTH_ROWS - 1) * 4]).toBeCloseTo(
    biome.grass.growth.verge.density,
    6,
  );
});

test("a growth row's mix reaches the field as each kind's share of its clumps", async () => {
  const tuftOf = await tuft();
  const mixed = meadowOf([
    { appearance: "grass_meadow", share: 3, drift: 0, dry: 0 },
    { appearance: "grass_rough", share: 1, drift: 0.7, dry: 0.3 },
  ]);
  const kinds = grassKinds(mixed, growing(mixed, tuftOf))!;
  const row = packedMix(mixed, kinds, "meadow");
  expect(row.map((s) => s.name)).toEqual(["grass_meadow", "grass_rough"]);
  expect(row[0].share).toBeCloseTo(0.75, 6);
  expect(row[1].share).toBeCloseTo(0.25, 6);
  // How a kind drifts and dries travels with it.
  expect(row[1].drift).toBeCloseTo(0.7, 6);
  expect(row[1].dry).toBeCloseTo(0.3, 6);
});

test("a clump's tints average to one, so its mean colour is the ground's it grows on", async () => {
  const meadow = await tuft();
  const { shapes } = grassKinds(biome, growing(biome, meadow))!;
  // The first kind's near tier leads the shapes.
  const count = GRASS_SPEC.blades * grassBladeVertices(GRASS_SEGMENTS[0]);
  for (let c = 0; c < 3; c++) {
    let sum = 0;
    for (let v = 0; v < count; v++) sum += shapes[v * SHAPE_FLOATS + 12 + c];
    expect(sum / count).toBeCloseTo(1, 5);
  }
});

test("the field refuses grass whose composed height exceeds 0.9 m", async () => {
  const source = await tuft({ ...GRASS_SPEC, height_m: [0.5, 0.5] });
  const taller = structuredClone(biome);
  for (const growth of Object.values(taller.grass.growth)) {
    growth.height = 1.4;
    growth.patches.height = [0.7, 1.2];
  }
  // Source and biome alone are 0.7 m. The clump's and the patch's
  // independent variations can each reach 1.2, taking the composed field
  // above the physical cap.
  expect(() => grassKinds(taller, growing(taller, source))).toThrow(/0.9/);
});

test("one tall kind in a mix is refused, and only while the row's patches lift it past the cap", async () => {
  const short = await tuft({ ...GRASS_SPEC, height_m: [0.2, 0.2] });
  const tall = await tuft({ ...GRASS_SPEC, height_m: [0.7, 0.7] });
  const mix = [
    { appearance: "grass_meadow", share: 9, drift: 0, dry: 0 },
    { appearance: "grass_rough", share: 1, drift: 0, dry: 0 },
  ];
  const sources = (of: Biome) => growing(of, short).set("grass_rough", tall);
  // 0.7 m, a clump up to 1.2 of it: 0.84 m under level patches, 0.92 m
  // where patches stand a tenth taller.
  const level = meadowOf(mix, { height: [0.8, 1] });
  expect(grassKinds(level, sources(level))).not.toBeNull();
  const lifted = meadowOf(mix, { height: [0.8, 1.1] });
  expect(() => grassKinds(lifted, sources(lifted))).toThrow(/grass.growth.meadow.*0.9/);
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

test("the effective height check includes the far tier's drawn vertices", async () => {
  const source = await tuft({ ...GRASS_SPEC, height_m: [0.3, 0.3] });
  const far = source.states[0].tiers[2];
  far.positions[far.positions.length - 1] = 1.0;
  expect(() => grassKinds(biome, growing(biome, source))).toThrow(/0.9/);
});

test("a kind answers the field's wind as far as its spec says, at every blade vertex", async () => {
  const responses = async (wind: number) => {
    const source = await tuft({ ...GRASS_SPEC, wind });
    const { shapes } = grassKinds(biome, growing(biome, source))!;
    const count = GRASS_SPEC.blades * grassBladeVertices(GRASS_SEGMENTS[0]);
    return Array.from({ length: count }, (_, v) => shapes[v * SHAPE_FLOATS + 15]);
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
  expect(kinds.names.length).toBeGreaterThan(0);
  expect(GRASS_MAX_HEIGHT_M).toBe(0.9);
});

test("the field's limits are the scene assets' own: the clump's height variation and the tallest grass", () => {
  // The bake and the shader read them here; the field's check in Rust
  // (`ground::grass::LIMITS`).
  expect([...grass_limits()]).toEqual([GRASS_HEIGHT_VARIATION[1], GRASS_MAX_HEIGHT_M]);
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
