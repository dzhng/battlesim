// @vitest-environment node
// Massing: artless buildings as one box a physical part, drawn from what the
// side knows, through the static chunk path.
import { expect, test } from "vitest";
import { color } from "math/color";
import {
  massingInstances,
  massingParts,
  validateMassingStyle,
  type MassingStyle,
} from "@packages/battle-renderer/src/scenery/massing";
import {
  createTierPopulation,
  INSTANCE_FLOATS,
  selectTiers,
  type PlacedInstances,
} from "@packages/battle-renderer/src/scenery/lod";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import type { KnownProp, MapProp } from "@packages/battle-renderer/src/models/propAppearance";
import type { PublicBuildings } from "@packages/battle-renderer/src/worldMesh";

const style: MassingStyle = validateMassingStyle({
  families: ["prototype"],
  tints: { highrise: [0.2, 0.4, 0.8], default: [0.5, 0.5, 0.5] },
  ruin: [0.3, 0.3, 0.3],
});
const building = (owner: number, family: string, category: string, props: number[]) => ({
  owner,
  kind: "building",
  templateId: "t",
  category,
  regionalFamily: family,
  parts: props.map((prop, k) => ({ part: `p${k}`, prop })),
});
const buildings: PublicBuildings = {
  catalogueHash: "h",
  buildings: [
    building(0, "prototype", "highrise", [0, 1]),
    building(2, "prototype", "farmstead", [2]),
    building(3, "api_fixture", "farmstead", [3]),
  ],
};
const box = (id: number, x: number, half: [number, number, number]): MapProp => ({
  id,
  kind: "building",
  center: [x, 50],
  yaw: 0.5,
  half,
  baseZ: 2,
});
const props = [
  box(0, 100, [10, 6, 15]),
  box(1, 130, [4, 4, 6]),
  box(2, 400, [7, 5, 3]),
  box(3, 700, [15, 12, 4]),
  // A tree is no building's part.
  { ...box(9, 800, [0.4, 0.4, 5]), kind: "trunk" },
];
const linear = (srgb: [number, number, number]) => [...color.fromSRGB(srgb)];

/** Each instance as the fields a reader would check. */
function boxes(placed: PlacedInstances) {
  return Array.from(placed.kinds, (_, i) => {
    const r = Array.from(placed.records.subarray(i * INSTANCE_FLOATS, (i + 1) * INSTANCE_FLOATS));
    return {
      at: r.slice(0, 3),
      yaw: r[3],
      // The unit box spans ±1 across and 0 to 1 up.
      half: [r[4], r[5], r[6] / 2],
      tint: r.slice(8, 11),
      height: placed.heights[i],
    };
  });
}

test("only the style's families are massing, each part with its building's category", () => {
  expect([...massingParts(buildings, style)]).toEqual([
    [0, "highrise"],
    [1, "highrise"],
    [2, "farmstead"],
  ]);
});

test("each standing part is one box at its own size, tinted by its building's category", () => {
  const parts = massingParts(buildings, style);
  const placed = boxes(massingInstances(props, [], parts, style));
  expect(placed.length).toBe(3);
  const tower = placed[0];
  expect(tower.at).toEqual([100, 50, 2]);
  expect(tower.yaw).toBeCloseTo(0.5);
  expect(tower.half).toEqual([10, 6, 15]);
  expect(tower.height).toBe(30);
  tower.tint.forEach((c, k) => expect(c).toBeCloseTo(linear([0.2, 0.4, 0.8])[k], 6));
  // A category the style does not list takes the default tint.
  placed[2].tint.forEach((c, k) => expect(c).toBeCloseTo(linear([0.5, 0.5, 0.5])[k], 6));
});

test("a part the side has seen fall is drawn as its remains, or not at all", () => {
  const parts = massingParts(buildings, style);
  const ruin: KnownProp = {
    kind: "ruin",
    center: [100, 50],
    yaw: 0.5,
    half: [10, 6, 1],
    baseZ: 2,
    authoredProp: 0,
    replaces: 0,
  };
  const gone: KnownProp = { ...ruin, authoredProp: 2, replaces: 2, destroyed: true };
  // A wreck, and the ruin of a building that has art, are not massing.
  const wreck: KnownProp = { ...ruin, kind: "heavy_wreck", authoredProp: null, replaces: null };
  const housed: KnownProp = { ...ruin, authoredProp: 3, replaces: 3 };
  const placed = boxes(massingInstances(props, [ruin, gone, wreck, housed], parts, style));
  expect(placed.map((b) => b.height)).toEqual([2, 12]);
  placed[0].tint.forEach((c, k) => expect(c).toBeCloseTo(linear([0.3, 0.3, 0.3])[k], 6));
  expect(placed[1].at).toEqual([130, 50, 2]);
});

test("every massing box draws from the static buffer, from any camera", () => {
  const parts = massingParts(buildings, style);
  const population = createTierPopulation(massingInstances(props, [], parts, style), 1, 128, true);
  const view = {
    // All three in view, the 30 m tower some 64 px tall: a tree that size
    // would sort into a near tier.
    ...setDetailView(
      createDetailView(),
      {
        target: [250, 50, 2],
        distance: 600,
        pitch: 1.5,
        yaw: 0,
        fovY: 0.8,
        aspect: 16 / 9,
        near: 1,
      },
      1080,
    ),
    lodPx: [Infinity, Infinity, Infinity] as [number, number, number],
    shadow: { fall: [1, 0] as [number, number], reach: 2600 },
  };
  selectTiers(population, view);
  expect(population.counts[0]).toEqual([0, 0, 0, 0]);
  const ranged = (ranges: number[]) => ranges.reduce((n, v, k) => (k % 2 ? n + v : n), 0);
  expect(ranged(population.far[0])).toBe(3);
  expect(ranged(population.cast[0])).toBe(3);
});
