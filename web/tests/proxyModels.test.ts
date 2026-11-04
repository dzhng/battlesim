// @vitest-environment node
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { expect, test, vi } from "vitest";
import { joinTextures } from "@packages/scene-assets/src/codec";
import { PROXY_MESHES } from "@packages/battle-renderer/src/proxies";
import { proxyModels } from "../src/battle/render/proxyModels";

// Control the authored mesh colour at the asset boundary: this checks colour
// conversion independently of any later change to primitive art.
vi.mock("@packages/battle-renderer/src/proxies", async (importOriginal) => {
  const original = await importOriginal<typeof import("@packages/battle-renderer/src/proxies")>();
  const white = (source: Float32Array) => {
    const mesh = source.slice();
    for (let v = 0; v < mesh.length; v += 10) mesh.fill(1, v + 6, v + 9);
    return mesh;
  };
  return {
    ...original,
    PROXY_MESHES: {
      box: white(original.PROXY_MESHES.box),
      marker: white(original.PROXY_MESHES.marker),
    },
  };
});

test("proxy admission preserves drawn shapes, pre-gamma color, transforms and pick-only infantry", () => {
  const generated = proxyModels([
    { kind: "infantry", x: 9, y: 8, z: 7, yaw: 1, color: [1, 1, 1] },
    { kind: "box", x: 2, y: 3, z: 4, yaw: 0.7, color: [0.5, 0.25, 0.75] },
    { kind: "box", x: -2, y: -3, z: 0, yaw: 0, color: [0.5, 0.25, 0.75] },
    { kind: "marker", x: 5, y: 6, z: 0, yaw: -0.3, color: [1, 1, 1] },
  ]);
  expect(generated.assets).toHaveLength(2);
  expect(generated.models).toHaveLength(3);
  expect(generated.models[0]).toMatchObject({
    x: 2,
    y: 3,
    z: 4,
    yaw: 0.7,
    pose: { kind: "rest" },
    tier: 0,
  });
  expect(generated.models[0].appearance).toBe(generated.models[1].appearance);
  for (const [row, kind, color] of [
    [0, "box", [0.217638, 0.047366, 0.531049, 1]],
    [1, "marker", [1, 1, 1, 1]],
  ] as const) {
    const asset = generated.assets[row];
    const bundle = joinTextures(asset.bundle, []);
    expect(bundle.kind).toBe("articulated");
    if (bundle.kind !== "articulated") throw new Error("expected articulated proxy");
    const source = PROXY_MESHES[kind];
    const mesh = bundle.nodes[0].tiers[0];
    for (let v = 0; v < source.length / 10; v++) {
      expect(Array.from(mesh.positions.subarray(v * 3, v * 3 + 3))).toEqual(
        Array.from(source.subarray(v * 10, v * 10 + 3)),
      );
      expect(Array.from(mesh.colors.subarray(v * 4, v * 4 + 4))).toEqual([255, 255, 255, 255]);
    }
    // Known linear colours for the controlled input, rather than a second
    // implementation of the conversion (including its float rounding).
    bundle.materials[0].base_color.forEach((channel, i) =>
      expect(channel).toBeCloseTo(color[i], 5),
    );
    expect(bundle.materials[0]).toMatchObject({
      roughness: Math.fround(0.85),
      metallic: 0,
      tint: 0,
    });
    expect(asset.textureIds).toEqual([]);
  }
});

test("proxy highlights retain one art identity and cannot overwrite unrelated installed art", () => {
  const instance = { kind: "box", x: 0, y: 0, z: 0, yaw: 0, color: [1, 1, 1] } as const;
  const generated = proxyModels([instance, { ...instance, highlight: true }]);
  expect(generated.assets).toHaveLength(1);
  expect(generated.models.map((model) => model.highlight)).toEqual([undefined, true]);
  expect(() => proxyModels([instance], [generated.assets[0].name])).toThrow(
    "collides with installed art",
  );
  expect(proxyModels([])).toEqual({ assets: [], models: [] });
});

test("parsed model transport round-trips art and shares textures within one admission", async () => {
  const { modelAssetsOf } = await import("../src/battle/render/modelAssets");
  const { decodeTexture } = await import("@packages/scene-assets/src/texture");
  const generated = proxyModels([{ kind: "box", x: 0, y: 0, z: 0, yaw: 0, color: [1, 1, 1] }]);
  const bundle = joinTextures(generated.assets[0].bundle, []);
  if (bundle.kind !== "articulated") throw new Error("expected articulated model");
  const texture = {
    id: "shared-albedo",
    format: "rgba8unorm-srgb",
    width: 4,
    height: 4,
    levels: [4, 2, 1].map((edge) =>
      Uint8Array.from({ length: edge * edge * 4 }, (_, i) => [32, 64, 128, 255][i % 4]),
    ),
  } as const;
  bundle.textures = [{ ...texture, levels: [...texture.levels] }];
  bundle.materials[0].textures = { albedo: 0 };
  const appearance = {
    bundle,
    unit: "vehicle",
    scenery: null,
    footprint: null,
    mounts: null,
    regionalFamily: null,
    paints: null,
    wreck: null,
    factions: null,
  } as const;
  const installed: InstalledAppearances = {
    generation: 1,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    skeletons: new Map(),
    appearances: new Map([
      ["a", appearance],
      ["b", appearance],
    ]),
    onRequest: new Map(),
  };
  const assets = modelAssetsOf(installed, ["a", "b", "a"]);
  expect(assets.map((asset) => asset.name)).toEqual(["a", "b"]);
  expect(assets[0].textureBytes[0]).toBe(assets[1].textureBytes[0]);
  const pixels = decodeTexture("shared-albedo", assets[0].textureBytes[0]);
  expect(pixels.levels).toEqual(texture.levels);
  expect(joinTextures(assets[0].bundle, [pixels])).toEqual(bundle);
  expect(() => modelAssetsOf(installed, ["missing"])).toThrow("missing is not installed");
});
