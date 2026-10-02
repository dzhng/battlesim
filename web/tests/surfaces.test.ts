// @vitest-environment node
import { expect, test } from "vitest";
import { generatePlots, plotAt, type PlotSite } from "@packages/battle-renderer/src/terrain/plots";
import type { ForestShape } from "@packages/battle-renderer/src/terrain/forestShapes";
import {
  SURFACE_AREA_KINDS,
  type SurfaceAreaKind,
} from "@packages/battle-renderer/src/terrain/surfaces";
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
