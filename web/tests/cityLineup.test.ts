// @vitest-environment node
// The template line-up lab's own logic: where each template stands, the
// references the frame is handed, a forced tier, and the camera stations at a
// tier's boundary. The drawing is the renderer's, through the same scene a
// generated town uses.
import { expect, test } from "vitest";
import { boundaryRange, tierStyle } from "@apps/battle-lab/src/buildingTier";
import {
  cameraToFit,
  lineUp,
  lineupBuildings,
  lineupFallen,
  lineupIn,
  poseAtRange,
  type LineupTemplate,
} from "@apps/battle-lab/src/cityLineup";
import {
  createGpuMat4,
  createProjectedPoint,
  eyePosition,
  projectPoint,
  viewProjMatrix,
} from "@packages/renderer-core/src/camera3d";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import {
  buildingDraws,
  createBuildingScene,
  selectBuildings,
  type BuildingArt,
} from "@packages/battle-renderer/src/models/buildingPlacements";
import {
  FRAME_FLOATS,
  validateBuildingStyle,
  type BuildingStyle,
} from "@packages/battle-renderer/src/models/buildingReferences";
import type { TemplateArtLibrary } from "@packages/scene-assets/src/templateLibrary";

const part = (
  center: [number, number],
  half: [number, number, number],
  yaw = 0,
  base_z = 0,
): LineupTemplate["parts"][number] => ({ center, yaw, half_extents: half, base_z });

const template = (
  id: string,
  category: string,
  parts: LineupTemplate["parts"],
  set = "test",
): LineupTemplate => ({ id, category, set, parts });

// Houses 5 m tall, a tower 60 m tall, and an L-shaped farm whose second part
// is turned a quarter round and stands off the frame's origin.
const HOUSE_A = template("house-a", "detached_home", [part([0, 0], [5, 4, 2.5])]);
const HOUSE_B = template("house-b", "detached_home", [part([0, 0], [4, 6, 2.5])]);
const TOWER = template("tower", "highrise", [part([0, 0], [13, 13, 30])]);
const FARM = template("farm", "farmstead", [
  part([0, 0], [9, 4, 2]),
  part([12, 8], [6, 2, 3], Math.PI / 2),
]);
const SPACING = { gap_m: 12, row_gap_m: 20, margin_m: 40, backdrop: 2.5, grid_m: 64 };

const overlap = (a: number[], b: number[], c: number[], d: number[]) =>
  a[0] < d[0] && c[0] < b[0] && a[1] < d[1] && c[1] < b[1];

test("every template stands once, inside the map, clear of every other by the gap", () => {
  const lineup = lineUp([TOWER, HOUSE_B, FARM, HOUSE_A], SPACING);
  expect(lineup.entries.map((e) => e.id).sort()).toEqual(["farm", "house-a", "house-b", "tower"]);
  for (const e of lineup.entries) {
    expect(e.min[0]).toBeGreaterThanOrEqual(SPACING.margin_m);
    expect(e.min[1]).toBeGreaterThanOrEqual(SPACING.margin_m);
    expect(e.max[0]).toBeLessThanOrEqual(lineup.size[0] - SPACING.margin_m);
    expect(e.max[1]).toBeLessThanOrEqual(lineup.size[1] - SPACING.margin_m);
    expect(e.min[2]).toBe(0);
  }
  const grown = (e: (typeof lineup.entries)[number]) => [
    [e.min[0] - SPACING.gap_m / 2, e.min[1] - SPACING.gap_m / 2],
    [e.max[0] + SPACING.gap_m / 2, e.max[1] + SPACING.gap_m / 2],
  ];
  for (const a of lineup.entries)
    for (const b of lineup.entries) {
      if (a === b) continue;
      const [a0, a1] = grown(a);
      const [b0, b1] = grown(b);
      expect(overlap(a0, a1, b0, b1), `${a.id} and ${b.id}`).toBe(false);
    }
});

test("the ground runs on behind the line-up, as far as a picture from the south sees past its tallest roof", () => {
  const lineup = lineUp([TOWER, HOUSE_A], SPACING);
  const back = Math.max(...lineup.entries.map((e) => e.max[1]));
  // The tower is 60 m tall: 150 m of ground behind it, and the margin.
  expect(lineup.size[1] - back).toBeGreaterThanOrEqual(SPACING.margin_m + 2.5 * 60);
  // A line-up of houses needs far less.
  const houses = lineUp([HOUSE_A, HOUSE_B], SPACING);
  expect(houses.size[1]).toBeLessThan(lineup.size[1] - 100);
});

test("a category is one row with its fronts on a line, and taller categories stand behind", () => {
  const lineup = lineUp([TOWER, HOUSE_B, FARM, HOUSE_A], SPACING);
  const at = Object.fromEntries(lineup.entries.map((e) => [e.id, e]));
  // The two houses share a row: the same front line, side by side in id order.
  expect(at["house-a"].min[1]).toBe(at["house-b"].min[1]);
  expect(at["house-a"].max[0]).toBeLessThan(at["house-b"].min[0]);
  // Rows run south to north by their tallest building: the camera stands south.
  expect(at["house-a"].max[1]).toBeLessThan(at.farm.min[1]);
  expect(at.farm.max[1]).toBeLessThan(at.tower.min[1]);
  expect(lineup.rows.map((r) => r.category)).toEqual(["detached_home", "farmstead", "highrise"]);
});

