// @vitest-environment node
// Material transport: a source material says how much of its surface is there
// (opaque, cutout, blended) and whether it is a room behind a window, and the
// bundle carries exactly that. Wear and the tint mask keep their own channels.
import { lerp } from "math";
import { expect, test } from "vitest";
import type { GltfJson } from "@packages/scene-assets/src/glb.ts";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake.ts";
import { buildClips } from "@packages/scene-assets/src/build.ts";
import { joinTextures, splitTextures } from "@packages/scene-assets/src/codec.ts";
import { decodeTexture, encodeTexture } from "@packages/scene-assets/src/texture.ts";
import { publishGzip } from "@packages/scene-assets/src/gzip.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { importScene } from "@packages/scene-assets/src/scene.ts";
import { validateAppearance } from "@packages/scene-assets/src/validate.ts";
import {
  bundlePath,
  type ArticulatedBundle,
  type Bundle,
  type FindingCode,
  type Material,
  type StaticBundle,
} from "@packages/scene-assets/src/schema.ts";
import {
  AUTHORITY,
  bakedBundle,
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

/** `bundle` as a page receives it: its content and each texture travel as
 *  their own files and are joined again. */
function travelled(bundle: Bundle): Bundle {
  const { content, textures } = splitTextures(bundle);
  return joinTextures(
    content,
    textures.map((t) => decodeTexture(t.id, encodeTexture(t))),
  );
}

/** The panel's material after the whole trip: built and travelled. */
async function shipped(edit?: (material: GltfJson, b: GltfBuilder) => void): Promise<Material> {
  const built = await panel(edit);
  expect(built.findings).toEqual([]);
  const bundle = travelled(built.bundle!) as StaticBundle;
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
  const bundle = travelled(built.bundle!) as StaticBundle;
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

// Material roles. Black rubber and sight glass are held dark by what the
// surface draws on average, as the model shader does: its albedo texture's
// mean times the vertex colour (`colour_scale` times the base colour), and the
// wear colour where the vertex's wear passes the texture's threshold.

/** A linear value as a recipe's PNG stores it: an sRGB byte. */
const srgbByte = (x: number) =>
  Math.round(255 * (x <= 0.0031308 ? 12.92 * x : 1.055 * x ** (1 / 2.4) - 0.055));
/** The rubber recipe's mean (`textures.py` `rubber`) and the exporters' summer
 *  dust (`parts.DUST`), linear. */
const RUBBER = 0.024;
const DUST = [0.15, 0.13, 0.1] as const;

/** A tyre as the exporters write one (`parts.textured`): the rubber recipe's
 *  albedo, factors 1, and a vertex colour at a third of its film's colour
 *  relative to the recipe's mean (`textures.macro`). `film` is how far the
 *  dust film carries it toward `DUST`. */
const tyre =
  (film: number, wear = 0) =>
  (m: GltfJson, b: GltfBuilder) => {
    m.extras = { role: "rubber", wear: [0.12, 0.105, 0.085, 1], colour_scale: 3 };
    m.pbrMetallicRoughness = {
      baseColorFactor: [1, 1, 1, 1],
      metallicFactor: 1,
      roughnessFactor: 1,
    };
    const byte = srgbByte(RUBBER);
    m.pbrMetallicRoughness.baseColorTexture = {
      index: b.texture(4, () => [byte, byte, byte, 128]),
    };
    const [r, g, bl] = DUST.map((d) => lerp(RUBBER, d, film) / RUBBER / 3);
    b.colour = [r, g, bl, wear];
  };

/** Glass as the exporters wrote optics (`parts.flat_paint`): a base colour of
 *  a half and the colour, over it, in the vertex colour. */
const optic =
  (colour: [number, number, number], roughness: number) => (m: GltfJson, b: GltfBuilder) => {
    m.extras = { role: "glass" };
    m.pbrMetallicRoughness = {
      baseColorFactor: [0.5, 0.5, 0.5, 1],
      metallicFactor: 0,
      roughnessFactor: roughness,
    };
    b.colour = [colour[0] / 0.5, colour[1] / 0.5, colour[2] / 0.5, 1];
  };

test("a tyre in black rubber, and dark smooth glass, ship their roles", async () => {
  const rubber = await shipped(tyre(0));
  expect(rubber.role).toBe("rubber");
  // Dust low on the tread only: a little of the film, the tyre still black.
  expect((await shipped(tyre(0.05))).role).toBe("rubber");
  expect((await shipped(optic([0.01, 0.014, 0.015], 0.08))).role).toBe("glass");
});

test("a material that names no role ships none", async () => {
  expect((await shipped()).role).toBeUndefined();
});

const ROLE_REFUSED: [string, FindingCode, (m: GltfJson, b: GltfBuilder) => void][] = [
  ["a role the contract does not have", "material.role", (m) => (m.extras = { role: "chrome" })],
  // The roster's tyre recipe, filmed all over with `DUST` as far as the
  // exporters' `textured('rubber', 'rubber', dirt=.3)` films its lowest rim
  // (dirt × 0.7, times the film's break-up at its most).
  ["a tyre filmed grey with dust all over", "material.role_rubber", tyre(0.3 * 0.7 * 0.8)],
  // Black rubber, but worn everywhere past its threshold (a half): the light
  // wear colour (dried mud) is what draws.
  ["a black tyre worn through to its light wear colour", "material.role_rubber", tyre(0, 1)],
  // Today's roster optics: `flat_paint('optics', (.018, .06, .07), rough=.15)`.
  ["cyan optics, as the roster's were", "material.role_glass", optic([0.018, 0.06, 0.07], 0.15)],
  ["dark glass that is matte", "material.role_glass", optic([0.01, 0.014, 0.015], 0.6)],
];

for (const [what, code, edit] of ROLE_REFUSED)
  test(`refused, and not baked: ${what} (${code})`, async () => {
    const built = await panel(edit);
    expect(built.findings.map((f) => f.code)).toEqual([code]);
    expect(built.bundle).toBeNull();
  });

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

/** A baked catalog as runtime files, with the tank's bundle travelling as
 *  `edit` encodes it. */
async function runtimeWith(edit: (bundle: Bundle) => Uint8Array) {
  const sources = testSources();
  const result = await bakeCatalog(testCatalog(), async (p) => sources[p], {
    authority: AUTHORITY,
  });
  const out = { runtime: result.runtime, files: new Map(result.files) };
  const hash = result.runtime.appearances.tank.bundle;
  await publishGzip(out, hash, edit(await bakedBundle(result, hash)), bundlePath);
  out.files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  return out.files;
}

test("a bundle in the format before coverage is refused, not read", async () => {
  // Format 3 is the last whose materials carried no coverage.
  const files = await runtimeWith((bundle) => {
    const { content } = splitTextures(bundle);
    new DataView(content.buffer).setUint32(4, 3, true);
    return content;
  });
  const library = new AppearanceLibrary(memoryFetch(files, "/a/"));
  await expect(library.load("/a/")).rejects.toThrow(/bundle format 3, .*re-bake/);
  expect(library.installed).toBeNull();
});

test("a bundle whose material says nothing of its coverage is refused, not taken for opaque", async () => {
  const files = await runtimeWith((bundle) => {
    delete ((bundle as ArticulatedBundle).materials[0] as Partial<Material>).coverage;
    return splitTextures(bundle).content;
  });
  const library = new AppearanceLibrary(memoryFetch(files, "/a/"));
  await expect(library.load("/a/")).rejects.toThrow(/material paint: no coverage/);
  expect(library.installed).toBeNull();
});
