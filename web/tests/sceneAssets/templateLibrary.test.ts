// @vitest-environment node
// City kits and the template art library, end to end: a kit bakes to a bundle
// of modules; the sets pack, deterministically, into one library the loader
// installs and binds to its kits as they are asked for; the resolver puts a template's rows where its frame
// says; and art that leaves its physical template, or a catalogue the sets do
// not cover, is refused by name.
import { expect, test } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { decodeBundle } from "@packages/scene-assets/src/codec.ts";
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
  PROTOTYPE_KIT,
  PROTOTYPE_SET,
  prototypeKitGlb,
  prototypeTemplates,
  templateSetText,
} from "@packages/scene-assets/src/prototypeSet.ts";
import { physicalTemplates } from "@web/maps/node";
import { AUTHORITY, testCatalog, testSources } from "./synthetic";
import {
  CATALOGUE,
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
  setBytes,
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

async function library(set?: unknown, catalogue?: unknown[]): Promise<TemplateArtLibrary> {
  const result = await bake(set, catalogue);
  expect(result.reports.flatMap((r) => r.findings)).toEqual([]);
  return decodeTemplateLibrary(
    result.files.get(templateLibraryPath(result.runtime.templates!.library))!,
  );
}

const close = (actual: ArrayLike<number>, expected: number[]) =>
  expected.forEach((value, i) => expect(actual[i]).toBeCloseTo(value, 4));

// ---------------------------------------------------------------- the kit

test("a kit bakes to one static bundle whose states are its modules, each in its own frame", async () => {
  const result = await bake();
  const entry = result.runtime.appearances[KIT];
  expect(entry).toMatchObject({ unit: "kit", kind: "static" });
  const kit = decodeBundle(result.files.get(bundlePath(entry.bundle))!) as StaticBundle;
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
  expect(report.bytes).toBe(result.files.get(bundlePath(entry.bundle))!.byteLength);
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

// ---------------------------------------------------------------- packing and loading

test("packing the same sources twice gives the same bytes", async () => {
  const [a, b] = [await bake(), await bake()];
  expect(a.ok).toBe(true);
  expect(b.runtime).toEqual(a.runtime);
  const path = templateLibraryPath(a.runtime.templates!.library);
  expect(b.files.get(path)).toEqual(a.files.get(path));
  // The order the catalog and the file list things in is not content.
  const reordered = testSet((set) => set.templates.reverse());
  expect((await bake(reordered, [YARD, HOUSE])).runtime).toEqual(a.runtime);
});

test("the library holds every template's status and rows, and names what it covers", async () => {
  const lib = await library();
  expect(lib.covers).toEqual([physicalTemplates().complete([HOUSE, YARD]).hash]);
  expect(lib.templates.map((t) => [t.id, t.set, t.status, Object.keys(t.states)])).toEqual([
    ["test-house", "test", "release", ["intact"]],
    ["test-yard", "test", "prototype", ["intact"]],
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
  const installed = await loader.withKits([KIT]);
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
  files.set(bundlePath(otherKit), other.files.get(bundlePath(otherKit))!);
  const runtime = structuredClone(result.runtime);
  runtime.appearances[KIT].bundle = otherKit;
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(runtime)));
  await expect(loader.load("/assets/")).rejects.toThrow(
    /kit\.missing: kit "city_kit_test" is bundle/,
  );
  expect(loader.installed).toBe(first);

  const corrupt = files.get(templateLibraryPath(result.runtime.templates!.library))!.slice();
  corrupt[corrupt.length - 1] ^= 0xff;
  files.set(templateLibraryPath(result.runtime.templates!.library), corrupt);
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  await expect(loader.load("/assets/")).rejects.toThrow(/template library .* content hash/);
  expect(loader.installed).toBe(first);
});

test("a module its kit lacks is refused by name when the library is bound", async () => {
  const result = await bake();
  const lib = decodeTemplateLibrary(
    result.files.get(templateLibraryPath(result.runtime.templates!.library))!,
  );
  const bundle = decodeBundle(
    result.files.get(bundlePath(result.runtime.appearances[KIT].bundle))!,
  ) as StaticBundle;
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
  expect(refusal(() => resolve("test-house", frame, "ruin", lib))).toMatch(
    /^state\.missing \| .*template "test-house" \(set test\) has no "ruin" rows/,
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
      /^templates\.coverage: template test-shed is in the physical catalogue "towns", but no set of it has art for it/,
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
  catalog.city_sets!.second = { templates: "second.json", kit: KIT, catalogue: CATALOGUE };
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

// ---------------------------------------------------------------- two catalogues

/** A solid box: a valid physical template with no floor, door or bay of its
 *  own, as an authored map's building is. */
const BOX = {
  ...descriptor("test-box"),
  floor_heights_m: null,
  entrances: null,
  edges: (descriptor("test-box").edges as object[]).map((edge) => ({ ...edge, bays: null })),
};

/** The test set over its own catalogue, and a `boxes` set that dresses BOX
 *  and names the catalogue `named`: every finding, and the library if any. */
async function twoCatalogues(
  towns: unknown[],
  boxes: { rows: unknown[]; complete: boolean },
  named = "boxes",
) {
  const catalog = cityCatalog();
  catalog.city_sets!.boxes = { templates: "boxes.json", kit: KIT, catalogue: named };
  const set = testSet((s) => {
    s.set = "boxes";
    s.templates = [
      {
        status: "release",
        descriptor: structuredClone(BOX),
        states: { intact: [shellRow(BOX.parts[0])] },
      },
    ];
  });
  const sources: Record<string, Uint8Array> = { ...citySources(), "boxes.json": setBytes(set) };
  const result = await bakeCatalog(
    catalog,
    async (path) => sources[path],
    cityContext(towns, [{ name: "boxes", ...boxes }]),
  );
  const baked = result.runtime.templates;
  return {
    findings: result.reports.flatMap((r) => r.findings).map((f) => `${f.code}: ${f.message}`),
    library: baked && decodeTemplateLibrary(result.files.get(templateLibraryPath(baked.library))!),
  };
}

test("one library covers two catalogues, each held to its own rule", async () => {
  const physical = physicalTemplates();
  const { findings, library } = await twoCatalogues([HOUSE, YARD], {
    rows: [BOX],
    complete: false,
  });
  expect(findings).toEqual([]);
  expect(library!.covers).toEqual([
    physical.valid([BOX]).hash,
    physical.complete([HOUSE, YARD]).hash,
  ]);
  expect(library!.templates.map((t) => [t.id, t.set])).toEqual([
    ["test-box", "boxes"],
    ["test-house", "test"],
    ["test-yard", "test"],
  ]);
  // The same box in a catalogue of complete buildings has no floor, door or bay to be placed by.
  const strict = await twoCatalogues([HOUSE, YARD], { rows: [BOX], complete: true });
  expect(strict.library).toBeUndefined();
  expect(strict.findings.join("\n")).toMatch(
    /templates\.physical: .*template test-box is not a physical template a map may place: test-box: floor, entrance or bay geometry is unresolved/,
  );
});

test("a catalogue is covered by the sets that name it, and no template is in two", async () => {
  // The yard's row moved to the boxes' catalogue: its own set no longer finds
  // it, and no set of the boxes' catalogue dresses it.
  const moved = await twoCatalogues([HOUSE], { rows: [BOX, YARD], complete: false });
  expect(moved.findings).toEqual([
    expect.stringMatching(
      /^templates\.coverage: template test-yard is in the physical catalogue "boxes", but no set of it has art for it/,
    ),
    expect.stringMatching(
      /^templates\.catalogue: set test: template test-yard is not in the physical catalogue "towns"/,
    ),
  ]);
  const twice = await twoCatalogues([HOUSE, YARD], { rows: [BOX, HOUSE], complete: false });
  expect(twice.findings).toContainEqual(
    expect.stringMatching(
      /^templates\.catalogue: template test-house is in the physical catalogues "boxes" and "towns"/,
    ),
  );
  const unnamed = await twoCatalogues([HOUSE, YARD], { rows: [BOX], complete: false }, "attic");
  expect(unnamed.findings).toEqual([
    expect.stringMatching(
      /^templates\.catalogue: .*set "boxes" on catalogue "attic", which is not one of towns, boxes/,
    ),
  ]);
});

test("a descriptor the contract refuses is refused here, in the contract's words", async () => {
  const doorless = testSet((set) => (set.templates[0].descriptor.entrances = []));
  expect((await refusals(doorless, [HOUSE, YARD])).join("\n")).toMatch(
    /templates\.physical: .*template test-house is not a physical template a map may place: test-house: complete building geometry needs an entrance/,
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

// ---------------------------------------------------------------- the prototype set and the catalogue

test("the prototype set is each physical part as a tinted box, labelled a stand-in", async () => {
  const tints = { detached_home: [0.5, 1, 0], default: [0.2, 0.2, 0.2] } as const;
  const set = prototypeTemplates([HOUSE, { ...YARD, category: "industry" }], tints);
  const catalog = {
    ...cityCatalog(),
    appearances: {
      [PROTOTYPE_KIT]: { unit: "kit" as const, source: KIT_SOURCE, basis_yaw_deg: 0 },
    },
    city_sets: {
      [PROTOTYPE_SET]: { templates: "set.json", kit: PROTOTYPE_KIT, catalogue: CATALOGUE },
    },
  };
  const sources: Record<string, Uint8Array> = {
    [KIT_SOURCE]: prototypeKitGlb(),
    "set.json": new TextEncoder().encode(templateSetText(set)),
  };
  const result = await bakeCatalog(
    catalog,
    async (path) => sources[path],
    cityContext([HOUSE, { ...YARD, category: "industry" }]),
  );
  // Fit is zero: the boxes are the parts, the turned barn included.
  expect(set.fit).toEqual({ side_m: 0, top_m: 0 });
  expect(result.reports.flatMap((r) => r.findings)).toEqual([]);
  const lib = decodeTemplateLibrary(
    result.files.get(templateLibraryPath(result.runtime.templates!.library))!,
  );
  expect(lib.templates.map((t) => t.status)).toEqual(["prototype", "prototype"]);
  const yard = templateRows(lib, "test-yard", "intact");
  expect(yard.count).toBe(YARD.parts.length);
  close(
    lib.rows.transform.subarray((yard.first + 1) * ROW_TRANSFORM_FLOATS),
    [10, 0, 0, 0.5, 8, 12, 4],
  );
  const house = templateRows(lib, "test-house", "intact").first;
  expect([...lib.rows.tint.subarray(house * 3, house * 3 + 3)]).toEqual([128, 255, 0]);
  expect([...lib.rows.tint.subarray(yard.first * 3, yard.first * 3 + 3)]).toEqual([51, 51, 51]);
  // Generated twice, the same bytes.
  expect(prototypeKitGlb()).toEqual(prototypeKitGlb());
});

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
