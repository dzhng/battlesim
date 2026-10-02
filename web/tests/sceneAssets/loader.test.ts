// @vitest-environment node
// The one loader: installs a whole catalog generation at once, verifies every
// content hash, and keeps what is installed when anything fails.
import { expect, test } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { bundlePath } from "@packages/scene-assets/src/schema.ts";
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
  expect([...installed.appearances.keys()].sort()).toEqual(["crate", "rifleman", "tank", "truck"]);
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
  const tankFile = bundlePath(result.runtime.appearances.tank.bundle);
  const corrupt = files.get(tankFile)!.slice();
  corrupt[corrupt.length - 1] ^= 0xff;
  files.set(tankFile, corrupt);
  await expect(library.load("/assets/")).rejects.toThrow(/content hash/);
  expect(library.installed).toBe(first);
  expect(library.installed?.appearances.size).toBe(4);
});

test("an LFS pointer served in place of a bundle names the exact pull", async () => {
  const { library, files, result } = await served();
  const hash = result.runtime.appearances.truck.bundle;
  files.set(
    bundlePath(hash),
    new TextEncoder().encode(
      `version https://git-lfs.github.com/spec/v1\noid sha256:${hash}\nsize 999\n`,
    ),
  );
  await expect(library.load("/assets/")).rejects.toThrow(
    `git lfs pull --include="assets/runtime/${hash}/bundle.bin"`,
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
