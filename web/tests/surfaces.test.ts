// @vitest-environment node
import { expect, test } from "vitest";
import { generatePlots, plotAt, type PlotSite } from "@packages/battle-renderer/src/terrain/plots";
import { validateBiome, type Biome } from "@packages/battle-renderer/src/terrain/biome";
import summer from "@fixtures/biomes/summer.json";

// Fixed field rules prevent seeded agricultural cuts from obscuring the guide contract.
const base = validateBiome(summer as unknown as Biome);
const biome: Biome = {
  ...base,
  field_rules: { ...base.field_rules, extent_m: 0, size_m: [1000, 1000], tract_m: 1000 },
};
const empty: PlotSite = {
  map: [0, 0, 100, 100],
  buildings: [],
  surfaceStrokeStride: 6,
  surfaceRunStride: 4,
  surfaceRuns: new Float32Array(0),
  surfaceTriangleStride: 7,
  surfaceBoundaryStride: 5,
  surfaceStrokes: new Float32Array(0),
  surfaceTriangles: new Float32Array(0),
  surfaceBoundaries: new Float32Array(0),
};

function paving(kind: number): PlotSite {
  return {
    ...empty,
    surfaceStrokes: Float32Array.of(0, 50, 100, 50, 2, kind),
    surfaceRuns: kind === 1 ? Float32Array.of(0, 50, 100, 50) : new Float32Array(0),
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

test("sidewalk paving leaves fields unchanged while roads guide their boundaries", () => {
  const untouched = generatePlots(empty, biome);
  expect(generatePlots(paving(4), biome)).toEqual(untouched);
  const road = generatePlots(paving(1), biome);
  expect(plotAt(road, 50, 30)!.plot).not.toBe(plotAt(road, 50, 70)!.plot);
});

test("road polygon guides use only the ring, never the triangulation diagonal", () => {
  const polygon = {
    ...paving(1),
    surfaceStrokes: new Float32Array(0),
    surfaceRuns: new Float32Array(0),
  };
  const fields = generatePlots(polygon, biome);
  expect(plotAt(fields, 50, 50)!.edge).toBe(10);
  expect(plotAt(fields, 50, 40)!.edge).toBe(0);
  expect(plotAt(fields, 50, 30)!.plot).not.toBe(plotAt(fields, 50, 50)!.plot);
  expect(plotAt(fields, 50, 70)!.plot).not.toBe(plotAt(fields, 50, 50)!.plot);
});
