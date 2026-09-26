// The land past the map edge: a flat frame of ground around the map box out to
// `presentation.light.backdrop.reach_m`, lit by the same sun and sky and hazed
// by the same aerial term as the map, so the map's edge fades into distance
// instead of ending at a khaki band or a hard eye-level horizon. It is
// scenery, never gameplay: it takes no fog, casts nothing and is never picked.
import { MeshBuilder, type Mesh } from "../mesh";
import type { LightPresentation } from "../light/sceneLight";
import type { Box3 } from "math/shapes";

export function backdropMesh(box: Box3, backdrop: LightPresentation["backdrop"]): Mesh {
  const [minX, minY, minZ, maxX, maxY] = box;
  // mapBox pads a metre below the lowest ground; the backdrop sits on it.
  const z = minZ + 1;
  const r = backdrop.reach_m;
  const color = [...backdrop.albedo, 1] as const;
  const mesh = new MeshBuilder();
  const band = (x0: number, y0: number, x1: number, y1: number) =>
    mesh.quad([x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z], color);
  band(minX - r, minY - r, maxX + r, minY); // south, full width
  band(minX - r, maxY, maxX + r, maxY + r); // north, full width
  band(minX - r, minY, minX, maxY); // west
  band(maxX, minY, maxX + r, maxY); // east
  return mesh.build();
}
