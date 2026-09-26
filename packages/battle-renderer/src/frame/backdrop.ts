// The land past the map edge: a flat frame of ground around the map box out to
// `presentation.light.backdrop.reach_m`, lit by the same sun and sky and hazed
// by the same aerial term as the map, so the map's edge fades into distance
// instead of ending at a khaki band or a hard eye-level horizon. It is
// scenery, never gameplay: it takes no fog, casts nothing and is never picked.
import { MeshBuilder, type Mesh } from "../mesh";
import type { LightPresentation } from "../light/sceneLight";
import type { MapBox } from "./receiverRange";

export function backdropMesh(box: MapBox, backdrop: LightPresentation["backdrop"]): Mesh {
  const [lo, hi] = box;
  // mapBox pads a metre below the lowest ground; the backdrop sits on it.
  const z = lo[2] + 1;
  const r = backdrop.reach_m;
  const color = [...backdrop.albedo, 1] as const;
  const mesh = new MeshBuilder();
  const band = (x0: number, y0: number, x1: number, y1: number) =>
    mesh.quad([x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z], color);
  band(lo[0] - r, lo[1] - r, hi[0] + r, lo[1]); // south, full width
  band(lo[0] - r, hi[1], hi[0] + r, hi[1] + r); // north, full width
  band(lo[0] - r, lo[1], lo[0], hi[1]); // west
  band(hi[0], lo[1], hi[0] + r, hi[1]); // east
  return mesh.build();
}
