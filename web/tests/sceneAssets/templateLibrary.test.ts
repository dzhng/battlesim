// @vitest-environment node
// City kits and the template art library, end to end: a kit bakes to a bundle
// of modules; the sets pack, deterministically, into one library the loader
// installs and binds to its kits as they are asked for; the resolver puts a template's rows where its frame
// says; and art that leaves its physical template, or a catalogue the sets do
// not cover, is refused by name.
import { expect, test } from "vitest";
import {
  bakeCatalog,
  runtimeCatalogText,
  type BakeResult,
} from "@packages/scene-assets/src/bake.ts";
import { gzipTransport, unpackGzip } from "@packages/scene-assets/src/gzip.ts";
import { encodeBundle } from "@packages/scene-assets/src/codec.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import {
  bundlePath,
  templateLibraryPath,
  type StaticBundle,
} from "@packages/scene-assets/src/schema.ts";
import {
  ROW_TRANSFORM_FLOATS,
  TemplateArtError,
  bindModules,
  decodeTemplateLibrary,
  placeRows,
  resolve,
  templateRows,
  type TemplateArtLibrary,
} from "@packages/scene-assets/src/templateLibrary.ts";
import {
  catalogueRows,
  catalogueText,
  type TemplateSetSource,
} from "@packages/scene-assets/src/templateSource.ts";
import {
  STAND_IN_KIT,
  STAND_IN_MODULE,
  standInKitGlb,
} from "@packages/scene-assets/src/standInKit.ts";
import { physicalTemplates } from "@web/maps/node";
import { AUTHORITY, bakedBundle, testCatalog, testSources } from "./synthetic";
import {
  HOUSE,
  KIT,
  KIT_SOURCE,
  MODULES,
  YARD,
  cityCatalog,
  cityContext,
  citySources,
  descriptor,
  kitGlb,
  row,
  ruinRow,
  shellRow,
  solid,
  testSet,
  type ModuleSpec,
} from "./city";

async function bake(
  set: unknown = testSet(),
  catalogue: unknown[] = [HOUSE, YARD],
  kit: Uint8Array = kitGlb(),
) {
  const sources = citySources(set, kit);
  return bakeCatalog(cityCatalog(), async (path) => sources[path], cityContext(catalogue));
}

/** The findings of baking `set` over `catalogue`: code and message. */
async function refusals(set: unknown, catalogue?: unknown[], kit?: Uint8Array) {
  const result = await bake(set, catalogue, kit);
  expect(result.ok).toBe(false);
  expect(result.runtime.templates).toBeUndefined();
  return result.reports.flatMap((r) => r.findings).map((f) => `${f.code}: ${f.message}`);
}

async function rawContent(result: BakeResult, hash: string, path: (hash: string) => string) {
  const gzip = gzipTransport(result.runtime, hash);
  return unpackGzip(result.files.get(path(gzip.hash))!, gzip, hash);
}

async function library(set?: unknown, catalogue?: unknown[]): Promise<TemplateArtLibrary> {
  const result = await bake(set, catalogue);
  expect(result.reports.flatMap((r) => r.findings)).toEqual([]);
  return decodeTemplateLibrary(
    await rawContent(result, result.runtime.templates!.library, templateLibraryPath),
  );
}

const close = (actual: ArrayLike<number>, expected: number[]) =>
  expected.forEach((value, i) => expect(actual[i]).toBeCloseTo(value, 4));

// ---------------------------------------------------------------- the kit

test("a kit bakes to one static bundle whose states are its modules, each in its own frame", async () => {
  const result = await bake();
  const entry = result.runtime.appearances[KIT];
  expect(entry).toMatchObject({ unit: "kit", kind: "static" });
  const kit = (await bakedBundle(result, entry.bundle)) as StaticBundle;
  expect(kit.states.map((s) => s.name)).toEqual(["shell", "sill"]);
  for (const state of kit.states) expect(state.tiers).toHaveLength(4);
  // The sill is laid out 40 m along in the file; its geometry is its own frame's.
  const sill = kit.states[1].bounds;
  close(sill.min, [-0.5, -0.4, 0]);
  close(sill.max, [0.5, 0, 0.2]);
  const report = result.reports.find((r) => r.name === KIT)!;
  expect(report.stats?.modules).toEqual([
    { name: "shell", triangles: [12, 12, 12, 12] },
    { name: "sill", triangles: [12, 12, 12, 12] },
  ]);
  expect(report.bytes).toBe(encodeBundle(kit).byteLength);
});

