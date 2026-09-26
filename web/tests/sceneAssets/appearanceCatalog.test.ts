// @vitest-environment node
// Which appearance the battle draws for a unit kind on a side: one bundle per
// kind, recoloured per side by the tint mask its materials carry.
import { expect, test } from "vitest";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import type { Catalog } from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, GltfBuilder, soldierGlb, testCatalog, testSources } from "./synthetic";

async function install(catalog: Catalog = testCatalog()) {
  const sources = testSources();
  const result = await bakeCatalog(catalog, async (path) => sources[path], {
    authority: AUTHORITY,
  });
  expect(result.ok).toBe(true);
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  return new AppearanceLibrary(memoryFetch(files, "/assets/")).load("/assets/");
}

test("each unit kind resolves to its one appearance, tinted by the side", async () => {
  const catalog = new AppearanceCatalog(await install());
  const sides = testCatalog().sides;
  expect(catalog.resolve("rifle", "blue")).toEqual({ appearance: "rifleman", tint: sides.blue });
  expect(catalog.resolve("rifle", "red")).toEqual({ appearance: "rifleman", tint: sides.red });
  expect(catalog.resolve("tank", "red")?.appearance).toBe("tank");
  expect(catalog.resolve("recon", "blue")).toBeNull();
});

test("two appearances for one unit kind are refused as ambiguous", async () => {
  const base = testCatalog();
  const catalog: Catalog = {
    ...base,
    appearances: { ...base.appearances, rifleman_b: { ...base.appearances.rifleman } },
  };
  await expect(install(catalog).then((i) => new AppearanceCatalog(i))).rejects.toThrow(
    /rifle.*rifleman.*rifleman_b|rifle.*rifleman_b.*rifleman/,
  );
});

test("a material's glTF extras tint is its side-tint mask weight", () => {
  const b = new GltfBuilder();
  b.json.materials.push({ name: "cloth", pbrMetallicRoughness: {}, extras: { tint: 0.6 } });
  b.roots(b.node({ name: "part", mesh: b.box([0, 0, 0], [1, 1, 1]) }));
  const { scene } = importScene(b.glb(), "tinted.glb", 0);
  expect(scene!.materials.map((m) => [m.name, m.tint])).toEqual([
    ["paint", 0],
    ["cloth", 0.6],
  ]);
  expect(importScene(soldierGlb(), "x.glb", 90).scene!.materials.every((m) => m.tint === 0)).toBe(
    true,
  );
});
