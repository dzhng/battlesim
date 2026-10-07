// @vitest-environment node
// Regional looks: a scenery appearance may be a look of one of the presets'
// regional families. The catalog load leaves it out, and a map fetches its
// own family's looks beside its kits, no other family's.
import { expect, test } from "vitest";
import { mapDownloads } from "@apps/battle-lab/src/gameAppearances";
import { PropAppearances } from "@packages/battle-renderer/src/models/propAppearance.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { gzipTransport } from "@packages/scene-assets/src/gzip.ts";
import { AppearanceLibrary, memoryFetch, type Fetch } from "@packages/scene-assets/src/loader.ts";
import { bundlePath, type Catalog } from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, blockGlb, testCatalog, testSources } from "./synthetic";

const FAMILIES = ["paris", "new_york"];
const BASE = "/assets/";

/** The synthetic catalog with a crate look of each of `looks`' families (a
 *  slightly different block each, so each is its own file). */
function regionalCatalog(looks: Record<string, string>, unit: "scenery" | "vehicle" = "scenery") {
  const catalog: Catalog = testCatalog();
  const sources = testSources();
  Object.entries(looks).forEach(([name, family], k) => {
    const source = `assets/source/${name}.glb`;
    catalog.appearances[name] =
      unit === "scenery"
        ? { ...catalog.appearances.crate, states: { default: source }, regional_family: family }
        : { ...catalog.appearances.truck, source, regional_family: family };
    sources[source] =
      unit === "scenery"
        ? blockGlb(6 - 0.01 * (k + 1))
        : sources[catalog.appearances.truck.source!];
  });
  return { catalog, sources };
}

const bake = ({ catalog, sources }: ReturnType<typeof regionalCatalog>) =>
  bakeCatalog(catalog, async (path) => sources[path], {
    authority: AUTHORITY,
    regionalFamilies: FAMILIES,
  });

const errorsOf = (result: Awaited<ReturnType<typeof bake>>, name: string) =>
  result.reports
    .filter((r) => r.name === name)
    .flatMap((r) => r.findings.filter((f) => f.severity === "error").map((f) => f.code));

test("a regional look bakes only for a family the presets name, and only as scenery", async () => {
  const result = await bake(regionalCatalog({ crate_paris: "paris", crate_mars: "mars" }));
  expect(errorsOf(result, "crate_paris")).toEqual([]);
  expect(result.runtime.appearances.crate_paris.regional_family).toBe("paris");
  expect(errorsOf(result, "crate_mars")).toEqual(["structure.regional_family"]);
  expect(result.runtime.appearances.crate_mars).toBeUndefined();
  // A vehicle is the same everywhere: a body, not a region's dressing.
  const vehicle = await bake(regionalCatalog({ truck_paris: "paris" }, "vehicle"));
  expect(errorsOf(vehicle, "truck_paris")).toEqual(["structure.regional_family"]);
  // A bake told of no families admits no regional look.
  const { catalog, sources } = regionalCatalog({ crate_paris: "paris" });
  const blind = await bakeCatalog(catalog, async (path) => sources[path], { authority: AUTHORITY });
  expect(errorsOf(blind, "crate_paris")).toEqual(["structure.regional_family"]);
});

/** Paris and New York looks of the crate, baked and served from memory with
 *  every fetch recorded. */
async function served() {
  const result = await bake(regionalCatalog({ crate_paris: "paris", crate_ny: "new_york" }));
  expect(result.ok).toBe(true);
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
  const file = (name: string) => {
    const hash = result.runtime.appearances[name].bundle;
    return bundlePath(gzipTransport(result.runtime, hash).hash);
  };
  return { fetched, loader: new AppearanceLibrary(fetcher), file };
}

const NO_BUILDINGS = {
  templates: [],
  template: new Uint16Array(),
  frames: new Float64Array(),
  owners: new Uint32Array(),
};

test("the catalog load leaves the regional looks to the maps of their family", async () => {
  const { loader, fetched, file } = await served();
  const catalog = await loader.load(BASE);
  expect(catalog.appearances.has("crate")).toBe(true);
  expect(catalog.appearances.has("crate_paris")).toBe(false);
  expect(fetched).not.toContain(file("crate_paris"));
  expect(fetched).not.toContain(file("crate_ny"));
  expect([...catalog.onRequest].sort()).toEqual([
    ["crate_ny", "new_york"],
    ["crate_paris", "paris"],
  ]);
});

test("a map of one family fetches its family's looks and no other family's", async () => {
  const { loader, fetched, file } = await served();
  const catalog = await loader.load(BASE);
  fetched.length = 0;
  const paris = await loader.withAppearances(mapDownloads(catalog, NO_BUILDINGS, "paris"));
  expect(fetched).toEqual([file("crate_paris")]);
  expect(paris.appearances.get("crate_paris")?.regionalFamily).toBe("paris");
  expect(paris.appearances.has("crate_ny")).toBe(false);
  // What it fetched is what its props draw.
  const layout = {
    propAppearance: { crate: { drawn_by: "crate" } },
    blockingPropKinds: {},
    unitAppearance: {},
  };
  const box = {
    kind: "crate",
    center: [0, 0] as const,
    yaw: 0,
    half: [5, 4, 3] as const,
    baseZ: 0,
  };
  expect(new PropAppearances(paris, layout, "paris").choose({ kind: "crate", half: box.half })?.name).toBe(
    "crate_paris",
  );
  // A map of no family fetches no regional look, and draws the shared one.
  fetched.length = 0;
  const none = await loader.withAppearances(mapDownloads(catalog, NO_BUILDINGS, null));
  expect(fetched).toEqual([]);
  expect(new PropAppearances(none, layout, null).choose({ kind: "crate", half: box.half })?.name).toBe("crate");
});
