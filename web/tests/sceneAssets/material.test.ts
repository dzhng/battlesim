// @vitest-environment node
// Material transport: a source material says how much of its surface is there
// (opaque, cutout, blended) and whether it is a room behind a window, and the
// bundle carries exactly that. Wear and the tint mask keep their own channels.
import { expect, test } from "vitest";
import type { GltfJson } from "@packages/scene-assets/src/glb.ts";
import { sha256Hex } from "@packages/scene-assets/src/glb.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { buildClips } from "@packages/scene-assets/src/build.ts";
import { decodeBundle, encodeBundle } from "@packages/scene-assets/src/codec.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import {
  bundlePath,
  type ArticulatedBundle,
  type FindingCode,
  type Material,
  type StaticBundle,
} from "@packages/scene-assets/src/schema.ts";
import {
  AUTHORITY,
  SKELETON_ENTRY,
  TANK_DRAWS,
  TOLERANCES,
  panelGlb,
  soldierGlb,
  tankGlb,
  testCatalog,
  testSources,
  withJson,
  type GltfBuilder,
} from "./synthetic";

const context = { authority: AUTHORITY, tolerances: TOLERANCES };

/** A panel, validated as a static appearance with nothing to fit. */
async function panel(edit?: (material: GltfJson, b: GltfBuilder) => void) {
  return validateAppearance(
    {
      name: "panel",
      entry: {
        unit: "scenery",
        scenery: "hedgerow",
        states: { summer: "panel.glb" },
        basis_yaw_deg: 0,
      },
      files: { "panel.glb": panelGlb(edit) },
    },
    context,
  );
}

/** The panel's material after the whole trip: built, encoded and decoded. */
async function shipped(edit?: (material: GltfJson, b: GltfBuilder) => void): Promise<Material> {
  const built = await panel(edit);
  expect(built.findings).toEqual([]);
  const bundle = decodeBundle(encodeBundle(built.bundle!)) as StaticBundle;
  expect(bundle.materials.length).toBe(1);
  return bundle.materials[0];
}

/** A normal map whose alpha is the coverage: a grille, open on its left half. */
const grille = (m: GltfJson, b: GltfBuilder) => {
  m.normalTexture = { index: b.texture(4, (x) => [128, 128, 255, x < 2 ? 0 : 255]) };
};
const WEAR = [0.25, 0.125, 0.0625, 0.75];

test("a material that names no alpha mode is opaque, and no room", async () => {
  const material = await shipped();
  expect(material.coverage).toEqual({ kind: "opaque" });
  expect(material.interior).toBeUndefined();
});

test("a cutout ships its cutoff and its coverage texels, beside its wear and tint", async () => {
  const built = await panel((m, b) => {
    m.alphaMode = "MASK";
    m.alphaCutoff = 0.25;
    m.extras = { wear: WEAR, tint: 0.5 };
    grille(m, b);
  });
  expect(built.findings).toEqual([]);
  const bundle = decodeBundle(encodeBundle(built.bundle!)) as StaticBundle;
  const [material] = bundle.materials;
  expect(material.coverage).toEqual({ kind: "cutout", cutoff: 0.25 });
  expect(material.wear).toEqual(WEAR);
  expect(material.tint).toBe(0.5);
  const texels = bundle.textures[material.textures!.normal!].levels[0];
  expect(Array.from({ length: 4 }, (_, x) => texels[x * 4 + 3])).toEqual([0, 0, 255, 255]);
});

test("a mask that names no cutoff cuts at glTF's default, a half", async () => {
  const material = await shipped((m, b) => {
    m.alphaMode = "MASK";
    grille(m, b);
  });
  expect(material.coverage).toEqual({ kind: "cutout", cutoff: 0.5 });
});

test("a blended material ships its opacity in the base colour's alpha", async () => {
  const material = await shipped((m) => {
    m.alphaMode = "BLEND";
    m.pbrMetallicRoughness.baseColorFactor = [0.5, 0.75, 1, 0.25];
  });
  expect(material.coverage).toEqual({ kind: "blended" });
  expect(material.base_color).toEqual([0.5, 0.75, 1, 0.25]);
});

test("a room surface ships the interior sheet it looks up", async () => {
  for (const sheet of ["rooms", "shops"]) {
    const material = await shipped((m) => (m.extras = { interior: sheet }));
    expect(material.interior).toBe(sheet);
    expect(material.coverage).toEqual({ kind: "opaque" });
  }
});

test("an opaque material ignores its alphas: neither is a finding or a coverage", async () => {
  const material = await shipped((m, b) => {
    m.pbrMetallicRoughness.baseColorFactor = [1, 1, 1, 0.5];
    m.alphaCutoff = 0.9;
    grille(m, b);
  });
  expect(material.coverage).toEqual({ kind: "opaque" });
});

