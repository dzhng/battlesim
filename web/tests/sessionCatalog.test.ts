// @vitest-environment node
// A battle session draws, reads and loads exactly the units its own catalog
// holds: a lab's test set adds units the game's never sees, and a session
// handed a unit its catalog lacks refuses it by name instead of drawing
// nothing.
import { expect, test } from "vitest";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { bundleFiles, catalogLoadBytes } from "@packages/scene-assets/src/gzip.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { ownDocuments } from "@web/battle/catalog/node";
import { admitScenario, catalogSet, resolveCatalogSet } from "@web/battle/catalog/sets";
import { AUTHORITY, tankGlb, testCatalog, testSources } from "./sceneAssets/synthetic";

/** A fake test-unit document: a tank only a lab's catalog holds. */
const LAB_ONLY = JSON.stringify({
  units: { lab_only_tank: { extends: "test_tank", name: "Lab-only tank", appearance: "tank" } },
});

const scenario = (kind: string) => ({
  units: [{ side: "blue", kind, position: [0, 0] }],
});

async function installed() {
  const sources = testSources();
  const result = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  expect(result.ok).toBe(true);
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  return new AppearanceLibrary(memoryFetch(files, "/assets/")).load("/assets/");
}

test("a lab session draws a unit only its test set holds", async () => {
  const catalog = await resolveCatalogSet("test", [...ownDocuments("test"), LAB_ONLY]);
  const units = admitScenario(scenario("lab_only_tank"), catalog);
  const drawn = new AppearanceCatalog(await installed(), units).resolve("lab_only_tank", "blue");
  expect(drawn?.appearance).toBe("tank");
  // The scenario's rules carry the same documents the session drew from, so
  // the authority runs the unit the page draws.
  expect(catalog.rules.catalog).toEqual(catalog.units.documents);
});

/** A bake of test art (`tank`, `truck`, and `lab_tank` painted in the tank's
 *  textures) beside scenery; no unit of the game set wears any of it.
 *  Fetches are recorded. */
async function sharedArt() {
  const catalog = testCatalog();
  catalog.appearances.lab_tank = {
    ...catalog.appearances.tank,
    source: "lab-tank.glb",
  };
  const sources: Record<string, Uint8Array> = {
    ...testSources(),
    "assets/source/test-tank.glb": tankGlb({ textures: { size: 16 } }),
    "lab-tank.glb": tankGlb({ textures: { size: 16 }, antenna: 2.6 }),
  };
  const result = await bakeCatalog(catalog, async (path) => sources[path], {
    authority: AUTHORITY,
  });
  expect(result.ok).toBe(true);
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  const served = memoryFetch(files, "/assets/");
  const requests: string[] = [];
  const library = new AppearanceLibrary(async (url) => {
    requests.push(url);
    return served(url);
  });
  return { runtime: result.runtime, files, library, requests };
}

const LAB_TANK = JSON.stringify({
  units: {
    lab_only_tank: {
      extends: "test_tank",
      name: "Lab-only tank",
      appearance: "lab_tank",
    },
  },
});

test("a game page's catalog load leaves out the art only test units wear, and counts what it fetches", async () => {
  const { runtime, files, library, requests } = await sharedArt();
  const game = await catalogSet("game");
  const installed = await library.load("/assets/", game.units.appearances);
  expect(installed.appearances.has("tank")).toBe(false);
  expect(installed.appearances.has("crate")).toBe(true);
  expect(installed.appearances.has("lab_tank")).toBe(false);
  expect(installed.appearances.has("truck")).toBe(false);
  const fetched = (name: string) =>
    requests.includes(`/assets/${bundleFiles(runtime, runtime.appearances[name].bundle)[0]}`);
  expect(fetched("tank")).toBe(false);
  expect(fetched("lab_tank")).toBe(false);
  expect(fetched("truck")).toBe(false);
  // What the load is counted as is what it fetched, each file once.
  const bytes = requests
    .filter((url) => url !== "/assets/catalog.json")
    .reduce((n, url) => n + files.get(url.slice("/assets/".length))!.byteLength, 0);
  expect(catalogLoadBytes(runtime, game.units.appearances)).toBe(bytes);
  expect(new Set(requests).size).toBe(requests.length);
});

test("a lab page after a game page adds its test units' art, fetching nothing twice", async () => {
  const { runtime, library, requests } = await sharedArt();
  await library.load("/assets/", (await catalogSet("game")).units.appearances);
  const lab = await resolveCatalogSet("test", [...ownDocuments("test"), LAB_TANK]);
  const installed = await library.withUnits(lab.units.appearances);
  expect(installed.appearances.has("lab_tank")).toBe(true);
  expect(installed.appearances.has("truck")).toBe(false);
  const units = admitScenario(scenario("lab_only_tank"), lab);
  expect(new AppearanceCatalog(installed, units).resolve("lab_only_tank", "blue")?.appearance).toBe(
    "lab_tank",
  );
  // The scenery came with the game's load and is not fetched again.
  expect(requests.filter((url) => url.endsWith("/catalog.json"))).toHaveLength(1);
  expect(new Set(requests).size).toBe(requests.length);
  expect(requests).toContain(
    `/assets/${bundleFiles(runtime, runtime.appearances.lab_tank.bundle)[0]}`,
  );
  // Asking again for what is held fetches nothing.
  const before = requests.length;
  expect(await library.withUnits(lab.units.appearances)).toBe(installed);
  expect(requests).toHaveLength(before);
});

test("a game session refuses a unit outside its catalog by name", async () => {
  const game = await catalogSet("game");
  expect(() => admitScenario(scenario("test_tank"), game)).toThrow(
    /test_tank.*not in the game catalog/,
  );
  expect(() => admitScenario(scenario("lab_only_tank"), game)).toThrow(
    /lab_only_tank.*not in the game catalog/,
  );
});
