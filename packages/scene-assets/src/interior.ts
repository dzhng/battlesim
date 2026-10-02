// The interior atlas's way into a bundle. A room material names its sheet and
// has no texture of its own (`material.ts`); the bake gives the bundle that
// sheet as the material's albedo texture, so it reaches the renderer as every
// texture does: square, fully mipped, addressed by its content, one layer on
// the GPU however many kits show it.
//
// A sheet's source is the picture `blender/city/interiors.py` writes, two
// columns of cells. A texture is square, so the cells are laid out again, in
// the same order, `INTERIOR_ATLAS.columns` to a row.

import {
  CHANNEL_FORMAT,
  INTERIOR_ATLAS,
  type Bundle,
  type Finding,
  type InteriorSheet,
  type SkeletonClips,
  type Texture,
} from "./schema.ts";
import { bakeTexture, decodePng } from "./texture.ts";

/** A sheet's texture from its source PNG. Throws when the picture is not the
 *  atlas's cells. */
export async function interiorTexture(png: Uint8Array): Promise<Texture> {
  const { cells, cell_px, source_columns, columns } = INTERIOR_ATLAS;
  const source = await decodePng(png);
  const [width, height] = [source_columns * cell_px, (cells / source_columns) * cell_px];
  if (source.width !== width || source.height !== height)
    throw new Error(
      `is ${source.width}×${source.height} px, not the atlas's ${width}×${height} (${source_columns} columns of ${cell_px} px cells)`,
    );
  const size = columns * cell_px;
  const pixels = new Uint8Array(size * size * 4);
  for (let i = 3; i < pixels.length; i += 4) pixels[i] = 255;
  for (let cell = 0; cell < cells; cell++) {
    const from = [(cell % source_columns) * cell_px, Math.floor(cell / source_columns) * cell_px];
    const to = [(cell % columns) * cell_px, Math.floor(cell / columns) * cell_px];
    for (let y = 0; y < cell_px; y++) {
      const row = ((from[1] + y) * width + from[0]) * 4;
      pixels.set(source.pixels.subarray(row, row + cell_px * 4), ((to[1] + y) * size + to[0]) * 4);
    }
  }
  return bakeTexture({ width: size, height: size, pixels }, "albedo", CHANNEL_FORMAT.albedo);
}

/** How the bake answers for a sheet: its texture, or why it has none. */
export type InteriorSheets = (sheet: InteriorSheet) => Promise<Texture | string>;

/**
 * Give every room material of `bundle` its sheet as its albedo texture. A
 * sheet the bake cannot answer for is a `material.interior` finding, and the
 * material is left without one.
 */
export async function bindInteriors(
  label: string,
  bundle: Exclude<Bundle, SkeletonClips>,
  sheets: InteriorSheets,
): Promise<Finding[]> {
  const findings: Finding[] = [];
  for (const material of bundle.materials) {
    if (material.interior === undefined) continue;
    const texture = await sheets(material.interior);
    if (typeof texture === "string") {
      findings.push({
        code: "material.interior",
        severity: "error",
        message: `${label}: material "${material.name}" shows the interior sheet "${material.interior}", which ${texture}`,
        fix: "name the sheet's PNG under `interiors` in assets/catalog.json, as blender/city/interiors.py writes it",
      });
      continue;
    }
    let at = bundle.textures.findIndex((t) => t.id === texture.id);
    if (at < 0) at = bundle.textures.push(texture) - 1;
    material.textures = { albedo: at };
  }
  return findings;
}