const UNSUPPORTED: [string, FindingCode, (m: GltfJson, b: GltfBuilder) => void][] = [
  ["an alpha mode glTF does not have", "material.coverage", (m) => (m.alphaMode = "DITHER")],
  [
    "a cutoff above 1",
    "material.coverage",
    (m, b) => {
      m.alphaMode = "MASK";
      m.alphaCutoff = 1.5;
      grille(m, b);
    },
  ],
  [
    "a cutoff below 0",
    "material.coverage",
    (m, b) => {
      m.alphaMode = "MASK";
      m.alphaCutoff = -0.1;
      grille(m, b);
    },
  ],
  [
    "a cutoff that is not a number",
    "material.coverage",
    (m, b) => {
      m.alphaMode = "MASK";
      m.alphaCutoff = "half";
      grille(m, b);
    },
  ],
  ["a cutout with no texture to cut it", "material.coverage_source", (m) => (m.alphaMode = "MASK")],
  [
    "a cutout whose normal map is whole everywhere",
    "material.coverage_source",
    (m, b) => {
      m.alphaMode = "MASK";
      m.normalTexture = { index: b.texture(4, () => [128, 128, 255, 255]) };
    },
  ],
  [
    "a cutout whose coverage is all in the albedo's alpha, where wear lives",
    "material.coverage_source",
    (m, b) => {
      m.alphaMode = "MASK";
      m.pbrMetallicRoughness.baseColorTexture = {
        index: b.texture(4, (x) => [200, 200, 200, x < 2 ? 0 : 255]),
      };
    },
  ],
  [
    "a cutout that cuts everything",
    "material.coverage_source",
    (m, b) => {
      m.alphaMode = "MASK";
      m.pbrMetallicRoughness.baseColorFactor = [1, 1, 1, 0.25];
      grille(m, b);
    },
  ],
  ["a blended surface that is whole", "material.coverage_source", (m) => (m.alphaMode = "BLEND")],
  [
    "a blended surface that wears",
    "material.wear",
    (m) => {
      m.alphaMode = "BLEND";
      m.pbrMetallicRoughness.baseColorFactor = [1, 1, 1, 0.25];
      m.extras = { wear: WEAR };
    },
  ],
  [
    "a sheet the atlas does not have",
    "material.interior",
    (m) => (m.extras = { interior: "attic" }),
  ],
  [
    "a room that is not opaque",
    "material.interior",
    (m) => {
      m.alphaMode = "BLEND";
      m.pbrMetallicRoughness.baseColorFactor = [1, 1, 1, 0.25];
      m.extras = { interior: "rooms" };
    },
  ],
  [
    "a room with textures of its own",
    "material.interior",
    (m, b) => {
      m.extras = { interior: "rooms" };
      b.textureMaterial(4);
    },
  ],
  ["a room that wears", "material.interior", (m) => (m.extras = { interior: "shops", wear: WEAR })],
];

for (const [what, code, edit] of UNSUPPORTED)
  test(`refused, and not baked: ${what} (${code})`, async () => {
    const built = await panel(edit);
    expect(built.findings.map((f) => f.code)).toEqual([code]);
    expect(built.findings[0].severity).toBe("error");
    expect(built.bundle).toBeNull();
  });

test("a room on a body that moves is refused: a vehicle's and a soldier's", async () => {
  const room = (bytes: Uint8Array) =>
    withJson(bytes, (j) => (j.materials[0].extras = { interior: "rooms" }));
  const tank = await validateAppearance(
    {
      name: "tank",
      entry: { unit: "vehicle", source: "tank.glb", basis_yaw_deg: 0, mounts: TANK_DRAWS },
      files: { "tank.glb": room(tankGlb()) },
    },
    context,
  );
  expect(tank.findings.map((f) => f.code)).toEqual(["material.interior"]);
  const { scene } = importScene(soldierGlb(), "rig.glb", 90);
  const clips = buildClips(scene!, "rig.glb", "test-rig", 30, SKELETON_ENTRY.clips).built!;
  const soldier = await validateAppearance(
    {
      name: "soldier",
      entry: { unit: "soldier", source: "s.glb", basis_yaw_deg: 90, skeleton: "test-rig" },
      files: { "s.glb": room(soldierGlb({ animated: false })) },
      skeleton: { clips, aim_reference: SKELETON_ENTRY.aim_reference },
    },
    context,
  );
  expect(soldier.findings.map((f) => f.code)).toEqual(["material.interior"]);
});

/** A baked catalog as runtime files, with the tank's bundle replaced by `edit`'s. */
async function runtimeWith(edit: (bytes: Uint8Array) => Promise<Uint8Array> | Uint8Array) {
  const sources = testSources();
  const result = await bakeCatalog(testCatalog(), async (p) => sources[p], {
    authority: AUTHORITY,
  });
  const files = new Map(result.files);
  const was = result.runtime.appearances.tank.bundle;
  const bytes = await edit(files.get(bundlePath(was))!.slice());
  const hash = await sha256Hex(bytes);
  files.delete(bundlePath(was));
  files.set(bundlePath(hash), bytes);
  result.runtime.appearances.tank.bundle = hash;
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  return files;
}

test("a bundle in the format before coverage is refused, not read", async () => {
  // Format 3 is the last whose materials carried no coverage.
  const files = await runtimeWith((bytes) => {
    new DataView(bytes.buffer).setUint32(4, 3, true);
    return bytes;
  });
  const library = new AppearanceLibrary(memoryFetch(files, "/a/"));
  await expect(library.load("/a/")).rejects.toThrow(/bundle format 3, .*re-bake/);
  expect(library.installed).toBeNull();
});

test("a bundle whose material says nothing of its coverage is refused, not taken for opaque", async () => {
  const files = await runtimeWith((bytes) => {
    const bundle = decodeBundle(bytes) as ArticulatedBundle;
    delete (bundle.materials[0] as Partial<Material>).coverage;
    return encodeBundle(bundle);
  });
  const library = new AppearanceLibrary(memoryFetch(files, "/a/"));
  await expect(library.load("/a/")).rejects.toThrow(/material paint: no coverage/);
  expect(library.installed).toBeNull();
});
