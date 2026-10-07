// @vitest-environment node
// Runtime transport: a texture is its own content-addressed file, fetched once
// however many bundles use it; every bundle travels gzipped with its original
// content hash kept as its art identity; what a page downloads with the
// catalog is counted and held to a limit; and the bake counts the distinct
// texture layers each GPU array would hold.
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { encodeBundle } from "@packages/scene-assets/src/codec.ts";
import { sha256Hex } from "@packages/scene-assets/src/glb.ts";
import { catalogLoadBytes } from "@packages/scene-assets/src/gzip.ts";
import { textureLayerFindings } from "@packages/scene-assets/src/texture.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import {
  CATALOG_LOAD_MAX_BYTES,
  type ArticulatedBundle,
  type Catalog,
  type RuntimeCatalog,
} from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, tankGlb, testCatalog, testSources } from "./synthetic";

/** The test catalog with a second vehicle painted in the tank's textures. */
async function sharedPaint() {
  const catalog: Catalog = testCatalog();
  catalog.appearances.tank_antenna = { ...catalog.appearances.tank, source: "tank-antenna.glb" };
  const sources: Record<string, Uint8Array> = {
    ...testSources(),
    "assets/source/test-tank.glb": tankGlb({ textures: { size: 16 } }),
    "tank-antenna.glb": tankGlb({ textures: { size: 16 }, antenna: 2.6 }),
  };
  const result = await bakeCatalog(catalog, async (path) => sources[path], {
    authority: AUTHORITY,
  });
  expect(result.ok).toBe(true);
  const files = new Map(result.files);
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  return { result, files };
}

test("a texture two bundles share is one runtime file, fetched once, and each bundle keeps its art hash", async () => {
  const { result, files } = await sharedPaint();
  const { tank, tank_antenna } = result.runtime.appearances;
  expect(tank.bundle).not.toBe(tank_antenna.bundle);
  const fetch = memoryFetch(files, "/assets/");
  const requests: string[] = [];
  const library = new AppearanceLibrary(async (url) => {
    requests.push(url);
    return fetch(url);
  });
  const installed = await library.load("/assets/");
  const a = installed.appearances.get("tank")!.bundle as ArticulatedBundle;
  const b = installed.appearances.get("tank_antenna")!.bundle as ArticulatedBundle;
  expect(b.textures.map((t) => t.id)).toEqual(a.textures.map((t) => t.id));
  expect(a.textures).toHaveLength(3);
  // The installed bundles are exactly the art the bake hashed.
  expect(await sha256Hex(encodeBundle(a))).toBe(tank.bundle);
  expect(await sha256Hex(encodeBundle(b))).toBe(tank_antenna.bundle);
  // Each texture is stored once, and fetched once, for both bundles.
  const textureFiles = [...files.keys()].filter((path) => path.endsWith("/texture.bin"));
  expect(textureFiles).toHaveLength(3);
  for (const path of textureFiles)
    expect(requests.filter((url) => url === `/assets/${path}`)).toHaveLength(1);
  // A bundle travels without its textures' pixels.
  const pixels = a.textures.reduce(
    (n, t) => n + t.levels.reduce((m, level) => m + level.byteLength, 0),
    0,
  );
  expect(result.runtime.gzip![tank.bundle].raw_bytes).toBeLessThanOrEqual(
    encodeBundle(a).byteLength - pixels,
  );
});

test("the bake counts each array's distinct texture layers, a shared texture once", async () => {
  const { result } = await sharedPaint();
  expect(result.textureLayers).toEqual({ albedo: 1, surface: 2 });
});

test("an array with more distinct layers than the limit is a bake error naming the array", () => {
  expect(textureLayerFindings({ albedo: 3, surface: 4 }, 4)).toEqual([]);
  const [finding] = textureLayerFindings({ albedo: 3, surface: 5 }, 4);
  expect(finding.severity).toBe("error");
  expect(finding.message).toMatch(/surface.*5.*4/);
});

test("the catalog load counts each texture once and refuses an oversized load before fetching", async () => {
  const { result, files } = await sharedPaint();
  const once = catalogLoadBytes(result.runtime);
  // Every served file is fetched by a catalog load, each once.
  const served = [...result.files.values()].reduce((n, bytes) => n + bytes.byteLength, 0);
  expect(once).toBe(served);
  const heavy: RuntimeCatalog = structuredClone(result.runtime);
  const id = heavy.textures![heavy.appearances.tank.bundle][0];
  heavy.gzip![id].bytes = CATALOG_LOAD_MAX_BYTES;
  expect(catalogLoadBytes(heavy)).toBeGreaterThan(CATALOG_LOAD_MAX_BYTES);
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(heavy)));
  const fetch = memoryFetch(files, "/assets/");
  const requests: string[] = [];
  const library = new AppearanceLibrary(async (url) => {
    requests.push(url);
    return fetch(url);
  });
  await expect(library.load("/assets/")).rejects.toThrow(/catalog load .* over/);
  expect(requests).toEqual(["/assets/catalog.json"]);
});

test("the shipped catalog's load is within its limit", () => {
  const catalog = JSON.parse(
    readFileSync(new URL("../../../assets/runtime/catalog.json", import.meta.url), "utf8"),
  ) as RuntimeCatalog;
  const bytes = catalogLoadBytes(catalog);
  expect(bytes).toBeGreaterThan(0);
  expect(bytes).toBeLessThanOrEqual(CATALOG_LOAD_MAX_BYTES);
});
