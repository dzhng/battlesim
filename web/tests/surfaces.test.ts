// @vitest-environment node
import { expect, test } from "vitest";
import { generatePlots, plotAt, type PlotSite } from "@packages/battle-renderer/src/terrain/plots";
import type { ForestShape } from "@packages/battle-renderer/src/terrain/forestShapes";
import {
  drawnStrokes,
  SURFACE_AREA_KINDS,
  type SurfaceAreaKind,
} from "@packages/battle-renderer/src/terrain/surfaces";
import {
  CUT_A,
  CUT_B,
  STROKE_CUTS,
  STROKE_FLOATS,
  strokeInside,
} from "@packages/battle-renderer/src/terrain/strokes";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome";
import summer from "@fixtures/biomes/summer.json";

// Fixed field rules prevent seeded agricultural cuts from obscuring the guide contract.
const base = validateBiome(summer as unknown as Biome);
const biome: Biome = {
  ...base,
  field_rules: {
    ...base.field_rules,
    extent_m: 0,
    size_m: [1000, 1000],
    tract_m: 1000,
    // A strip between two road edges lies along them, however long.
    max_aspect: 50,
  },
};
const empty: PlotSite = {
  map: [0, 0, 100, 100],
  buildings: [],
  surfaceStrokeStride: 7,
  surfaceRunStride: 4,
  surfaceRuns: new Float32Array(0),
  surfaceTriangleStride: 7,
  surfaceBoundaryStride: 5,
  surfaceStrokes: new Float32Array(0),
  surfaceTriangles: new Float32Array(0),
  surfaceBoundaries: new Float32Array(0),
  riverRuns: new Float32Array(0),
  riverRunStride: 4,
  forestShapes: [],
};

/** A strip of forest `width` metres wide along `stretches` (`ax, ay, bx, by` each). */
function strip(width: number, ...stretches: number[][]): ForestShape {
  return {
    canopy: 12,
    trunkRange: [0, 0],
    kind: "stroke",
    strokes: Float32Array.from(
      stretches.flatMap(([ax, ay, bx, by]) => [ax, ay, bx, by, width / 2, 0, 0]),
    ),
    triangles: new Float32Array(0),
    boundaries: new Float32Array(0),
  };
}

/** A strip of paving of one kind across the map, as a stroke and as a polygon. */
function paving(named: SurfaceAreaKind): PlotSite {
  const kind = SURFACE_AREA_KINDS.indexOf(named);
  return {
    ...empty,
    surfaceStrokes: Float32Array.of(0, 50, 100, 50, 2, kind, 3),
    surfaceRuns: named === "paving" ? new Float32Array(0) : Float32Array.of(0, 50, 100, 50),
    surfaceTriangles: Float32Array.of(0, 40, 100, 40, 100, 60, kind, 0, 40, 100, 60, 0, 60, kind),
    surfaceBoundaries: Float32Array.of(
      0,
      40,
      100,
      40,
      kind,
      100,
      40,
      100,
      60,
      kind,
      100,
      60,
      0,
      60,
      kind,
      0,
      60,
      0,
      40,
      kind,
    ),
  };
}

test("paving leaves fields unchanged while every road kind guides their boundaries", () => {
  const untouched = generatePlots(empty, biome);
  expect(generatePlots(paving("paving"), biome)).toEqual(untouched);
  for (const kind of ["road", "country_road", "dirt_track"] as const) {
    const road = generatePlots(paving(kind), biome);
    expect(plotAt(road, 50, 30)!.plot, kind).not.toBe(plotAt(road, 50, 70)!.plot);
  }
});

test("a river's authored run guides field boundaries as a road's does", () => {
  const river = generatePlots({ ...empty, riverRuns: Float32Array.of(0, 50, 100, 50) }, biome);
  expect(plotAt(river, 50, 30)!.plot).not.toBe(plotAt(river, 50, 70)!.plot);
  // The cut runs along the river: both banks' fields end on its centreline.
  expect(plotAt(river, 50, 44)!.edge).toBeCloseTo(6, 9);
  expect(plotAt(river, 20, 57)!.edge).toBeCloseTo(7, 9);
  expect(plotAt(generatePlots(empty, biome), 50, 30)!.plot).toBe(
    plotAt(generatePlots(empty, biome), 50, 70)!.plot,
  );
});

