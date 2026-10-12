// The water surface: flat cells at each river's surface height along its
// centreline, one per grid square the water (and a margin) touches
// (`Ground.water_cells`: four floats each, the south-west corner, the surface
// height and the side, in metres). Cells never overlap, so the blended
// surface is drawn once however the river bends; the terrain material cuts
// its edge on the simulation's own water distance.
import { MeshBuilder, type Mesh, type Rgba } from "../mesh";

const WHITE: Rgba = [1, 1, 1, 1];

export function waterSurfaceMesh(cells: Float32Array): Mesh {
  const mesh = new MeshBuilder();
  for (let o = 0; o < cells.length; o += 4) {
    const [x, y, z, cell] = [cells[o], cells[o + 1], cells[o + 2], cells[o + 3]];
    mesh.quad([x, y, z], [x + cell, y, z], [x + cell, y + cell, z], [x, y + cell, z], WHITE);
  }
  return mesh.build();
}
