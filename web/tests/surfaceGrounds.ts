/** Test-only grounds for the surface field: one that holds every shape the
 *  export has, and one dense enough that an index matters. The unit test
 *  (`surfaceField.test.ts`) and the GPU check (`scenes/_surfaceField.mjs`)
 *  read the same two. */
import type { Vec2 } from "math";
import { mulberry32 } from "math/random";
import {
  bareSource,
  type GroundSource,
} from "@packages/battle-renderer/src/terrain/terrainSurface.ts";

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

/** The forests' export streams: each forest's primitives carry its id. */
interface Forests {
  metadata: number[];
  strokes: number[];
  triangles: number[];
  boundaries: number[];
}

/** A regular `sides`-gon forest: a triangle fan and its ring. */
function polygonForest(into: Forests, cx: number, cy: number, radius: number, sides: number) {
  const id = into.metadata.length / 3;
  const ring: Vec2[] = [];
  for (let k = 0; k < sides; k++) {
    const turn = (k / sides) * Math.PI * 2;
    ring.push([cx + Math.cos(turn) * radius, cy + Math.sin(turn) * radius]);
  }
  for (let k = 0; k < sides; k++) {
    const a = ring[k],
      b = ring[(k + 1) % sides];
    into.boundaries.push(a[0], a[1], b[0], b[1], id);
    if (k > 0 && k < sides - 1)
      into.triangles.push(ring[0][0], ring[0][1], a[0], a[1], b[0], b[1], id);
  }
  into.metadata.push(id, 12, 2);
}

/** A town-sized ground: curved roads cut to 2 m segments, polygon and strip
 *  woods, and a winding river whose width swells and narrows. */
export function denseGround(sizeM: number, roads: number, woods: number): GroundSource {
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
      // As the simulation exports a stroke: the stretches within half its
      // width of either end, along it, are cut square on that side. (A
      // stretch pinned against the map's edge has no length, which no export
      // has, and so no end to cut.)
      const cut =
        nx === x && ny === y ? 0 : (s * 2 < half ? 1 : 0) | ((299 - s) * 2 < half ? 2 : 0);
      strokes.push(x, y, nx, ny, half, 1, cut);
      x = nx;
      y = ny;
    }
  }
  const forests: Forests = { metadata: [], strokes: [], triangles: [], boundaries: [] };
  for (let w = 0; w < woods; w++)
    polygonForest(
      forests,
      40 + next() * (sizeM - 80),
      40 + next() * (sizeM - 80),
      15 + next() * 25,
      24,
    );
  for (let s = 0; s < 60; s++)
    forests.strokes.push(
      100 + s * 4,
      900 + Math.sin(s / 6) * 30,
      104 + s * 4,
      900 + Math.sin((s + 1) / 6) * 30,
      9,
      woods,
      (s * 4 < 9 ? 1 : 0) | ((59 - s) * 4 < 9 ? 2 : 0),
    );
  forests.metadata.push(woods, 12, 1);
  const rivers: number[] = [];
  const bank = (s: number) => [s * 20, 500 + Math.sin(s / 10) * 60, 9 + Math.sin(s / 7) * 3];
  for (let s = 0; s < 80; s++) {
    const [ax, ay, halfA] = bank(s),
      [bx, by, halfB] = bank(s + 1);
    rivers.push(ax, ay, bx, by, halfA, halfB, 0.25, 0.25, 1, 1, -1);
  }
  return {
    ...bareSource([0, 0, sizeM, sizeM], 4),
    surfaceStrokes: Float32Array.from(strokes),
    // Two rectangle woods besides the polygons and the strip.
    forests: Float32Array.of(1200, 1200, 300, 240, 0, 1350, 1300, 200, 200, 0),
    forestRectIds: Uint32Array.of(woods + 1, woods + 2),
    forestMetadata: Float32Array.from([...forests.metadata, woods + 1, 12, 0, woods + 2, 12, 0]),
    forestStrokes: Float32Array.from(forests.strokes),
    forestTriangles: Float32Array.from(forests.triangles),
    forestBoundaries: Float32Array.from(forests.boundaries),
    forestTrunkRanges: new Uint32Array((woods + 3) * 2),
    rivers: Float32Array.from(rivers),
  };
}