test("a tree line's long stretches guide field boundaries, and the short chords of its bends do not", () => {
  const line = generatePlots({ ...empty, forestShapes: [strip(10, [0, 50, 100, 50])] }, biome);
  expect(plotAt(line, 50, 30)!.plot).not.toBe(plotAt(line, 50, 70)!.plot);
  // The fields either side end on the strip's centreline, under its trees.
  expect(plotAt(line, 50, 44)!.edge).toBeCloseTo(6, 9);
  // A bend is exported as chords far shorter than the strip is wide: a field
  // is not cut along one.
  const chord = generatePlots({ ...empty, forestShapes: [strip(10, [49, 50, 51, 50])] }, biome);
  expect(plotAt(chord, 50, 30)!.plot).toBe(plotAt(chord, 50, 70)!.plot);
  // A wood is not a line: its outline cuts nothing.
  const wood: ForestShape = { ...strip(10), kind: "rectangle", rect: [0, 40, 100, 20] };
  expect(generatePlots({ ...empty, forestShapes: [wood] }, biome)).toEqual(
    generatePlots(empty, biome),
  );
});

test("road polygon guides use only the ring, never the triangulation diagonal", () => {
  const polygon = {
    ...paving("road"),
    surfaceStrokes: new Float32Array(0),
    surfaceRuns: new Float32Array(0),
  };
  const fields = generatePlots(polygon, biome);
  expect(plotAt(fields, 50, 50)!.edge).toBe(10);
  expect(plotAt(fields, 50, 40)!.edge).toBe(0);
  expect(plotAt(fields, 50, 30)!.plot).not.toBe(plotAt(fields, 50, 50)!.plot);
  expect(plotAt(fields, 50, 70)!.plot).not.toBe(plotAt(fields, 50, 50)!.plot);
});

/** One country road along y = 50, 8 m wide, cut square at both ends. */
const COUNTRY = SURFACE_AREA_KINDS.indexOf("country_road");
const STREET = SURFACE_AREA_KINDS.indexOf("road");
const throughRoad: PlotSite = {
  ...empty,
  surfaceStrokes: Float32Array.of(0, 50, 300, 50, 4, COUNTRY, CUT_A | CUT_B),
};
/** A country road is a street through built ground, looked for 5 m beside
 *  it, and across a gap in it of under 40 m; the street's surface is carried
 *  6 m on under the road's. */
const asStreet = SURFACE_AREA_KINDS.map((_, tag) =>
  tag === COUNTRY ? { as: STREET, besideM: 5, gapM: 40, carryM: 6 } : null,
);
/** Each drawn stretch's `[from x, to x, kind]`, for a road along x. */
const pieces = (drawn: Float32Array) => {
  const out = [];
  for (let o = 0; o < drawn.length; o += STROKE_FLOATS)
    out.push([Math.round(drawn[o]), Math.round(drawn[o + 2]), drawn[o + 5]]);
  return out;
};
/** Houses either side of the road from x = 100 to 200. */
const builtBeside = (x: number) => x >= 100 && x <= 200;

test("a country road is drawn as a street between built ground, and as itself beyond", () => {
  const drawn = drawnStrokes(throughRoad, asStreet, builtBeside);
  // The road stops at the houses and starts again past them; the street
  // runs its carry on under it, either end.
  expect(pieces(drawn)).toEqual([
    [0, 100, COUNTRY],
    [200, 300, COUNTRY],
    [94, 206, STREET],
  ]);
  const stretches = [];
  for (let o = 0; o < drawn.length; o += STROKE_FLOATS)
    stretches.push(Array.from(drawn.subarray(o, o + STROKE_FLOATS)));
  // The street ends square; the road ends round over it, and keeps its own
  // square ends.
  expect(stretches.map((s) => s[STROKE_CUTS])).toEqual([CUT_A, CUT_B, CUT_A | CUT_B]);
  // All on the road's own line, as wide.
  for (const s of stretches) expect([s[1], s[3], s[4]]).toEqual([50, 50, 4]);
  expect(stretches[0][0]).toBe(0);
  expect(stretches[1][2]).toBe(300);
});

