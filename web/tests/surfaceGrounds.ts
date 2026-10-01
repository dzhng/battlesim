/** Test-only grounds for the surface field: one that holds every shape the
 *  export has, and one dense enough that an index matters. The unit test
 *  (`surfaceField.test.ts`) and the GPU check (`scenes/_surfaceField.mjs`)
 *  read the same two. */
import type { Vec2 } from "math";
import { mulberry32 } from "math/random";
import type { ForestShape } from "@packages/battle-renderer/src/terrain/forestShapes";
import type { TerrainSite } from "@packages/battle-renderer/src/terrain/terrainSurface.ts";

const road = (points: number[][], width_m: number, kind = "road") => ({
  kind,
  shape: { kind: "stroke", points, width_m },
});
const paving = (ring: number[][], kind = "road") => ({ kind, shape: { kind: "polygon", ring } });
const wood = (shape: unknown) => ({ shape });

/** Joins, a width change, overlapping and touching paving, a square larger
 *  than any reach, and every forest and water shape the export has. */
export const CURATED_GROUND = {
  size: [512, 384],
  fog_cell_m: 8,
  height_grid_m: 4,
  slope_cutoff_deg: 35,
  surfaces: [
    road(
      [
        [20, 40],
        [160, 40],
        [220, 110],
        [220, 200],
      ],
      8,
    ),
    road(
      [
        [160, 40],
        [300, 20],
      ],
      14,
    ),
    road(
      [
        [220, 110],
        [221, 111],
        [223, 112],
        [300, 150],
      ],
      5,
    ),
    paving([
      [20, 220],
      [60, 220],
      [60, 260],
      [20, 260],
    ]),
    paving([
      [40, 240],
      [80, 240],
      [80, 280],
      [40, 280],
    ]),
    paving([
      [80, 240],
      [120, 250],
      [110, 300],
      [80, 280],
    ]),
    paving([
      [300, 200],
      [480, 200],
      [480, 360],
      [300, 360],
    ]),
    paving([
      [214, 190],
      [226, 190],
      [226, 215],
      [214, 215],
    ]),
  ],
  forests: [
    wood({
      kind: "polygon",
      ring: [
        [20, 300],
        [110, 300],
        [110, 327],
        [47, 327],
        [47, 380],
        [20, 380],
      ],
    }),
    wood({
      kind: "polygon",
      ring: [
        [60, 310],
        [150, 320],
        [140, 370],
        [70, 360],
      ],
    }),
    wood({
      kind: "polygon",
      ring: [
        [330, 20],
        [500, 20],
        [500, 180],
        [330, 180],
      ],
    }),
    wood({
      kind: "stroke",
      points: [
        [230, 240],
        [270, 300],
        [260, 370],
      ],
      width_m: 18,
    }),
    wood({
      kind: "polygon",
      ring: [
        [400, 150],
        [470, 160],
        [440, 230],
      ],
    }),
  ],
  // A river that widens round a bend, crossing roads and a wood, and a
  // short straight one that runs into it.
  rivers: [
    {
      points: [
        { xy: [142, 60], width_m: 12, depth_m: 1.5 },
        { xy: [142, 200], width_m: 24, depth_m: 2 },
        { xy: [300, 260], width_m: 16, depth_m: 2 },
      ],
      surface_z: -0.5,
    },
    {
      points: [
        { xy: [60, 215], width_m: 14, depth_m: 1.5 },
        { xy: [142, 215], width_m: 14, depth_m: 1.5 },
      ],
      surface_z: -0.5,
    },
  ],
};