test("a module carries all four tiers, finest first", async () => {
  const sill = (...tiers: (number | null)[]): ModuleSpec => ({
    name: "sill",
    boxes: tiers.map((tier) => ({ tier, min: [-0.5, -0.4, 0], max: [0.5, 0, 0.2] })),
  });
  const missing = await refusals(testSet(), undefined, kitGlb([MODULES[0], sill(3)]));
  expect(missing.join("\n")).toMatch(
    /structure\.tier_count: .*module "sill" has no geometry in _LOD0, _LOD1, _LOD2/,
  );
  const inverted = await refusals(testSet(), undefined, kitGlb([MODULES[0], sill(null, 3)]));
  expect(inverted.join("\n")).toMatch(/structure\.tier_order: .*module "sill": tier 3 has 24/);
});

test("the stand-in kit is one module, a metre cube standing on its base, the same bytes each time", async () => {
  const catalog = {
    ...cityCatalog(),
    appearances: { [STAND_IN_KIT]: { unit: "kit" as const, source: KIT_SOURCE, basis_yaw_deg: 0 } },
    city_sets: {},
  };
  const result = await bakeCatalog(catalog, async () => standInKitGlb(), cityContext());
  expect(result.reports.flatMap((r) => r.findings)).toEqual([]);
  const entry = result.runtime.appearances[STAND_IN_KIT];
  const kit = (await bakedBundle(result, entry.bundle)) as StaticBundle;
  expect(kit.states.map((s) => s.name)).toEqual([STAND_IN_MODULE]);
  // A prop's stand-in is this box scaled by the prop's own size.
  close(kit.states[0].bounds.min, [-0.5, -0.5, 0]);
  close(kit.states[0].bounds.max, [0.5, 0.5, 1]);
  expect(standInKitGlb()).toEqual(standInKitGlb());
});

// ---------------------------------------------------------------- packing and loading

test("packing the same sources twice gives the same bytes", async () => {
  const [a, b] = [await bake(), await bake()];
  expect(a.ok).toBe(true);
  expect(b.runtime).toEqual(a.runtime);
  const path = templateLibraryPath(gzipTransport(a.runtime, a.runtime.templates!.library).hash);
  expect(b.files.get(path)).toEqual(a.files.get(path));
  // The order the catalog and the file list things in is not content.
  const reordered = testSet((set) => set.templates.reverse());
  expect((await bake(reordered, [YARD, HOUSE])).runtime).toEqual(a.runtime);
});

test("the library holds every template's status and rows, and names what it covers", async () => {
  const lib = await library();
  expect(lib.covers).toBe(physicalTemplates().catalogue([HOUSE, YARD]).hash);
  expect(lib.templates.map((t) => [t.id, t.set, t.status, Object.keys(t.states)])).toEqual([
    ["test-house", "test", "release", ["intact", "ruin"]],
    ["test-yard", "test", "prototype", ["intact", "ruin"]],
  ]);
  const house = templateRows(lib, "test-house", "intact");
  expect(house.count).toBe(2);
  const sill = house.first + 1;
  expect(lib.modules[lib.rows.module[sill]]).toEqual({ kit: 0, module: "sill" });
  expect(lib.kits[0].appearance).toBe(KIT);
  close(lib.rows.transform.subarray(sill * ROW_TRANSFORM_FLOATS), [2, -4, 1.2, 0, 1, 1, 1]);
  expect(lib.rows.tiers[sill]).toBe(15);
  expect([...lib.rows.tint.subarray(house.first * 3, house.first * 3 + 3)]).toEqual([
    200, 180, 160,
  ]);
});

async function served(set?: unknown) {
  const result = await bake(set);
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  return { result, files, loader: new AppearanceLibrary(memoryFetch(files, "/assets/")) };
}

