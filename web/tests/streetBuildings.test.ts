// @vitest-environment node
// The street test map's houses on the one building path, from what ships: the saved
// map's buildings, as references into the baked template art library, drawn
// standing until a side has seen one fall and as its ruin after.
import { readFileSync } from "node:fs";
import { afterAll, expect, test } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import { TEST_RULES, UNITS } from "./catalog";
import { gameBuildingStyle } from "@apps/battle-lab/src/gameModels";
import { knownFallen } from "@apps/battle-lab/src/destroyedBuildings";
import { WorldView, world_layout } from "@wasm/game_wasm.js";
import {
  indexBuildings,
  type BuildingArt,
  type FallenBuilding,
} from "@packages/battle-renderer/src/models/buildingReferences";
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
import { gzipTransport, unpackGzip } from "@packages/scene-assets/src/gzip";
import { presentBuildings } from "./support/presentedBuildings";

await loadPresentation();

const runtime = (path: string) =>
  readFileSync(new URL(`../../assets/runtime/${path}`, import.meta.url));
const catalog = JSON.parse(runtime("catalog.json").toString("utf8")) as RuntimeCatalog;
const hash = catalog.templates!.library;
const gzip = gzipTransport(catalog, hash);
const bytes = await unpackGzip(new Uint8Array(runtime(templateLibraryPath(gzip.hash))), gzip, hash);
const library = decodeTemplateLibrary(bytes);
// Which module is drawn is the question, not where its edges are: a box
// stands for every kit mesh's bounds.
const art: BuildingArt = {
  library,
  bytes,
  bounds: library.modules.map(() => ({ min: [-1, -1, 0], max: [1, 1, 1] })),
};

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

type Street = ReturnType<typeof presentBuildings>;

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

/** What the side knows destroyed is `fallen`; none refused. */
function setFallen(street: Street, fallen: readonly FallenBuilding[]) {
  expect(street.setFallen(fallen)).toEqual([]);
}

test("a street house stands until the side has seen it fall, then lies as its ruin", () => {
  const walk = presentBuildings(index.placed, art, gameBuildingStyle);
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
  expect(knownFallen(UNITS, index, [wreck])).toEqual([]);
  expect(drawn(walk)).toEqual(standing);

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
  setFallen(walk, knownFallen(UNITS, index, [ruin]));
  const fallen = modulesIn(houses.map((state, b) => (b === 0 ? "ruin" : state)));
  expect(fallen).not.toEqual(standing);
  expect(drawn(walk)).toEqual(fallen);

  // And forgetting is not a thing a side does, but a new battle is: none known, all stand.
  setFallen(walk, knownFallen(UNITS, index, []));
  expect(drawn(walk)).toEqual(standing);
  walk.free();
});
