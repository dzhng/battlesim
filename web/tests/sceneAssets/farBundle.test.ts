// @vitest-environment node
// The far mesh's cross-language fixture: the bake of the synthetic far prop
// publishes exactly the committed bundle the Rust renderer's test decodes and
// draws at its far tier (`crates/renderer-core/tests/far_mesh.rs`). Each side
// holds the same bytes, so neither language's far index can drift alone.
//
//   UPDATE_FAR_BUNDLE=1 bun run --cwd web test tests/sceneAssets/farBundle.test.ts
//
// rewrites the fixture from today's bake; review the Rust test after it.
import { readFileSync, writeFileSync } from "node:fs";
import { expect, test } from "vitest";
import { bakeCatalog } from "@packages/scene-assets/src/bake.ts";
import { encodeBundle } from "@packages/scene-assets/src/codec.ts";
import { readGzip } from "@packages/scene-assets/src/gzip.ts";
import { sha256Hex } from "@packages/scene-assets/src/glb.ts";
import { bundlePath, FAR_TIER } from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, bakedBundle, testCatalog, testSources } from "./synthetic";
import { farPropGlb } from "./farProp";

const FIXTURE = new URL(
  "../../../crates/renderer-core/tests/fixtures/far_prop.bundle",
  import.meta.url,
);

test("the bake of the synthetic far prop publishes the committed far-mesh bundle", async () => {
  const sources: Record<string, Uint8Array> = {
    ...testSources(),
    "assets/source/test-crate.glb": farPropGlb(1),
  };
  const result = await bakeCatalog(testCatalog(), async (path) => sources[path], {
    authority: AUTHORITY,
  });
  const hash = result.runtime.appearances.crate.bundle;
  const read = async (path: string) => {
    const bytes = result.files.get(path);
    if (!bytes) throw new Error(`${path}: not baked`);
    return bytes;
  };
  // The content its runtime file inflates to: what every Rust reader takes.
  // It names no texture files, so it is all a reader needs.
  const bytes = await readGzip(result.runtime, "bundle", hash, bundlePath, read);
  expect(result.runtime.textures?.[hash] ?? []).toEqual([]);
  // Read back through the transport, it is the art the bake named, with a
  // mesh at the far tier in its one state.
  const bundle = await bakedBundle(result, hash);
  expect(await sha256Hex(encodeBundle(bundle))).toBe(hash);
  if (bundle.kind !== "static") throw new Error(bundle.kind);
  expect(bundle.states.map((s) => s.tiers.length)).toEqual([FAR_TIER + 1]);
  if (process.env.UPDATE_FAR_BUNDLE === "1") writeFileSync(FIXTURE, bytes);
  expect(new Uint8Array(readFileSync(FIXTURE))).toEqual(bytes);
});