test("the loader installs the library, and each module is bound to its kit's state once the kit is asked for", async () => {
  const { loader, result } = await served();
  await loader.load("/assets/");
  const installed = await loader.withAppearances([KIT]);
  const art = installed.templates!;
  expect(art.library.art_hash).toBe(result.runtime.templates!.art_hash);
  const kit = installed.appearances.get(KIT)!.bundle as StaticBundle;
  expect(art.modules.map((m) => [m.kit, kit.states[m.state!].name])).toEqual([
    [KIT, "shell"],
    [KIT, "sill"],
  ]);
});

test("a library that does not match its kits fails the whole load", async () => {
  const { loader, files, result } = await served();
  const first = await loader.load("/assets/");
  // The same catalog, its kit swapped for one baked without the sill.
  const other = await bake(
    testSet((set) => set.templates[0].states.intact!.pop()),
    undefined,
    kitGlb([MODULES[0]]),
  );
  const otherKit = other.runtime.appearances[KIT].bundle;
  const otherPath = bundlePath(gzipTransport(other.runtime, otherKit).hash);
  files.set(otherPath, other.files.get(otherPath)!);
  const runtime = structuredClone(result.runtime);
  runtime.appearances[KIT].bundle = otherKit;
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(runtime)));
  await expect(loader.load("/assets/")).rejects.toThrow(
    /kit\.missing: kit "city_kit_test" is bundle/,
  );
  expect(loader.installed).toBe(first);

  const libraryPath = templateLibraryPath(
    gzipTransport(result.runtime, result.runtime.templates!.library).hash,
  );
  const corrupt = files.get(libraryPath)!.slice();
  corrupt[corrupt.length - 1] ^= 0xff;
  files.set(libraryPath, corrupt);
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  await expect(loader.load("/assets/")).rejects.toThrow(/gzip content hash/);
  expect(loader.installed).toBe(first);
});

test("a module its kit lacks is refused by name when the library is bound", async () => {
  const result = await bake();
  const lib = decodeTemplateLibrary(
    await rawContent(result, result.runtime.templates!.library, templateLibraryPath),
  );
  const bundle = (await bakedBundle(
    result,
    result.runtime.appearances[KIT].bundle,
  )) as StaticBundle;
  const without = { ...bundle, states: bundle.states.filter((s) => s.name !== "sill") };
  const bind = (kitBundle: StaticBundle | undefined) => () => bindModules(lib, () => kitBundle);
  expect(bind(bundle)()).toEqual([
    { kit: KIT, state: 0 },
    { kit: KIT, state: 1 },
  ]);
  expect(bind(without)).toThrow('module.missing: kit "city_kit_test" has no module "sill"');
  // A kit that is not installed binds its modules to no state.
  expect(bind(undefined)()).toEqual([
    { kit: KIT, state: null },
    { kit: KIT, state: null },
  ]);
});

// ---------------------------------------------------------------- the resolver

test("the resolver puts a row where the placed frame says", async () => {
  const lib = await library();
  // A quarter turn, then moved: the sill at (2, -4, 1.2) lands at (100 + 4, 50 + 2, 7 + 1.2).
  const frame = { translation: [100, 50, 7], yaw: Math.PI / 2 } as const;
  const placed = resolve("test-house", frame, "intact", lib);
  expect(placed.rows).toEqual(templateRows(lib, "test-house", "intact"));
  expect(placed.transforms).toHaveLength(2 * ROW_TRANSFORM_FLOATS);
  close(placed.transforms.subarray(ROW_TRANSFORM_FLOATS), [104, 52, 8.2, Math.PI / 2, 1, 1, 1]);
  // The shell keeps its stretch and turns with the building.
  close(placed.transforms, [100, 50, 7, Math.PI / 2, 12, 8, 6]);
  // A part's own yaw adds to the frame's.
  const yard = resolve("test-yard", { translation: [0, 0, 0], yaw: 1 }, "intact", lib);
  close(yard.transforms.subarray(ROW_TRANSFORM_FLOATS), [
    10 * Math.cos(1),
    10 * Math.sin(1),
    0,
    1.5,
    8,
    12,
    4,
  ]);
});

