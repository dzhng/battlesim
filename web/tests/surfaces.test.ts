// @vitest-environment node
import { expect, test } from "vitest";
import { generatePlots, plotAt, type PlotSite } from "@packages/battle-renderer/src/terrain/plots";
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
};

/** A strip of paving of one kind across the map, as a stroke and as a polygon. */
function paving(named: SurfaceAreaKind): PlotSite {
  const kind = SURFACE_AREA_KINDS.indexOf(named);
  return {
    ...empty,
    surfaceStrokes: Float32Array.of(0, 50, 100, 50, 2, kind, 3),
    surfaceRuns: named === "sidewalk" ? new Float32Array(0) : Float32Array.of(0, 50, 100, 50),
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

test("sidewalk paving leaves fields unchanged while every road kind guides their boundaries", () => {
  const untouched = generatePlots(empty, biome);
  expect(generatePlots(paving("sidewalk"), biome)).toEqual(untouched);
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
 *  it, and across a gap in it of under 40 m. */
const asStreet = SURFACE_AREA_KINDS.map((_, tag) =>
  tag === COUNTRY ? { as: STREET, besideM: 5, gapM: 40 } : null,
);
/** Each drawn stretch's `[from x, to x, kind]`, for a road along x. */
const pieces = (drawn: Float32Array) => {
  const out = [];
  for (let o = 0; o < drawn.length; o += STROKE_FLOATS)
    out.push([Math.round(drawn[o]), Math.round(drawn[o + 2]), drawn[o + 5]]);
  return out;
};
/** Houses north of the road only, from x = 100 to 200. */
const builtBeside = (x: number, y: number) => x >= 100 && x <= 200 && y > 50;

test("a country road is drawn as a street where built ground lies beside it, and as itself beyond", () => {
  const drawn = drawnStrokes(throughRoad, asStreet, builtBeside);
  const stretches = [];
  for (let o = 0; o < drawn.length; o += STROKE_FLOATS)
    stretches.push(Array.from(drawn.subarray(o, o + STROKE_FLOATS)));
  expect(stretches.map((s) => s[5])).toEqual([COUNTRY, STREET, COUNTRY]);
  // One road still: each piece starts where the last ended, on the same
  // line, as wide. The street ends square; the country road starts round
  // over each end, and keeps its own square ends.
  expect(stretches.map((s) => s[STROKE_CUTS])).toEqual([CUT_A, CUT_A | CUT_B, CUT_B]);
  expect(stretches[0].slice(0, 2)).toEqual([0, 50]);
  expect(stretches[2].slice(2, 4)).toEqual([300, 50]);
  for (const [k, at] of [100, 200].entries()) {
    expect(stretches[k].slice(2, 4)).toEqual(stretches[k + 1].slice(0, 2));
    expect(stretches[k][2]).toBeCloseTo(at, 0);
    expect(stretches[k][3]).toBe(50);
  }
  for (const s of stretches) expect(s[4]).toBe(4);
});

test("a street runs on across a short gap between a town's yards, and ends at a long one", () => {
  const yards = (spans: [number, number][]) => (x: number) =>
    spans.some(([from, to]) => x >= from && x <= to);
  expect(
    pieces(
      drawnStrokes(
        throughRoad,
        asStreet,
        yards([
          [60, 100],
          [130, 170],
          [230, 260],
        ]),
      ),
    ),
  ).toEqual([
    [0, 60, COUNTRY],
    [60, 170, STREET],
    [170, 230, COUNTRY],
    [230, 260, STREET],
    [260, 300, COUNTRY],
  ]);
});

test("a street's run through a town carries on from one stretch of its road to the next", () => {
  // The same road in two stretches that meet at x = 150, inside the town and
  // then inside a gap in it.
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
    [100, 150, STREET],
    [150, 200, STREET],
    [200, 300, COUNTRY],
  ]);
  const cuts = [];
  for (let o = 0; o < drawn.length; o += STROKE_FLOATS) cuts.push(drawn[o + STROKE_CUTS]);
  expect(cuts).toEqual([CUT_A, CUT_A, CUT_B, CUT_B]);
  const gapped = drawnStrokes(
    bent,
    asStreet,
    (x) => (x >= 100 && x <= 140) || (x >= 165 && x <= 200),
  );
  expect(pieces(gapped)).toEqual([
    [0, 100, COUNTRY],
    [100, 150, STREET],
    [150, 200, STREET],
    [200, 300, COUNTRY],
  ]);
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
  // A kind drawn as itself: the village's roads, which are streets by name.
  const itself = SURFACE_AREA_KINDS.map((_, tag) => ({ as: tag, besideM: 5, gapM: 40 }));
  expect(drawnStrokes(throughRoad, itself, builtBeside)).toBe(throughRoad.surfaceStrokes);
  expect(drawnStrokes(throughRoad, asStreet, () => false)).toEqual(throughRoad.surfaceStrokes);
});