test("a street runs on across a short gap between a town's yards, and ends at a long one", () => {
  const yards = (spans: [number, number][]) => (x: number) =>
    spans.some(([from, to]) => x >= from && x <= to);
  const drawn = drawnStrokes(
    throughRoad,
    asStreet,
    yards([
      [60, 100],
      [130, 170],
      [230, 260],
    ]),
  );
  expect(pieces(drawn).filter(([, , kind]) => kind === STREET)).toEqual([
    [54, 176, STREET],
    [224, 266, STREET],
  ]);
  expect(pieces(drawn).filter(([, , kind]) => kind === COUNTRY)).toEqual([
    [0, 60, COUNTRY],
    [170, 230, COUNTRY],
    [260, 300, COUNTRY],
  ]);
});

test("a street's run through a town carries on from one stretch of its road to the next", () => {
  // The same road in two stretches that meet at x = 150, inside the town.
  const bent: PlotSite = {
    ...empty,
    surfaceStrokes: Float32Array.from(
      [
        [0, 50, 150, 50, 4, COUNTRY, CUT_A],
        [150, 50, 300, 50, 4, COUNTRY, CUT_B],
      ].flat(),
    ),
  };
  const drawn = drawnStrokes(bent, asStreet, builtBeside);
  expect(pieces(drawn)).toEqual([
    [0, 100, COUNTRY],
    [200, 300, COUNTRY],
    [94, 150, STREET],
    [150, 206, STREET],
  ]);
  const cuts = [];
  for (let o = 0; o < drawn.length; o += STROKE_FLOATS) cuts.push(drawn[o + STROKE_CUTS]);
  // The street is not cut where its two stretches meet.
  expect(cuts).toEqual([CUT_A, CUT_B, CUT_A, CUT_B]);
});

test("a road with houses along one side only stays a country road", () => {
  expect(drawnStrokes(throughRoad, asStreet, (x, y) => builtBeside(x) && y > 50)).toBe(
    throughRoad.surfaceStrokes,
  );
});

test("drawing a road as a street through a town moves no edge of it", () => {
  const drawn = drawnStrokes(throughRoad, asStreet, builtBeside);
  const inside = (strokes: Float32Array, x: number, y: number) => {
    let deepest = -Infinity;
    for (let o = 0; o < strokes.length; o += STROKE_FLOATS)
      deepest = Math.max(deepest, strokeInside(strokes, o, x, y));
    return deepest;
  };
  // An edge is where the depth is 0: within a metre of it nothing may move.
  // (Deeper in, a street's square end is nearer than the road's edge was.)
  const near = (depth: number) => Math.min(depth, 1);
  // Across the road at its ends, at each change of kind, and between.
  for (const x of [-3, 0, 2, 60, 98, 100, 102, 150, 199.5, 200.5, 298, 300, 304])
    for (const y of [40, 45.9, 46.1, 50, 53.9, 54.1, 60])
      expect(near(inside(drawn, x, y)), `(${x}, ${y})`).toBeCloseTo(
        near(inside(throughRoad.surfaceStrokes, x, y)),
        4,
      );
});

test("a road no kind draws differently is the site's own strokes", () => {
  const none = SURFACE_AREA_KINDS.map(() => null);
  expect(drawnStrokes(throughRoad, none, builtBeside)).toBe(throughRoad.surfaceStrokes);
  // A kind drawn as itself: the street test map's roads, which are streets by name.
  const itself = SURFACE_AREA_KINDS.map((_, tag) => ({ ...asStreet[COUNTRY]!, as: tag }));
  expect(drawnStrokes(throughRoad, itself, builtBeside)).toBe(throughRoad.surfaceStrokes);
  expect(drawnStrokes(throughRoad, asStreet, () => false)).toEqual(throughRoad.surfaceStrokes);
});