test("a template's parts stand in the world as its descriptor places them on its frame", () => {
  const lineup = lineUp([FARM], SPACING);
  const [farm] = lineup.entries;
  const [x, y, z, yaw] = farm.frame;
  expect([z, yaw]).toEqual([0, 0]);
  expect(farm.parts.map((p) => p.center)).toEqual([
    [x, y],
    [x + 12, y + 8],
  ]);
  expect(farm.parts[1]).toMatchObject({ yaw: Math.PI / 2, half: [6, 2, 3], baseZ: 0 });
  // The turned part is 4 m along x and 12 m along y: the box holds it so.
  expect(farm.min[0]).toBeCloseTo(x - 9);
  expect(farm.max[0]).toBeCloseTo(x + 14);
  expect(farm.min[1]).toBeCloseTo(y - 4);
  expect(farm.max[1]).toBeCloseTo(y + 14);
  expect(farm.max[2]).toBe(6);
});

test("the references handed to the frame name each template at its frame, whole or a few", () => {
  const lineup = lineUp([TOWER, HOUSE_B, FARM, HOUSE_A], SPACING);
  const all = lineupBuildings(lineup.entries);
  expect(all.template.length).toBe(4);
  lineup.entries.forEach((e, i) => {
    expect(all.templates[all.template[i]]).toBe(e.id);
    expect([...all.frames.subarray(i * FRAME_FLOATS, (i + 1) * FRAME_FLOATS)]).toEqual(e.frame);
  });
  // One alone keeps where it stood and who it is (its tint is its owner's).
  const tower = lineup.entries.filter((e) => e.id === "tower");
  const solo = lineupBuildings(tower);
  expect(solo.templates).toEqual(["tower"]);
  expect([...solo.frames]).toEqual(tower[0].frame);
  expect(solo.owners[0]).toBe(all.owners[lineup.entries.indexOf(tower[0])]);
});

test("a line-up in a damage state is the templates that end in it, each destroyed into it; the others are left out", () => {
  const lineup = lineUp([FARM, HOUSE_A, TOWER], SPACING);
  // The farm and the house collapse; the tower stands gutted.
  const ends = { farm: "ruin", "house-a": "ruin", tower: "gutted" } as const;
  const library: TemplateArtLibrary = {
    ...LIBRARY,
    templates: Object.entries(ends).map(([id, state]) => ({
      id,
      set: "test",
      status: "release" as const,
      states: { intact: { first: 0, count: 1 }, [state]: { first: 1, count: 1 } },
    })),
  };
  const ids = (state: "intact" | "ruin" | "gutted") =>
    lineupIn(lineup.entries, state, library).map((e) => e.id);
  expect(ids("intact")).toEqual(lineup.entries.map((e) => e.id));
  expect(ids("ruin").sort()).toEqual(["farm", "house-a"]);
  expect(ids("gutted")).toEqual(["tower"]);
  // Each of them is known destroyed into that state; intact, nothing is known.
  const ruins = lineupIn(lineup.entries, "ruin", library);
  expect(lineupFallen(ruins, "ruin")).toEqual([
    { building: 0, state: "ruin" },
    { building: 1, state: "ruin" },
  ]);
  expect(lineupFallen(lineup.entries, "intact")).toEqual([]);
});

// One template whose shell (the whole building at every tier but the finest,
// which adds a window) is exactly its part: what is drawn ends where the
// part does, so a range to the part is a range to the drawn building.
const [SHELL, WINDOW] = [0, 1];
const LIBRARY: TemplateArtLibrary = {
  art_hash: "art",
  covers: "physical",
  kits: [{ appearance: "city_kit_test", bundle: "a" }],
  modules: [
    { kit: 0, module: "shell" },
    { kit: 0, module: "window" },
  ],
  templates: [
    { id: "house-a", set: "test", status: "release", states: { intact: { first: 0, count: 4 } } },
  ],
  rows: {
    module: Uint16Array.of(SHELL, WINDOW, WINDOW, WINDOW),
    transform: Float32Array.from(
      [
        [0, 0, 0, 0, 1, 1, 1],
        [0, 0, 1, 0, 1, 1, 1],
        [1, 0, 1, 0, 1, 1, 1],
        [2, 0, 1, 0, 1, 1, 1],
      ].flat(),
    ),
    // The shell everywhere; a window at tier 0, at tiers 0 and 1, at 0 to 2.
    tiers: Uint8Array.of(15, 1, 3, 7),
    tint: new Uint8Array(12).fill(255),
  },
};
const ART: BuildingArt = {
  library: LIBRARY,
  bounds: [
    { min: [-5, -4, 0], max: [5, 4, 5] },
    { min: [-0.5, -0.1, 0], max: [0.5, 0.1, 1] },
  ],
};
const STYLE: BuildingStyle = validateBuildingStyle({
  lod_px_per_m: [10, 4, 1.2],
  chunk_m: 64,
  pool_records: 1024,
  expand_rows: 100000,
  tint_jitter: 0,
});
const LENS = { fovY: 0.8, aspect: 16 / 9, near: 1 };
const HEIGHT_PX = 1080;

