// The land past the map edge: a flat frame of ground around the map box out to
// `presentation.light.backdrop.reach_m`, drawn with the biome's ground
// material (the patchwork runs on past the map, then gives way to the
// biome's distant colour), lit by the same sun and sky and hazed by the same
// aerial term as the map, so the map's edge fades into distance instead of
// ending at a plate. It is scenery, never gameplay: it takes no fog, casts
// nothing and is never picked.
import { MeshBuilder, type Mesh, type Rgba } from "../mesh";
import type { Box3 } from "math/shapes";

/** No tint: the biome material alone. */
const BIOME: Rgba = [0, 0, 0, 0];

export function backdropMesh(box: Box3, reach: number): Mesh {
  const [minX, minY, minZ, maxX, maxY] = box;
  // mapBox pads a metre below the lowest ground; the backdrop sits at that
  // ground's height, so the patchwork meets the map's edge level.
  const z = minZ + 1;
  const mesh = new MeshBuilder();
  const band = (x0: number, y0: number, x1: number, y1: number) =>
    mesh.quad([x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z], BIOME);
  band(minX - reach, minY - reach, maxX + reach, minY); // south, full width
  band(minX - reach, maxY, maxX + reach, maxY + reach); // north, full width
  band(minX - reach, minY, minX, maxY); // west
  band(maxX, minY, maxX + reach, maxY); // east
  return mesh.build();
}
