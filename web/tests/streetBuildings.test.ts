// @vitest-environment node
// The street test map's houses on the one building path, from what ships: the saved
// map's buildings, as references into the baked template art library, drawn
// standing until a side has seen one fall and as its ruin after.
import { afterAll, expect, test } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import { TEST_RULES, UNITS } from "./catalog";
import { gameBuildingStyle } from "@apps/battle-lab/src/gameModels";
import { WorldView, world_layout } from "@wasm/game_wasm.js";
import { buildingRemains } from "@packages/scene-assets/src/authority";
import {
  indexBuildings,
  type BuildingArt,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { mapProps, type KnownProp } from "@packages/battle-renderer/src/models/propAppearance";
import { readWorldExports, type WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { TIER_COUNT } from "@packages/scene-assets/src/schema";
import { templateRows, type TemplateState } from "@packages/scene-assets/src/templateLibrary";
import { loadMap } from "@web/maps/node";
import { presentBuildings, shippedBuildingArt } from "./support/presentedBuildings";

await loadPresentation();

const art: BuildingArt = await shippedBuildingArt();
const { library } = art;

// The street test map as the simulation builds it (loading the map starts the WebAssembly).
const map = loadMap("street").definition;
const rules = JSON.stringify(TEST_RULES);
const view = new WorldView(JSON.stringify(map), rules);
afterAll(() => view.free());
const exports = readWorldExports(view);
const props = mapProps(exports, JSON.parse(world_layout(rules)) as WorldLayout);
const index = indexBuildings(exports.buildings, props);

/** The modules the street draws from far off when its buildings are known in
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

/** The street's buildings as its presentation draws them, a side knowing
 *  them by the catalog's building rows. */
const presentStreet = () =>
  presentBuildings(index.placed, art, gameBuildingStyle, undefined, {
    partBuilding: index.partBuilding,
    shells: buildingRemains(UNITS).shells,
  });
type Street = ReturnType<typeof presentStreet>;

/** The whole street from far above, then every module drawn, by name. */
function drawn(street: Street): string[] {
  street.look(
    {
      target: [1000, 800, 0],
      distance: 2000,
      pitch: 1.5,
      yaw: Math.PI / 2,
      fovY: 0.8,
      aspect: 16 / 9,
      near: 1,
    },
    1080,
  );
  return street
    .drawn()
    .map((d) => library.modules[d.module].module)
    .sort();
}

test("a street house stands until the side has seen it fall, then lies as its ruin", () => {
  const walk = presentStreet();
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
  walk.know([wreck]);
  expect(drawn(walk)).toEqual(standing);
  expect(walk.fallen()).toEqual([]);

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
  walk.know([ruin]);
  const fallen = modulesIn(houses.map((state, b) => (b === 0 ? "ruin" : state)));
  expect(fallen).not.toEqual(standing);
  expect(drawn(walk)).toEqual(fallen);
  expect(walk.fallen()).toEqual([{ building: 0, state: "ruin" }]);

  // And forgetting is not a thing a side does, but a new battle is: none known, all stand.
  walk.know([]);
  expect(drawn(walk)).toEqual(standing);
  walk.free();
});