test("the resolver writes into the caller's buffer and nowhere else", async () => {
  const lib = await library();
  const out = new Float32Array(4 + 2 * ROW_TRANSFORM_FLOATS + 3).fill(-1);
  const frame = { translation: [1, 2, 3], yaw: 0 } as const;
  const placed = resolve("test-house", frame, "intact", lib, out, 4);
  expect(placed.transforms).toBe(out);
  expect(placed.offset).toBe(4);
  expect([...out.subarray(0, 4), ...out.subarray(out.length - 3)]).toEqual(Array(7).fill(-1));
  close(out.subarray(4 + ROW_TRANSFORM_FLOATS), [3, -2, 4.2, 0, 1, 1, 1]);
  const end = placeRows(lib, placed.rows, frame, out, 4);
  expect(end).toBe(4 + 2 * ROW_TRANSFORM_FLOATS);
});

test("missing art is refused by name, never replaced", async () => {
  const lib = await library();
  const frame = { translation: [0, 0, 0], yaw: 0 } as const;
  const refusal = (run: () => unknown) => {
    try {
      run();
    } catch (e) {
      return e instanceof TemplateArtError ? `${e.code} | ${e.message}` : String(e);
    }
    return "resolved";
  };
  expect(refusal(() => resolve("china-slab-35x11", frame, "intact", lib))).toMatch(
    /^template\.missing \| .*template "china-slab-35x11" has no art/,
  );
  expect(refusal(() => resolve("test-house", frame, "gutted", lib))).toMatch(
    /^state\.missing \| .*template "test-house" \(set test\) has no "gutted" rows/,
  );
});

// ---------------------------------------------------------------- fit and coverage

test("art stays on its physical parts, grown by the set's fit", async () => {
  // The sill reaches 0.4 m out of the south wall: inside a 0.5 m side fit.
  expect((await bake()).ok).toBe(true);
  const tight = await refusals(testSet((set) => (set.fit.side_m = 0.3)));
  expect(tight).toEqual([
    expect.stringMatching(
      /^templates\.fit: .*template test-house intact: row 1 \(module "sill"\) reaches \[.*, -4\.400, .*\], 0\.\d+ m outside part "body"/,
    ),
  ]);
  // Above the roof by more than the top fit.
  const chimney = (height: number) =>
    testSet((set) => set.templates[0].states.intact!.push(row(0, [0, 0, 6], [1, 1, height])));
  expect((await bake(chimney(1))).ok).toBe(true);
  expect((await refusals(chimney(1.5))).join("\n")).toMatch(
    /templates\.fit: .*row 2 \(module "shell"\)/,
  );
  // Below the ground.
  const sunk = testSet((set) => (set.templates[0].states.intact![0][3] = -0.5));
  expect((await refusals(sunk)).join("\n")).toMatch(/templates\.fit: .*row 0/);
});

test("fit follows a turned part, and a row only draws at the tiers it names", async () => {
  // The barn stands turned 0.5 rad: a shell that ignores the turn leaves it.
  const unturned = testSet((set) => (set.templates[1].states.intact![1][4] = 0));
  expect((await refusals(unturned)).join("\n")).toMatch(
    /templates\.fit: .*template test-yard intact: row 1 .*outside part "barn"/,
  );
  // A kit whose sill is far too deep in its coarsest tier only.
  const deepCoarse = kitGlb([
    MODULES[0],
    {
      name: "sill",
      boxes: [0, 1, 2, 3].map((tier) => ({
        tier,
        min: [-0.5, tier === 3 ? -3 : -0.4, 0],
        max: [0.5, 0, 0.2],
      })),
    },
  ]);
  expect((await refusals(testSet(), undefined, deepCoarse)).join("\n")).toMatch(/templates\.fit/);
  const fineOnly = testSet((set) => (set.templates[0].states.intact![1][8] = 0b0111));
  expect((await bake(fineOnly, undefined, deepCoarse)).ok).toBe(true);
});

