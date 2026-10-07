// @vitest-environment node
// The interior atlas's way into the runtime: a room material names its sheet,
// and the bake gives the bundle that sheet as a texture like any other, every
// cell of the atlas whole at every mip a lookup reads.
import { expect, test } from "vitest";
import { bakeCatalog } from "@packages/scene-assets/src/bake.ts";
import {
  INTERIOR_ATLAS,
  type Catalog,
  type StaticBundle,
} from "@packages/scene-assets/src/schema.ts";
import { AUTHORITY, TOLERANCES, bakedBundle, encodePng, panelGlb } from "./synthetic";

const { cells, cell_px, source_columns, columns } = INTERIOR_ATLAS;
/** Cell `i`'s colour in the test sheets: every cell its own. */
const colourOf = (i: number, sheet: number) => [20 + i * 20, 200 - i * 15, 40 + sheet * 100];

/** A sheet as `interiors.py` writes it: 2 columns of 128 px cells, each a flat colour here. */
function sheetPng(sheet: number, width = source_columns * cell_px): Uint8Array {
  const height = (cells / source_columns) * cell_px;
  const rgba = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++) {
      const cell = Math.floor(y / cell_px) * source_columns + Math.floor(x / cell_px);
      rgba.set([...colourOf(cell, sheet), 255], (y * width + x) * 4);
    }
  return encodePng(width, height, rgba);
}

const room = (sheet: string) => panelGlb((m) => (m.extras = { interior: sheet }));
function catalog(interiors: Catalog["interiors"]): Catalog {
  const panel = (source: string) =>
    ({
      unit: "scenery",
      scenery: "hedgerow",
      states: { summer: source },
      basis_yaw_deg: 0,
    }) as const;
  return {
    tolerances: TOLERANCES,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    skeletons: {},
    appearances: { flat: panel("flat.glb"), shop: panel("shop.glb"), house: panel("flat.glb") },
    ...(interiors ? { interiors } : {}),
  };
}
const SOURCES: Record<string, () => Uint8Array> = {
  "flat.glb": () => room("rooms"),
  "shop.glb": () => room("shops"),
  "rooms.png": () => sheetPng(0),
  "shops.png": () => sheetPng(1),
  "narrow.png": () => sheetPng(0, cell_px),
};
const bake = (interiors: Catalog["interiors"]) =>
  bakeCatalog(catalog(interiors), async (path) => SOURCES[path](), { authority: AUTHORITY });

async function baked(interiors: Catalog["interiors"]) {
  const result = await bake(interiors);
  const bundle = async (name: string) =>
    (await bakedBundle(result, result.runtime.appearances[name].bundle)) as StaticBundle;
  return { result, bundle };
}

test("a room's bundle carries its sheet as a texture: each cell whole, at every mip down to a texel a cell", async () => {
  const { result, bundle } = await baked({ rooms: "rooms.png", shops: "shops.png" });
  const appearances = result.reports.filter((r) => r.what === "appearance");
  expect(appearances.flatMap((r) => r.findings)).toEqual([]);
  for (const [name, sheet] of [
    ["flat", 0],
    ["shop", 1],
  ] as const) {
    const { materials, textures } = await bundle(name);
    const texture = textures[materials[0].textures!.albedo!];
    expect(texture.format).toBe("rgba8unorm-srgb");
    expect(texture.width).toBe(columns * cell_px);
    // Down to the level where a cell is one texel, a cell's texels are its own
    // room's and nothing else's: a flat cell stays its colour to the edge.
    for (let level = 0; cell_px >> level >= 1; level++) {
      const size = texture.width >> level;
      const edge = cell_px >> level;
      for (let cell = 0; cell < cells; cell++) {
        const [x0, y0] = [(cell % columns) * edge, Math.floor(cell / columns) * edge];
        for (const [x, y] of [
          [x0, y0],
          [x0 + edge - 1, y0],
          [x0, y0 + edge - 1],
          [x0 + edge - 1, y0 + edge - 1],
        ]) {
          const at = (y * size + x) * 4;
          expect(
            [...texture.levels[level].subarray(at, at + 3)],
            `sheet ${sheet} cell ${cell} level ${level}`,
          ).toEqual(colourOf(cell, sheet));
        }
      }
    }
  }
});

test("bundles that show one sheet share one texture", async () => {
  const { bundle } = await baked({ rooms: "rooms.png", shops: "shops.png" });
  const id = async (name: string) => {
    const b = await bundle(name);
    return b.textures[b.materials[0].textures!.albedo!].id;
  };
  expect(await id("house")).toBe(await id("flat"));
  expect(await id("shop")).not.toBe(await id("flat"));
});

test("a room whose sheet the catalog has no source for is refused, by name", async () => {
  const result = await bake({ rooms: "rooms.png" });
  const shop = result.reports.find((r) => r.name === "shop")!;
  expect(shop.findings.map((f) => f.code)).toEqual(["material.interior"]);
  expect(shop.findings[0].message).toContain("shops");
  expect(result.runtime.appearances.shop).toBeUndefined();
  // The rooms that have their sheet still bake.
  expect(result.runtime.appearances.flat).toBeDefined();
});

test("a sheet that is not the atlas's ten cells is refused", async () => {
  const result = await bake({ rooms: "narrow.png", shops: "shops.png" });
  const flat = result.reports.find((r) => r.name === "flat")!;
  expect(flat.findings.map((f) => f.code)).toEqual(["material.interior"]);
  expect(flat.findings[0].message).toContain("128×640");
  expect(result.runtime.appearances.flat).toBeUndefined();
});
