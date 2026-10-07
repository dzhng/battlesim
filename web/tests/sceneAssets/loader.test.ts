// @vitest-environment node
// The one loader: installs a whole catalog generation at once, verifies every
// content hash, and keeps what is installed when anything fails.
import { expect, test } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import {
  bundlePath,
  CATALOG_LOAD_MAX_BYTES,
  KIT_BUNDLE_MAX_BYTES,
  MAP_DOWNLOAD_MAX_BYTES,
} from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, testCatalog, testSources } from "./synthetic";

async function served() {
  const sources = testSources();
  const result = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  const files = new Map<string, Uint8Array>([
    ["catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime))],
    ...result.files,
  ]);
  return { result, files, library: new AppearanceLibrary(memoryFetch(files, "/assets/")) };
}

test("a catalog installs as one generation with every appearance and skeleton", async () => {
  const { library } = await served();
  const installed = await library.load("/assets/");
  expect(installed.generation).toBe(1);
  expect([...installed.appearances.keys()].sort()).toEqual([
    "crate",
    "rifleman",
    "tank",
    "tank_wreck",
    "truck",
    "truck_wreck",
  ]);
  expect(installed.appearances.get("tank")?.bundle.kind).toBe("articulated");
  expect(installed.skeletons.get("test-rig")?.clips.map((c) => c.name)).toContain("walk");
});

test("a static appearance arrives with the simulation box its art is authored to", async () => {
  const { library } = await served();
  const installed = await library.load("/assets/");
  expect(installed.appearances.get("crate")?.footprint).toEqual([5, 4, 3]);
  expect(installed.appearances.get("tank")?.footprint).toBeNull();
});

test("a bundle whose bytes do not match its hash fails the load and keeps the installed generation", async () => {
  const { library, files, result } = await served();
  const first = await library.load("/assets/");
  const tankFile = bundlePath(result.runtime.gzip![result.runtime.appearances.tank.bundle].hash);
  const corrupt = files.get(tankFile)!.slice();
  corrupt[corrupt.length - 1] ^= 0xff;
  files.set(tankFile, corrupt);
  await expect(library.load("/assets/")).rejects.toThrow(/content hash/);
  expect(library.installed).toBe(first);
  expect(library.installed?.appearances.size).toBe(6);
});

test("an LFS pointer served in place of a bundle names the exact pull", async () => {
  const { library, files, result } = await served();
  const hash = result.runtime.appearances.truck.bundle;
  const wire = result.runtime.gzip![hash].hash;
  files.set(
    bundlePath(wire),
    new TextEncoder().encode(
      `version https://git-lfs.github.com/spec/v1\noid sha256:${hash}\nsize 999\n`,
    ),
  );
  await expect(library.load("/assets/")).rejects.toThrow(
    `git lfs pull --include="assets/runtime/${wire}/bundle.bin"`,
  );
  expect(library.installed).toBeNull();
});

test("a skinned bundle whose skeleton is missing from the catalog is refused", async () => {
  const { library, files } = await served();
  const catalog = JSON.parse(new TextDecoder().decode(files.get("catalog.json")));
  catalog.skeletons = {};
  files.set("catalog.json", new TextEncoder().encode(JSON.stringify(catalog)));
  await expect(library.load("/assets/")).rejects.toThrow(/skeleton test-rig is not in the catalog/);
});

async function servedCity() {
  const { KIT, cityCatalog, cityContext, citySources, testSet, kitGlb } = await import("./city");
  const sources = citySources(testSet(), kitGlb());
  const result = await bakeCatalog(cityCatalog(), async (path) => sources[path], cityContext());
  const files = new Map(result.files);
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  return { KIT, result, files };
}

test("gzip kit and library transport installs the exact original art through immutable encoded URLs", async () => {
  const { gunzipSync } = await import("node:zlib");
  const { encodeBundle } = await import("@packages/scene-assets/src/codec.ts");
  const { sha256Hex } = await import("@packages/scene-assets/src/glb.ts");
  const { templateLibraryPath } = await import("@packages/scene-assets/src/schema.ts");
  const { encodeTemplateLibrary } = await import("@packages/scene-assets/src/templateLibrary.ts");
  const { KIT, result, files } = await servedCity();
  const kitHash = result.runtime.appearances[KIT].bundle;
  const kitPath = bundlePath(result.runtime.gzip![kitHash].hash);
  const artHash = result.runtime.templates!.library;
  const artPath = templateLibraryPath(result.runtime.gzip![artHash].hash);
  const fetch = memoryFetch(files, "/assets/");
  const requests: string[] = [];
  const library = new AppearanceLibrary(async (url) => {
    requests.push(url);
    return fetch(url);
  });
  await library.load("/assets/");
  const [installed, also] = await Promise.all([
    library.withAppearances([KIT]),
    library.withAppearances([KIT]),
  ]);
  expect(await sha256Hex(encodeBundle(installed.appearances.get(KIT)!.bundle))).toBe(kitHash);
  expect(also.appearances.get(KIT)!.bundle).toEqual(installed.appearances.get(KIT)!.bundle);
  expect(requests.filter((url) => url === `/assets/${kitPath}`)).toHaveLength(1);
  expect(encodeTemplateLibrary(installed.templates!.library)).toEqual(
    new Uint8Array(gunzipSync(files.get(artPath)!)),
  );
});

test("the baker publishes every bundle and the library as one gzip object, named apart from the art", async () => {
  const { gunzipSync } = await import("node:zlib");
  const { sha256Hex } = await import("@packages/scene-assets/src/glb.ts");
  const { templateLibraryPath } = await import("@packages/scene-assets/src/schema.ts");
  const { KIT, cityCatalog, cityContext, citySources, testSet, kitGlb } = await import("./city");
  const sources = citySources(testSet(), kitGlb());
  const result = await bakeCatalog(cityCatalog(), async (path) => sources[path], cityContext());
  const unit = (await served()).result;
  for (const [baked, hash, path] of [
    [result, result.runtime.appearances[KIT].bundle, bundlePath],
    [result, result.runtime.templates!.library, templateLibraryPath],
    [unit, unit.runtime.appearances.tank.bundle, bundlePath],
    [unit, unit.runtime.skeletons["test-rig"], bundlePath],
  ] as const) {
    const wire = baked.runtime.gzip?.[hash];
    expect(wire, hash).toBeDefined();
    expect(baked.files.has(path(hash))).toBe(false);
    const encoded = baked.files.get(path(wire!.hash))!;
    expect(await sha256Hex(encoded)).toBe(wire!.hash);
    expect(gunzipSync(encoded).length).toBe(wire!.raw_bytes);
  }
  // The library's content is its art; a bundle's is joined with its textures first.
  const library = result.runtime.gzip![result.runtime.templates!.library];
  expect(await sha256Hex(gunzipSync(result.files.get(templateLibraryPath(library.hash))!))).toBe(
    result.runtime.templates!.library,
  );
});

test("gzip inflation beyond the declared kit length is refused without consuming the installed generation", async () => {
  const { KIT, result, files } = await servedCity();
  const hash = result.runtime.appearances[KIT].bundle;
  result.runtime.gzip![hash].raw_bytes -= 1;
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  const library = new AppearanceLibrary(memoryFetch(files, "/assets/"));
  const installed = await library.load("/assets/");
  await expect(library.withAppearances([KIT])).rejects.toThrow(/exceeds decoded length/);
  expect(library.installed).toBe(installed);
  expect(library.installed!.appearances.has(KIT)).toBe(false);
});

test.each([
  ["wire length", /gzip length/],
  ["wire hash", /gzip content hash/],
  ["truncated gzip", /kit "city_kit_test"/],
  ["raw length", /decoded length/],
  ["raw hash", /decoded content hash/],
  ["missing record", /missing gzip transport/],
  ["raw kit budget", /invalid gzip lengths/],
] as const)(
  "invalid %s is refused atomically before its art is installed",
  async (failure, message) => {
    const { KIT, result, files } = await servedCity();
    const { packGzip } = await import("@packages/scene-assets/src/gzip.ts");
    const { gunzipSync } = await import("node:zlib");
    const { sha256Hex } = await import("@packages/scene-assets/src/glb.ts");
    const hash = result.runtime.appearances[KIT].bundle;
    const record = result.runtime.gzip![hash];
    const path = bundlePath(record.hash);
    const encoded = files.get(path)!.slice();
    if (failure === "wire length") files.set(path, encoded.subarray(1));
    if (failure === "wire hash") {
      encoded[encoded.length - 1] ^= 0xff;
      files.set(path, encoded);
    }
    if (failure === "raw length") record.raw_bytes += 1;
    if (failure === "missing record") delete result.runtime.gzip![hash];
    if (failure === "raw kit budget") record.raw_bytes = KIT_BUNDLE_MAX_BYTES + 1;
    if (failure === "truncated gzip") {
      const truncated = encoded.subarray(0, encoded.length - 8);
      record.hash = await sha256Hex(truncated);
      record.bytes = truncated.length;
      files.set(bundlePath(record.hash), truncated);
    }
    if (failure === "raw hash") {
      const raw = new Uint8Array(gunzipSync(encoded));
      raw[raw.length - 1] ^= 0xff;
      const repacked = await packGzip(raw);
      result.runtime.gzip![hash] = repacked.transport;
      files.set(bundlePath(repacked.transport.hash), repacked.bytes);
    }
    files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
    const library = new AppearanceLibrary(memoryFetch(files, "/assets/"));
    const installed = await library.load("/assets/");
    await expect(library.withAppearances([KIT])).rejects.toThrow(message);
    expect(library.installed).toBe(installed);
    expect(library.installed!.appearances.has(KIT)).toBe(false);
  },
);

test("a map's complete selected kit download is refused before any kit request when it exceeds the aggregate budget", async () => {
  const { KIT, result, files } = await servedCity();
  const hash = result.runtime.appearances[KIT].bundle;
  result.runtime.gzip![hash].bytes = MAP_DOWNLOAD_MAX_BYTES + 1;
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  const fetch = memoryFetch(files, "/assets/");
  const requested: string[] = [];
  const loader = new AppearanceLibrary(async (url) => {
    requested.push(url);
    return fetch(url);
  });
  const installed = await loader.load("/assets/");
  requested.length = 0;
  await expect(loader.withAppearances([KIT])).rejects.toThrow(/map download .* over/);
  expect(requested).toEqual([]);
  expect(loader.installed).toBe(installed);
});

test("an oversized catalog load is refused before fetching it and preserves the installed generation", async () => {
  const { result, files } = await servedCity();
  const fetch = memoryFetch(files, "/assets/");
  const requests: string[] = [];
  const loader = new AppearanceLibrary(async (url) => {
    requests.push(url);
    return fetch(url);
  });
  const installed = await loader.load("/assets/");
  result.runtime.gzip![result.runtime.templates!.library].bytes = CATALOG_LOAD_MAX_BYTES + 1;
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  requests.length = 0;
  await expect(loader.load("/assets/")).rejects.toThrow(/catalog load .* over/);
  expect(requests).toEqual(["/assets/catalog.json"]);
  expect(loader.installed).toBe(installed);
});
