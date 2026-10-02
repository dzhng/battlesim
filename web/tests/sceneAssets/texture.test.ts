// @vitest-environment node
// Baked textures ride the bundle. A textured source bakes to
// content-addressed textures with every mip level, the same source always to
// the same bytes, and the PNG it embeds decodes to exactly its pixels.
import { expect, test } from "vitest";
import { PNG } from "pngjs";
import { mat4 } from "math";
import { bakeCatalog } from "@packages/scene-assets/src/bake.ts";
import { mergeParts } from "@packages/scene-assets/src/build.ts";
import { decodeBundle, encodeBundle } from "@packages/scene-assets/src/codec.ts";
import { decodePng, mipChain } from "@packages/scene-assets/src/texture.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import { bundlePath, type ArticulatedBundle } from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, TANK_DRAWS, TOLERANCES, tankGlb, testCatalog, testSources } from "./synthetic";

async function texturedTank(size = 8) {
  const result = await validateAppearance(
    {
      name: "tank",
      entry: { unit: "vehicle", source: "tank.glb", basis_yaw_deg: 0, mounts: TANK_DRAWS },
      files: { "tank.glb": tankGlb({ textures: { size } }) },
    },
    { authority: AUTHORITY, tolerances: TOLERANCES },
  );
  expect(result.findings).toEqual([]);
  return result.bundle as ArticulatedBundle;
}

test("a textured source bakes every channel, with every mip level, onto its material", async () => {
  const bundle = await texturedTank(8);
  const paint = bundle.materials.find((m) => m.name === "paint")!;
  expect(Object.keys(paint.textures!).sort()).toEqual(["albedo", "normal", "orm"]);
  const albedo = bundle.textures[paint.textures!.albedo!];
  expect(albedo.format).toBe("rgba8unorm-srgb");
  expect(bundle.textures[paint.textures!.normal!].format).toBe("rgba8unorm");
  expect(albedo.levels.map((l) => l.byteLength)).toEqual([8 * 8 * 4, 4 * 4 * 4, 2 * 2 * 4, 4]);
  // Level 0 is the embedded image, untouched.
  expect(Array.from(albedo.levels[0].subarray(0, 8))).toEqual([0, 0, 90, 200, 37, 0, 90, 200]);
});

test("meshes of a textured source carry unit tangents with a handedness", async () => {
  const bundle = await texturedTank();
  const hull = bundle.nodes.find((n) => n.name === "hull")!.tiers[0];
  const t = hull.tangents!;
  expect(t.length).toBe((hull.positions.length / 3) * 4);
  for (let v = 0; v < t.length; v += 4) {
    expect(Math.hypot(t[v], t[v + 1], t[v + 2]) / 32767).toBeCloseTo(1, 3);
    expect(Math.abs(t[v + 3])).toBe(32767);
  }
});

test("a sliver triangle's missing tangent becomes any unit direction across its normal", () => {
  const primitive = {
    positions: Float32Array.of(0, 0, 0, 1, 0, 0, 2, 0, 0),
    normals: Float32Array.of(0, 0, 1, 0, 0, 1, 0, 1, 0),
    tangents: Float32Array.of(1, 0, 0, 1, 0, 0, 0, -1, 0, 0, 0, -1),
    uvs: new Float32Array(6),
    colors: new Float32Array(12).fill(1),
    joints: null,
    weights: null,
    indices: Uint32Array.of(0, 1, 2),
    material: 0,
  };
  const mesh = mergeParts([{ primitive, material: 0, transform: mat4.create() }], false);
  const t = Array.from(mesh.tangents!, (x) => x / 32767);
  expect(t.slice(0, 4)).toEqual([1, 0, 0, 1]);
  for (const v of [1, 2]) {
    const [x, y, z, w] = t.slice(v * 4, v * 4 + 4);
    const n = Array.from(primitive.normals.subarray(v * 3, v * 3 + 3));
    expect(Math.hypot(x, y, z)).toBeCloseTo(1, 4);
    expect(x * n[0] + y * n[1] + z * n[2]).toBeCloseTo(0, 4);
    expect(Math.abs(w)).toBe(1);
  }
});

test("the same textured source gives the same bundle hash, and survives the codec", async () => {
  const hashes = [];
  for (let i = 0; i < 2; i++) {
    const sources: Record<string, Uint8Array> = {
      ...testSources(),
      "assets/source/test-tank.glb": tankGlb({ textures: { size: 16 } }),
    };
    const result = await bakeCatalog(testCatalog(), async (p) => sources[p], {
      authority: AUTHORITY,
    });
    expect(result.ok).toBe(true);
    const hash = result.runtime.appearances.tank.bundle;
    const decoded = decodeBundle(result.files.get(bundlePath(hash))!) as ArticulatedBundle;
    expect(decoded.textures.length).toBe(3);
    expect(encodeBundle(decoded)).toEqual(result.files.get(bundlePath(hash)));
    hashes.push(hash);
  }
  expect(hashes[1]).toBe(hashes[0]);
});

test("a texture's address follows its content", async () => {
  const [a, b] = [await texturedTank(8), await texturedTank(16)];
  expect(a.textures.map((t) => t.id)).toEqual((await texturedTank(8)).textures.map((t) => t.id));
  expect(a.textures[0].id).not.toBe(b.textures[0].id);
});

test("PNGs decode to their exact pixels, RGBA and RGB alike", async () => {
  for (const colorType of [6, 2] as const) {
    const png = new PNG({ width: 5, height: 3, colorType, inputHasAlpha: true });
    for (let i = 0; i < 15; i++)
      png.data.set([i * 17, 255 - i * 9, (i * 53) & 255, colorType === 6 ? 100 + i : 255], i * 4);
    const decoded = await decodePng(new Uint8Array(PNG.sync.write(png, { colorType })));
    expect([decoded.width, decoded.height]).toEqual([5, 3]);
    for (let i = 0; i < 15; i++) {
      const at = Array.from(decoded.pixels.subarray(i * 4, i * 4 + 4));
      expect(at).toEqual([i * 17, 255 - i * 9, (i * 53) & 255, colorType === 6 ? 100 + i : 255]);
    }
  }
});

test("mips average colour in linear light and keep normals unit length", () => {
  const image = {
    width: 2,
    height: 2,
    pixels: Uint8Array.from([0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255]),
  };
  // Half black, half white is linear 0.5: sRGB 188, not 128.
  expect(Array.from(mipChain(image, "albedo")[1])).toEqual([188, 188, 188, 128]);
  expect(Array.from(mipChain(image, "orm")[1])).toEqual([128, 128, 128, 128]);
  const tilted = {
    width: 2,
    height: 2,
    pixels: Uint8Array.from([
      255, 128, 128, 255, 128, 128, 255, 255, 255, 128, 128, 255, 128, 128, 255, 255,
    ]),
  };
  const [x, y, z] = Array.from(
    mipChain(tilted, "normal")[1].subarray(0, 3),
    (c) => (c / 255) * 2 - 1,
  );
  expect(Math.hypot(x, y, z)).toBeCloseTo(1, 1);
  expect(x).toBeCloseTo(z, 1);
});
