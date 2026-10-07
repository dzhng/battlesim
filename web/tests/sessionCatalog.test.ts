// @vitest-environment node
// A battle session draws, reads and loads exactly the units its own catalog
// holds: a lab's test set adds units the game's never sees, and a session
// handed a unit its catalog lacks refuses it by name instead of drawing
// nothing.
import { expect, test } from "vitest";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { ownDocuments } from "@web/battle/catalog/node";
import { admitScenario, catalogSet, resolveCatalogSet } from "@web/battle/catalog/sets";
import { AUTHORITY, testCatalog, testSources } from "./sceneAssets/synthetic";

/** A fake test-unit document: a tank only a lab's catalog holds. */
const LAB_ONLY = JSON.stringify({
  units: { lab_only_tank: { extends: "test_tank", name: "Lab-only tank", appearance: "tank" } },
});

const scenario = (kind: string) => ({ units: [{ side: "blue", kind, position: [0, 0] }] });

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

test("a game session refuses a unit outside its catalog by name", async () => {
  const game = await catalogSet("game");
  expect(() => admitScenario(scenario("test_tank"), game)).toThrow(
    /test_tank.*not in the game catalog/,
  );
  expect(() => admitScenario(scenario("lab_only_tank"), game)).toThrow(
    /lab_only_tank.*not in the game catalog/,
  );
});