/** A regular `sides`-gon forest: a triangle fan and its ring. */
function polygonForest(cx: number, cy: number, radius: number, sides: number): ForestShape {
  const ring: Vec2[] = [];
  for (let k = 0; k < sides; k++) {
    const turn = (k / sides) * Math.PI * 2;
    ring.push([cx + Math.cos(turn) * radius, cy + Math.sin(turn) * radius]);
  }
  const triangles: number[] = [],
    boundaries: number[] = [];
  for (let k = 0; k < sides; k++) {
    const a = ring[k],
      b = ring[(k + 1) % sides];
    boundaries.push(a[0], a[1], b[0], b[1], 0);
    if (k > 0 && k < sides - 1) triangles.push(ring[0][0], ring[0][1], a[0], a[1], b[0], b[1], 0);
  }
  return {
    canopy: 12,
    trunkRange: [0, 0],
    kind: "polygon",
    strokes: new Float32Array(0),
    triangles: Float32Array.from(triangles),
    boundaries: Float32Array.from(boundaries),
  };
}

/** A town-sized ground: curved roads cut to 2 m segments, polygon and strip
 *  woods, and a winding river whose width swells and narrows. */
export function denseGround(sizeM: number, roads: number, woods: number): TerrainSite {
  const random = mulberry32.create(7);
  const next = () => mulberry32.sample(random);
  const strokes: number[] = [];
  for (let r = 0; r < roads; r++) {
    let x = next() * sizeM,
      y = next() * sizeM,
      heading = next() * Math.PI * 2;
    const half = 3 + Math.floor(next() * 3) * 1.5;
    for (let s = 0; s < 300; s++) {
      heading += (next() - 0.5) * 0.12;
      const nx = Math.min(sizeM, Math.max(0, x + Math.cos(heading) * 2)),
        ny = Math.min(sizeM, Math.max(0, y + Math.sin(heading) * 2));
      // The road's first and last stretch end square. (A stretch pinned
      // against the map's edge has no length, which no export has, and so
      // no end to cut.)
      const cut = nx === x && ny === y ? 0 : s === 0 ? 1 : s === 299 ? 2 : 0;
      strokes.push(x, y, nx, ny, half, 1, cut);
      x = nx;
      y = ny;
    }
  }
  const forestShapes: ForestShape[] = [];
  for (let w = 0; w < woods; w++) {
    const shape = polygonForest(
      40 + next() * (sizeM - 80),
      40 + next() * (sizeM - 80),
      15 + next() * 25,
      24,
    );
    forestShapes.push(shape);
  }
  const strip: number[] = [];
  for (let s = 0; s < 60; s++)
    strip.push(
      100 + s * 4,
      900 + Math.sin(s / 6) * 30,
      104 + s * 4,
      900 + Math.sin((s + 1) / 6) * 30,
      9,
      0,
      s === 0 ? 1 : s === 59 ? 2 : 0,
    );
  forestShapes.push({
    canopy: 12,
    trunkRange: [0, 0],
    kind: "stroke",
    strokes: Float32Array.from(strip),
    triangles: new Float32Array(0),
    boundaries: new Float32Array(0),
  });
  const rivers: number[] = [];
  const bank = (s: number) => [s * 20, 500 + Math.sin(s / 10) * 60, 9 + Math.sin(s / 7) * 3];
  for (let s = 0; s < 80; s++) {
    const [ax, ay, halfA] = bank(s),
      [bx, by, halfB] = bank(s + 1);
    rivers.push(ax, ay, bx, by, halfA, halfB, 0.25, 0.25, 1, 1, -1);
  }
  return {
    map: [0, 0, sizeM, sizeM],
    gridM: 4,
    surfaceStrokes: Float32Array.from(strokes),
    surfaceStrokeStride: 7,
    // No authored runs: the field plots are not what these grounds test.
    surfaceRuns: new Float32Array(0),
    surfaceRunStride: 4,
    surfaceTriangles: new Float32Array(0),
    surfaceTriangleStride: 7,
    surfaceBoundaries: new Float32Array(0),
    surfaceBoundaryStride: 5,
    forests: Float32Array.of(1200, 1200, 300, 240, 1350, 1300, 200, 200),
    forestShapes,
    rivers: Float32Array.from(rivers),
    riverRuns: new Float32Array(0),
    riverRunStride: 4,
    buildings: [],
    footprints: new Float32Array(0),
  };
}