test("a row's module must be in the kit", async () => {
  const unknown = await refusals(testSet((set) => set.modules.push("balcony")));
  expect(unknown).toContainEqual(
    expect.stringMatching(/^templates\.module: .*module "balcony" is not in kit "city_kit_test"/),
  );
  const unlisted = await refusals(testSet((set) => (set.templates[0].states.intact![1][0] = 7)));
  expect(unlisted).toEqual([
    expect.stringMatching(
      /^templates\.module: .*template test-house intact row 1: module 7 is not one of the set's 2 modules/,
    ),
  ]);
});

test("every template is drawn intact", async () => {
  const none = await refusals(
    testSet((set) => {
      set.templates[0].states = { ruin: set.templates[0].states.intact };
    }),
  );
  expect(none).toEqual([
    expect.stringMatching(/^templates\.state: .*template test-house has no "intact" rows/),
  ]);
});

// ---------------------------------------------------------------- damage states

/** An apartment block of `floors` floors, 3 m each, as a template of one part. The
 *  category admits four to eight floors: the rule's six-floor line runs through it. */
function block(id: string, floors: number) {
  const d = descriptor(id, [{ id: "body", center: [0, 0], half: [6, 4, 1.5 * floors] }]);
  d.category = "urban_apartment";
  d.floor_heights_m = Array.from({ length: floors }, (_, k) => 3 * k);
  return d;
}

/** A set of one template, `d`, drawn intact as its parts and destroyed as `damage` says. */
const blockSet = (d: typeof HOUSE, damage: TemplateSetSource["templates"][number]["states"]) =>
  testSet((set) => {
    set.templates = [
      {
        status: "release",
        descriptor: d,
        states: { intact: d.parts.map((p) => shellRow(p)), ...damage },
      },
    ];
  });

test("a template carries the damage state its floors call for, and no other", async () => {
  // Six floors or fewer collapse: a ruin, and no burnt shell.
  const low = block("test-low", 6);
  const lowRuin = [ruinRow(low.parts[0], 4.5)];
  const standing = low.parts.map((p) => shellRow(p));
  await library(blockSet(low, { ruin: lowRuin }), [low]);
  expect(await refusals(blockSet(low, {}), [low])).toEqual([
    expect.stringMatching(
      /^templates\.state: .*template test-low \(6 floors\) collapses, and has no "ruin" rows/,
    ),
  ]);
  expect(await refusals(blockSet(low, { ruin: lowRuin, gutted: standing }), [low])).toEqual([
    expect.stringMatching(
      /^templates\.state: .*template test-low \(6 floors\) collapses, and has "gutted" rows/,
    ),
  ]);
  // Taller ones stand, gutted: a burnt shell, and no ruin.
  const tall = block("test-tall", 7);
  const shell = tall.parts.map((p) => shellRow(p));
  await library(blockSet(tall, { gutted: shell }), [tall]);
  expect(await refusals(blockSet(tall, {}), [tall])).toEqual([
    expect.stringMatching(
      /^templates\.state: .*template test-tall \(7 floors\) stands gutted, and has no "gutted" rows/,
    ),
  ]);
  expect(
    await refusals(blockSet(tall, { gutted: shell, ruin: [ruinRow(tall.parts[0], 5)] }), [tall]),
  ).toEqual([
    expect.stringMatching(
      /^templates\.state: .*template test-tall \(7 floors\) stands gutted, and has "ruin" rows/,
    ),
  ]);
});