/** The tiers a camera `pose` draws the line-up's one house at. */
function tiersDrawn(style: BuildingStyle, pose: ReturnType<typeof poseAtRange>): number[] {
  const lineup = lineUp([HOUSE_A], SPACING);
  const scene = createBuildingScene(lineupBuildings(lineup.entries), ART, style);
  const view = setDetailView(
    createDetailView(),
    { ...LENS, ...pose, target: [...pose.target, 0] },
    HEIGHT_PX,
  );
  selectBuildings(scene, view, null);
  const tiers = new Set<number>();
  buildingDraws(scene, (source, _module, tier, first, count) => {
    const records = source === 0 ? scene.coarse.records : scene.pool.records;
    // A record hidden in place (a resident chunk's coarse row) draws nothing.
    for (let i = first; i < first + count; i++) if (records[i * 16 + 12] !== 0) tiers.add(tier);
  });
  return [...tiers].sort();
}

test("a forced tier draws the building at that tier from any distance", () => {
  const [house] = lineUp([HOUSE_A], SPACING).entries;
  for (const tier of [0, 1, 2, 3])
    for (const range of [20, 150, 600, 3000])
      expect(
        tiersDrawn(tierStyle(STYLE, tier), poseAtRange(house, range, -1.57, 0.85)),
        `tier ${tier} at ${range} m`,
      ).toEqual([tier]);
  // No tier forced: the style's own thresholds, untouched.
  expect(tierStyle(STYLE, null)).toBe(STYLE);
  expect(tiersDrawn(STYLE, poseAtRange(house, 3000, -1.57, 0.85))).toEqual([3]);
});

test("a station at a tier's boundary stands where the building changes tier", () => {
  const [house] = lineUp([HOUSE_A], SPACING).entries;
  for (const boundary of [1, 2, 3] as const) {
    const range = boundaryRange(STYLE, boundary, LENS.fovY, HEIGHT_PX);
    // The eye is that far from the nearest point of the building.
    const pose = poseAtRange(house, range, -1.57, 0.85);
    const eye = eyePosition([0, 0, 0], { ...LENS, ...pose, target: [...pose.target, 0] });
    const nearest = eye.map((v, axis) => Math.min(house.max[axis], Math.max(house.min[axis], v)));
    expect(Math.hypot(...eye.map((v, axis) => v - nearest[axis]))).toBeCloseTo(range, 3);
    // A step nearer it draws the finer tier, a step farther the coarser.
    expect(
      tiersDrawn(STYLE, poseAtRange(house, range * 0.99, -1.57, 0.85)),
      `inside boundary ${boundary}`,
    ).toEqual([boundary - 1]);
    expect(
      tiersDrawn(STYLE, poseAtRange(house, range * 1.01, -1.57, 0.85)),
      `outside boundary ${boundary}`,
    ).toEqual([boundary]);
  }
  // A metre covers 10 px at 128 m and 1.2 px at 1,064 m, at 1080 pixels.
  expect(boundaryRange(STYLE, 1, 0.8, 1080)).toBeCloseTo(127.7, 0);
  expect(boundaryRange(STYLE, 3, 0.8, 1080)).toBeCloseTo(1064, 0);
});

test("a framing to fit holds the whole box in the picture, as large as it goes", () => {
  const lens = { fovY: 0.8, aspect: 16 / 9, near: 1 };
  const boxes: [number[], number[]][] = [
    [
      [100, 100, 0],
      [190, 139, 11],
    ],
    [
      [40, 40, 0],
      [66, 66, 61],
    ],
    [
      [0, 0, 0],
      [8, 11, 5],
    ],
  ];
  for (const [min, max] of boxes) {
    const camera = cameraToFit(min, max, -1.57, 0.85, lens, 0.9);
    const viewProj = viewProjMatrix(createGpuMat4(), camera);
    const point = createProjectedPoint();
    let reach = 0;
    for (let k = 0; k < 8; k++) {
      const corner = [0, 1, 2].map((axis) => (k & (1 << axis) ? max : min)[axis]);
      const { ndc, clipW } = projectPoint(point, viewProj, corner as [number, number, number]);
      expect(clipW).toBeGreaterThan(0);
      reach = Math.max(reach, Math.abs(ndc[0]), Math.abs(ndc[1]));
    }
    expect(reach).toBeLessThanOrEqual(0.9 + 1e-6);
    expect(reach).toBeGreaterThan(0.89);
    expect([camera.yaw, camera.pitch]).toEqual([-1.57, 0.85]);
  }
});
