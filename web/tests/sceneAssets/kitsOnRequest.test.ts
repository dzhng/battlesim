// @vitest-environment node
// Kits on request: the catalog loads without its kits, a kit is fetched when
// something that draws it asks, and a map asks for the kits its own
// buildings' templates place and no other. Each request is a generation of
// its own, installed whole or not at all.
import { expect, test } from "vitest";
import { mapAppearances, mapKits } from "@apps/battle-lab/src/gameAppearances";
import type { PlacedBuildings } from "@packages/battle-renderer/src/models/buildingReferences.ts";
import { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch, type Fetch } from "@packages/scene-assets/src/loader.ts";
import { gzipTransport } from "@packages/scene-assets/src/gzip.ts";
import { bundlePath, templateLibraryPath } from "@packages/scene-assets/src/schema.ts";
import {
  HOUSE,
  KIT,
  MODULES,
  YARD,
  cityCatalog,
  cityContext,
  citySources,
  kitGlb,
  setBytes,
  testSet,
} from "./city";

const YARD_KIT = "city_kit_yard";
const BASE = "/assets/";

/** Two sets over one catalogue, each with a kit of its own: `test` dresses
 *  the house, `yard` the yard. Served from memory, every fetch recorded. */
async function served() {
  const catalog = cityCatalog();
  catalog.appearances[YARD_KIT] = { unit: "kit", source: "yard/kit.glb", basis_yaw_deg: 0 };
  catalog.city_sets!.yard = { templates: "yard/templates.json", kit: YARD_KIT };
  const sources: Record<string, Uint8Array> = {
    ...citySources(testSet((set) => set.templates.pop())),
    // The shell alone: another kit's bytes, so the two are two files.
    "yard/kit.glb": kitGlb([MODULES[0]]),
    "yard/templates.json": setBytes(
      testSet((set) => {
        set.set = "yard";
        set.kit = YARD_KIT;
        set.modules = ["shell"];
        set.templates.shift();
      }),
    ),
  };
  const result = await bakeCatalog(catalog, async (path) => sources[path], cityContext());
  expect(result.reports.flatMap((r) => r.findings)).toEqual([]);
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  const inner = memoryFetch(files, BASE);
  const fetched: string[] = [];
  const fetcher: Fetch = (url) => {
    fetched.push(url.slice(BASE.length));
    return inner(url);
  };
  const path = (kit: string) =>
    bundlePath(gzipTransport(result.runtime, result.runtime.appearances[kit].bundle).hash);
  return {
    files,
    fetched,
    loader: new AppearanceLibrary(fetcher),
    kitFile: { [KIT]: path(KIT), [YARD_KIT]: path(YARD_KIT) },
    libraryFile: templateLibraryPath(
      gzipTransport(result.runtime, result.runtime.templates!.library).hash,
    ),
  };
}

/** A map with one building of each of `templates`. */
const placedOf = (...templates: string[]): PlacedBuildings => ({
  templates,
  template: Uint16Array.from(templates.keys()),
  frames: new Float64Array(templates.length * 4),
  owners: Uint32Array.from(templates.keys()),
});

const LAYOUT = { propAppearance: {}, blockingPropKinds: {}, unitAppearance: {} };

test("the catalog loads with its template library and no kit, each kit named as not fetched", async () => {
  const { loader, fetched, libraryFile } = await served();
  const installed = await loader.load(BASE);
  expect(fetched.sort()).toEqual(["catalog.json", libraryFile].sort());
  expect([...installed.appearances.keys()]).toEqual([]);
  expect([...installed.onRequest].sort()).toEqual([
    [KIT, null],
    [YARD_KIT, null],
  ]);
  // The library is whole: every row is there, bound to no kit yet.
  expect(installed.templates!.library.templates.map((t) => t.id)).toEqual([HOUSE.id, YARD.id]);
  expect(installed.templates!.modules.map((m) => [m.kit, m.state])).toEqual([
    [KIT, null],
    [KIT, null],
    [YARD_KIT, null],
  ]);
});

test("a map fetches the kit its buildings' templates draw from, and no other", async () => {
  const { loader, fetched, kitFile } = await served();
  const catalog = await loader.load(BASE);
  fetched.length = 0;
  const installed = await loader.withAppearances(mapKits(catalog, placedOf(HOUSE.id)));
  expect(fetched).toEqual([kitFile[KIT]]);
  expect(installed.generation).toBe(catalog.generation + 1);
  expect(loader.installed).toBe(installed);
  expect([...installed.appearances.keys()]).toEqual([KIT]);
  expect(installed.templates!.modules.map((m) => [m.kit, m.state])).toEqual([
    [KIT, 0],
    [KIT, 1],
    [YARD_KIT, null],
  ]);
  // What the models layer installs to draw it has the kit.
  const drawn = mapAppearances(
    installed,
    [],
    new PropAppearances(installed, LAYOUT, null),
    placedOf(HOUSE.id),
    false,
  );
  expect([...drawn.appearances.keys()]).toEqual([KIT]);
});

test("a map with no buildings fetches no kit and installs nothing new", async () => {
  const { loader, fetched } = await served();
  const catalog = await loader.load(BASE);
  fetched.length = 0;
  expect(await loader.withAppearances(mapKits(catalog, placedOf()))).toBe(catalog);
  expect(fetched).toEqual([]);
});