test("a ruin stays inside the remains the simulation leaves", async () => {
  // The remains are a quarter of the building's height, never under 2 m nor over 6 m
  // (the test rule): a ruin as tall as that fits, and one a little taller does not.
  for (const [height, remains] of [
    [6, 2],
    [20, 5],
    [30, 6],
  ]) {
    const d = descriptor("test-hall", [{ id: "body", center: [0, 0], half: [6, 4, height / 2] }]);
    await library(blockSet(d, { ruin: [ruinRow(d.parts[0], remains)] }), [d]);
    expect(
      await refusals(blockSet(d, { ruin: [ruinRow(d.parts[0], remains + 0.25)] }), [d]),
    ).toEqual([
      expect.stringMatching(
        new RegExp(
          `^templates\\.fit: .*template test-hall ruin: row 0 \\(module "shell"\\) reaches .*, 0\\.24\\d m outside part "body" at its ruin height \\(${remains}\\.000 m\\)`,
        ),
      ),
    ]);
  }
  // Broken walls end raggedly: a set says how far above the remains they may reach.
  const jagged = (over: number, allowance: number) => {
    const set = blockSet(HOUSE, { ruin: [ruinRow(HOUSE.parts[0], 2 + over)] });
    set.fit.ruin_top_m = allowance;
    return set;
  };
  await library(jagged(0.3, 0.3), [HOUSE]);
  expect((await refusals(jagged(0.5, 0.3), [HOUSE])).join("\n")).toMatch(
    /templates\.fit: .*test-house ruin: .*ruin top 0\.3 m/,
  );
  // The roof's allowance is not the ruin's, and the sides' fit is the standing building's.
  expect((await refusals(jagged(0.9, 0), [HOUSE])).join("\n")).toMatch(/templates\.fit/);
  const spilt = blockSet(HOUSE, { ruin: [row(0, [0, 0, 0], [13.2, 8, 2])] });
  expect((await refusals(spilt, [HOUSE])).join("\n")).toMatch(
    /templates\.fit: .*test-house ruin: .*reaches \[-?6\.600, /,
  );
  await library(blockSet(HOUSE, { ruin: [row(0, [0, 0, 0], [12.8, 8, 2])] }), [HOUSE]);
});

test("every part of a building falls to the one height, the building's", async () => {
  // A 20 m hall with an 8 m wing: both leave 5 m of remains, though a quarter of the wing is 2 m.
  const d = descriptor("test-works", [
    { id: "hall", center: [-10, 0], half: [5, 4, 10] },
    { id: "wing", center: [10, 0], half: [4, 4, 4] },
  ]);
  await library(blockSet(d, { ruin: d.parts.map((p) => ruinRow(p, 5)) }), [d]);
  expect(
    (await refusals(blockSet(d, { ruin: d.parts.map((p) => ruinRow(p, 5.5)) }), [d])).join("\n"),
  ).toMatch(/templates\.fit: .*test-works ruin: .*ruin height \(5\.000 m\).*; 1 more row/);
});

test("a state draws something at every tier: a building never vanishes with distance", async () => {
  // Every row of the house stops short of the coarsest tier, which is the
  // tier the whole map draws at.
  const fineOnly = await refusals(
    testSet((set) => {
      for (const row of set.templates[0].states.intact!) row[8] = 0b0111;
    }),
  );
  expect(fineOnly).toEqual([
    expect.stringMatching(
      /^templates\.state: .*template test-house intact draws nothing at tier 3/,
    ),
  ]);
});

test("the catalogue and the sets cover each other exactly", async () => {
  const shed = descriptor("test-shed", [{ id: "body", center: [0, 0], half: [3, 3, 2] }]);
  expect(await refusals(testSet(), [HOUSE, YARD, shed])).toEqual([
    expect.stringMatching(
      /^templates\.coverage: template test-shed is in the physical catalogue, but no set has art for it/,
    ),
  ]);
  expect(await refusals(testSet(), [HOUSE])).toEqual([
    expect.stringMatching(
      /^templates\.catalogue: .*template test-yard is not in the physical catalogue/,
    ),
  ]);
  const taller = structuredClone(HOUSE);
  taller.parts[0].half_extents[2] = 3.5;
  expect(await refusals(testSet(), [taller, YARD])).toEqual([
    expect.stringMatching(/^templates\.catalogue: .*template test-house differs from its row/),
  ]);
});

test("a template is dressed by exactly one set", async () => {
  const catalog = cityCatalog();
  catalog.city_sets!.second = { templates: "second.json", kit: KIT };
  const second = testSet((set) => {
    set.set = "second";
    set.templates.pop();
  });
  const sources = citySources();
  sources["second.json"] = new TextEncoder().encode(JSON.stringify(second));
  const result = await bakeCatalog(catalog, async (path) => sources[path], cityContext());
  expect(result.reports.flatMap((r) => r.findings).map((f) => `${f.code}: ${f.message}`)).toEqual([
    expect.stringMatching(
      /^templates\.coverage: template test-house is dressed by 2 sets \(second, test\)/,
    ),
  ]);
});

test("a descriptor the contract refuses is refused here, in the contract's words", async () => {
  const doorless = testSet((set) => (set.templates[0].descriptor.entrances = []));
  expect((await refusals(doorless, [HOUSE, YARD])).join("\n")).toMatch(
    /templates\.physical: .*template test-house is not a physical template a map may place: test-house: a building needs an entrance/,
  );
});

// ---------------------------------------------------------------- identity

test("art identity moves with the art; physical identity only with the geometry", async () => {
  const base = (await bake()).runtime.templates!;
  const tinted = (await bake(testSet((set) => (set.templates[0].states.intact![0][9] = 90))))
    .runtime.templates!;
  const moved = (await bake(testSet((set) => (set.templates[0].states.intact![1][1] = 1)))).runtime
    .templates!;
  const coarser = (await bake(testSet((set) => (set.templates[0].states.intact![1][8] = 3))))
    .runtime.templates!;
  const accepted = (await bake(testSet((set) => (set.templates[1].status = "release")))).runtime
    .templates!;
  const thicker = (
    await bake(
      testSet(),
      undefined,
      kitGlb([MODULES[0], solid("sill", [-0.5, -0.4, 0], [0.5, 0, 0.3])]),
    )
  ).runtime.templates!;
  const variants = [tinted, moved, coarser, accepted, thicker];
  for (const variant of variants) expect(variant.covers).toEqual(base.covers);
  expect(new Set([base, ...variants].map((v) => v.art_hash)).size).toBe(variants.length + 1);

  // The same rows on a taller house: another catalogue, so other art.
  const taller = structuredClone(HOUSE);
  taller.parts[0].half_extents[2] = 3.2;
  const regrown = (
    await bake(
      testSet((set) => (set.templates[0].descriptor = structuredClone(taller))),
      [taller, YARD],
    )
  ).runtime.templates!;
  expect(regrown.covers).not.toEqual(base.covers);
  expect(regrown.art_hash).not.toBe(base.art_hash);
});

test("a catalog without city sets bakes as before, with no library", async () => {
  const sources = testSources();
  const result = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    ...cityContext(),
    authority: AUTHORITY,
  });
  expect(result.ok).toBe(true);
  expect(result.runtime.templates).toBeUndefined();
  expect(runtimeCatalogText(result.runtime)).not.toContain("templates");
});

