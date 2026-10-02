// @vitest-environment node
// The village's houses on the one building path, from what ships: the saved
// map's buildings, as references into the baked template art library, drawn
// as their farms until a side has seen one fall and as its ruin after.
import { readFileSync } from "node:fs";
import { afterAll, expect, test } from "vitest";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { gameBuildingStyle } from "@apps/battle-lab/src/gameModels";
import { knownFallen } from "@apps/battle-lab/src/destroyedBuildings";
import { WorldView, world_layout } from "@wasm/game_wasm.js";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import {
  buildingDraws,
  COARSE,
  createBuildingScene,
  POOL,
  selectBuildings,
  setFallenBuildings,
  type BuildingArt,
  type BuildingScene,
} from "@packages/battle-renderer/src/models/buildingPlacements";
import { indexBuildings } from "@packages/battle-renderer/src/models/buildingReferences";
import { MODEL_RECORD_FLOATS } from "@packages/battle-renderer/src/models/modelInstances";
import { mapProps, type KnownProp } from "@packages/battle-renderer/src/models/propAppearance";
import { readWorldExports, type WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import {
  TIER_COUNT,
  templateLibraryPath,
  type RuntimeCatalog,
} from "@packages/scene-assets/src/schema";
import {
  decodeTemplateLibrary,
  templateRows,
  type TemplateState,
} from "@packages/scene-assets/src/templateLibrary";
import { loadMap } from "@web/maps/node";

const runtime = (path: string) =>
  readFileSync(new URL(`../../assets/runtime/${path}`, import.meta.url));
const catalog = JSON.parse(runtime("catalog.json").toString("utf8")) as RuntimeCatalog;
const library = decodeTemplateLibrary(
  new Uint8Array(runtime(templateLibraryPath(catalog.templates!.library))),
);
// Which module is drawn is the question, not where its edges are: a box
// stands for every kit mesh's bounds.
const art: BuildingArt = {
  library,
  bounds: library.modules.map(() => ({ min: [-1, -1, 0], max: [1, 1, 1] })),
};

// The village as the simulation builds it (loading the map starts the WebAssembly).
const map = loadMap("village").definition;
const rules = JSON.stringify(GAME_RULES);
const view = new WorldView(JSON.stringify(map), rules);
afterAll(() => view.free());
const exports = readWorldExports(view);
const props = mapProps(exports, JSON.parse(world_layout(rules)) as WorldLayout);
const index = indexBuildings(exports.buildings, props);

/** The modules the village draws from far off when its buildings are known in
 *  `states`, a building each: every row of its template's state that draws at
 *  the coarsest tier. */
function modulesIn(states: readonly TemplateState[]): string[] {
  const { placed } = index;
  const { rows, modules } = library;
  return states
    .flatMap((state, b) => {
      const range = templateRows(library, placed.templates[placed.template[b]], state);
      return Array.from({ length: range.count }, (_, i) => range.first + i)
        .filter((r) => rows.tiers[r] & (1 << (TIER_COUNT - 1)))
        .map((r) => modules[rows.module[r]].module);
    })
    .sort();
}

/** The whole village from far above, then every module drawn, by name. A
 *  record hidden in place is drawn as nothing. */
function drawn(scene: BuildingScene): string[] {
  const camera = {
    target: [1000, 800, 0] as [number, number, number],
    distance: 2000,
    pitch: 1.5,
    yaw: Math.PI / 2,
    fovY: 0.8,
    aspect: 16 / 9,
    near: 1,
  };
  selectBuildings(scene, setDetailView(createDetailView(), camera, 1080), null);
  const out: string[] = [];
  buildingDraws(scene, (source, module, _tier, first, count) => {
    const records =
      source === POOL
        ? scene.pool.records
        : (source === COARSE ? scene.coarse : scene.ruins!).records;
    for (let i = first; i < first + count; i++) {
      const record = records.subarray(i * MODEL_RECORD_FLOATS, (i + 1) * MODEL_RECORD_FLOATS);
      if (record[12] !== 0) out.push(library.modules[module].module);
    }
  });
  return out.sort();
}

test("a village house stands as its farm until the side has seen it fall, then lies as its ruin", () => {
  const scene = createBuildingScene(index.placed, art, gameBuildingStyle);
  const houses = index.parts.map((): TemplateState => "intact");
  const standing = modulesIn(houses);
  expect(houses.length).toBeGreaterThan(1);
  // Whatever fell that the side did not see, and whatever else it knows of:
  // every house stands.
  const wreck: KnownProp = {
    kind: "heavy_wreck",
    center: [975, 752],
    yaw: 0,
    half: [3.5, 1.8, 1.2],
    baseZ: 0,
    replaces: null,
    authoredProp: null,
  };
  expect(knownFallen(index, [wreck])).toEqual([]);
  expect(drawn(scene)).toEqual(standing);

  // It has seen the first house come down: that house's ruin, and the others
  // as they stood.
  const [house] = index.parts[0];
  const ruin: KnownProp = {
    ...house,
    kind: "ruin",
    half: [house.half[0], house.half[1], 1],
    replaces: house.id,
    authoredProp: house.id,
  };
  setFallenBuildings(scene, knownFallen(index, [ruin]));
  const fallen = modulesIn(houses.map((state, b) => (b === 0 ? "ruin" : state)));
  expect(fallen).not.toEqual(standing);
  expect(drawn(scene)).toEqual(fallen);

  // And forgetting is not a thing a side does, but a new battle is: none known, all stand.
  setFallenBuildings(scene, knownFallen(index, []));
  expect(drawn(scene)).toEqual(standing);
});

test("every building an authored map can place has art for standing and for fallen", () => {
  const boxes = JSON.parse(
    readFileSync(new URL("../../fixtures/building-templates.json", import.meta.url), "utf8"),
  ) as { templates: { id: string }[] };
  const drawnStates = new Map(library.templates.map((t) => [t.id, Object.keys(t.states)]));
  for (const { id } of boxes.templates)
    expect(drawnStates.get(id), id).toEqual(expect.arrayContaining(["intact", "ruin"]));
});