test("a kit fetched for one map is not fetched again for the next", async () => {
  const { loader, fetched, kitFile } = await served();
  const catalog = await loader.load(BASE);
  const first = await loader.withAppearances(mapKits(catalog, placedOf(HOUSE.id)));
  fetched.length = 0;
  // The same map again: the generation it already has.
  expect(await loader.withAppearances(mapKits(catalog, placedOf(HOUSE.id)))).toBe(first);
  // The next map has a yard too: only the yard's kit is new.
  const both = placedOf(HOUSE.id, YARD.id);
  const second = await loader.withAppearances(mapKits(first, both));
  expect(fetched).toEqual([kitFile[YARD_KIT]]);
  expect([...second.appearances.keys()].sort()).toEqual([KIT, YARD_KIT]);
  // The first map's kit is the bundle it was, not another copy.
  expect(second.appearances.get(KIT)).toBe(first.appearances.get(KIT));
});

test("two askers for one kit share one fetch", async () => {
  const { loader, fetched, kitFile } = await served();
  await loader.load(BASE);
  fetched.length = 0;
  const [a, b] = await Promise.all([
    loader.withAppearances([KIT]),
    loader.withAppearances([KIT, YARD_KIT]),
  ]);
  expect(fetched.sort()).toEqual([kitFile[KIT], kitFile[YARD_KIT]].sort());
  expect(a.appearances.has(KIT)).toBe(true);
  expect([...b.appearances.keys()].sort()).toEqual([KIT, YARD_KIT]);
  expect([...loader.installed!.appearances.keys()].sort()).toEqual([KIT, YARD_KIT]);
});

test("a kit that fails to arrive is named, and the installed generation stays", async () => {
  const { loader, files, kitFile } = await served();
  const catalog = await loader.load(BASE);
  const first = await loader.withAppearances([KIT]);
  const yard = files.get(kitFile[YARD_KIT])!;
  files.delete(kitFile[YARD_KIT]);
  await expect(
    loader.withAppearances(mapKits(catalog, placedOf(HOUSE.id, YARD.id))),
  ).rejects.toThrow(/kit "city_kit_yard": [0-9a-f]{64}\/bundle\.bin: HTTP 404/);
  expect(loader.installed).toBe(first);
  const corrupt = yard.slice();
  corrupt[corrupt.length - 1] ^= 0xff;
  files.set(kitFile[YARD_KIT], corrupt);
  await expect(loader.withAppearances([YARD_KIT])).rejects.toThrow(
    /kit "city_kit_yard": .*gzip content hash/,
  );
  expect(loader.installed).toBe(first);
  // Served whole again, the same request succeeds.
  files.set(kitFile[YARD_KIT], yard);
  expect((await loader.withAppearances([YARD_KIT])).appearances.has(YARD_KIT)).toBe(true);
});

test("a kit the catalog does not name is refused by name", async () => {
  const { loader } = await served();
  const catalog = await loader.load(BASE);
  await expect(loader.withAppearances(["city_kit_nowhere"])).rejects.toThrow(
    'kit "city_kit_nowhere" is not in the appearance catalog',
  );
  expect(loader.installed).toBe(catalog);
  await expect(
    new AppearanceLibrary(memoryFetch(new Map(), BASE)).withAppearances([KIT]),
  ).rejects.toThrow(/before a catalog was loaded/);
});

test("a map whose kit is not installed is refused by name, never drawn without it", async () => {
  const { loader } = await served();
  const catalog = await loader.load(BASE);
  const house = await loader.withAppearances([KIT]);
  const fit = new PropAppearances(house, LAYOUT, null);
  // The house's kit is there; the yard's was never asked for.
  expect(() => mapAppearances(house, [], fit, placedOf(HOUSE.id, YARD.id), false)).toThrow(
    'kit.missing: kit "city_kit_yard" is not installed',
  );
  expect(() => mapAppearances(catalog, [], fit, placedOf(HOUSE.id), false)).toThrow(
    'kit.missing: kit "city_kit_test" is not installed',
  );
  // A map with no buildings draws from no kit, so none is missed.
  expect([...mapAppearances(catalog, [], fit, placedOf(), false).appearances.keys()]).toEqual([]);
});

test("a catalog reload during arrival retries the complete map selection, including previously held kits", async () => {
  const { files, kitFile } = await served();
  const fetch = memoryFetch(files, BASE);
  let delay = false;
  let release!: () => void;
  let started!: () => void;
  const waiting = new Promise<void>((resolve) => {
    release = resolve;
  });
  const arrived = new Promise<void>((resolve) => {
    started = resolve;
  });
  const loader = new AppearanceLibrary(async (url) => {
    if (delay && url === BASE + kitFile[YARD_KIT]) {
      started();
      await waiting;
    }
    return fetch(url);
  });
  await loader.load(BASE);
  const first = await loader.withAppearances([KIT]);
  delay = true;
  const requested = loader.withAppearances([KIT, YARD_KIT]);
  await arrived;
  await loader.load(BASE);
  delay = false;
  release();
  const current = await requested;
  expect([...current.appearances.keys()].sort()).toEqual([KIT, YARD_KIT]);
  expect(current.appearances.get(KIT)!.bundle).toEqual(first.appearances.get(KIT)!.bundle);
  expect(loader.installed).toBe(current);
});