// ---------------------------------------------------------------- the catalogue

test("the catalogue's rows are the sets' descriptors, and unchanged rows stay as written", () => {
  const physical = physicalTemplates();
  const sets = (...descriptors: (typeof HOUSE)[]): TemplateSetSource[] => [
    testSet((set) => {
      set.templates = descriptors.map((d) => ({
        status: "release",
        descriptor: d,
        states: { intact: d.parts.map((part) => shellRow(part)) },
      }));
    }),
  ];
  const shed = descriptor("test-shed", [{ id: "body", center: [0, 0], half: [3, 3, 2] }]);
  const written = structuredClone([YARD, HOUSE]);
  // In step: the very rows, in the catalogue's own order.
  const same = catalogueRows(sets(HOUSE, YARD), written, physical);
  expect(same).toEqual(written);
  expect(same[0]).toBe(written[0]);
  // A new template is added, a changed one replaced in place, a dropped one removed.
  const taller = structuredClone(HOUSE);
  taller.parts[0].half_extents[2] = 3.5;
  expect(catalogueRows(sets(taller, shed), written, physical)).toEqual([taller, shed]);
  // Its text reads back as the rows, with every number a real.
  const text = catalogueText([YARD, shed]);
  expect(JSON.parse(text)).toEqual([YARD, shed]);
  expect(text).toContain('"half_extents": [4.0, 6.0, 2.0]');
  expect(text).toContain('"yaw": 0.5,');
});
