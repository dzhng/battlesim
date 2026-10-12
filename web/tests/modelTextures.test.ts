// @vitest-environment node
import { expect, test } from "vitest";
import { uploadTextureArray } from "@packages/battle-renderer/src/models/modelTextures";
import type { Texture } from "@packages/scene-assets/src/schema";

const texture = (id: string): Texture => ({
  id,
  format: "rgba8unorm-srgb",
  width: 1,
  height: 1,
  levels: [new Uint8Array(4)],
});

test("texture-array uploads refuse an adapter layer overflow before allocation", () => {
  const device = { limits: { maxTextureArrayLayers: 1 } } as GPUDevice;
  expect(() =>
    uploadTextureArray(device, "model-albedo", "rgba8unorm-srgb", [texture("a"), texture("b")]),
  ).toThrow("maxTextureArrayLayers (1)");
});
