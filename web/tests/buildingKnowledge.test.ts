// @vitest-environment node
// Destroyed buildings from the simulation to the state a side draws: the
// shelling encounter on the camera lab's map played in the built WebAssembly,
// each side's publication pushed into a presentation that draws the map's
// buildings, and the state its frame draws each in. A low compound is shelled down and a tower gutted while
// blue watches and red stands behind two slabs; later red walks out and sees.
import { presentationSettings } from "@web/presentationSettings";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { gameBuildingStyle } from "@apps/battle-lab/src/gameModels";
import { buildingRemains } from "@packages/scene-assets/src/authority";
import { shippedBuildingArt } from "./support/presentedBuildings";
import { TEST_RULES, labScenario } from "./catalog";
import { type LabEncounter } from "@apps/battle-lab/src/scenarios";
import { Battle, initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import {
  indexBuildings,
  type BuildingIndex,
} from "@packages/battle-renderer/src/models/buildingReferences";
import {
  knownStanding,
  mapProps,
  type MapProp,
} from "@packages/battle-renderer/src/models/propAppearance";
import { gameBiome } from "@apps/battle-lab/src/gameBiome";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";
import { buildingCollapse } from "@packages/scene-assets/src/authority";
import { UNITS } from "./catalog";
import { ruinHeight } from "@packages/scene-assets/src/templateSource";
import { loadEncounter, loadMap } from "@web/maps/node";
import type { ObservationLayout, ObservationView } from "../src/battle/sim/observationView";
import {
  ScenePresentation,
  loadPresentation,
  type FallenBuilding,
} from "../src/battle/sim/presentation";

await loadPresentation();
const ART = await shippedBuildingArt();

let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

const SEED = 7;
/** A camera over the map: what it frames is not the question. */
const OVERHEAD = {
  eye: [0, 0, 800],
  target: [1, 1, 0],
  up: [0, 0, 1],
  fovY: 0.8,
  near: 1,
  width: 1920,
  height: 1080,
};
/** The compound the encounter shells down, and the tower it guts. */
const [COMPOUND, TOWER] = ["china-apartment-block-u-5f", "china-tower-20f"];

interface Played {
  index: BuildingIndex;
  props: MapProp[];
  exports: WorldExports;
  layout: WorldLayout;
  /** Step the battle to `tick`. */
  to(tick: number): void;
  /** A side's whole observation now. */
  sees(side: "blue" | "red"): ObservationView;
  /** The buildings a side's presentation draws destroyed now, from its
   *  whole publication. */
  drawn(side: "blue" | "red"): FallenBuilding[];
  /** The boxes a side's fog of war is cut by now, from its whole publication. */
  occluders(side: "blue" | "red"): ReturnType<ScenePresentation["fogOccluders"]>;
  /** The building of `template`, as `index` numbers it. */
  building(template: string): number;
  free(): void;
}

/** The camera lab's map with `encounter` on it, as a battle. */
function play(encounter: LabEncounter): Played {
  const map = loadMap("camera-lab").definition;
  const rules = JSON.stringify(TEST_RULES);
  const view = new WorldView(JSON.stringify(map), rules);
  const layout = JSON.parse(world_layout(rules)) as WorldLayout;
  const exports = readWorldExports(view);
  const props = mapProps(exports, layout);
  const index = indexBuildings(exports.buildings, props);
  const battle = new Battle(
    labScenario(map, encounter.units, encounter.events, encounter.scripts),
    SEED,
  );
  const observations = JSON.parse(battle.observation_layout()) as ObservationLayout;
  return {
    index,
    props,
    exports,
    layout,
    to(tick) {
      while (battle.tick() < tick) battle.step();
    },
    sees(side) {
      // A whole publication, read by a decoder with nothing behind it.
      battle.resync_observation();
      const len = battle.publish(side);
      return new ScenePresentation(observations).decode(
        new Float32Array(memory.buffer, battle.publication_ptr(), len),
      )!;
    },
    drawn(side) {
      const presentation = new ScenePresentation(observations);
      try {
        presentation.loadWorld(
          { sun_azimuth: 0, sun_elevation: 1, buildings: gameBuildingStyle },
          new Map(),
          {
            placed: index.placed,
            art: ART,
            partBuilding: index.partBuilding,
            shells: buildingRemains(UNITS).shells,
          },
        );
        battle.resync_observation();
        const len = battle.publish(side);
        presentation.decode(new Float32Array(memory.buffer, battle.publication_ptr(), len));
        presentation.frame(0, OVERHEAD, { selected: [], xray: false, ghosts: [] }, 0);
        return presentation.fallenBuildings();
      } finally {
        presentation.free();
      }
    },
    occluders(side) {
      const presentation = new ScenePresentation(observations);
      try {
        const ground = { exports, layout: JSON.stringify(layout), biome: gameBiome, family: null };
        presentation.setGround(ground, null);
        battle.resync_observation();
        const len = battle.publish(side);
        presentation.decode(new Float32Array(memory.buffer, battle.publication_ptr(), len));
        presentation.frame(0, OVERHEAD, { selected: [], xray: false, ghosts: [] }, 0);
        presentation.setFog({ sensors: TEST_RULES.sensors });
        presentation.presentedGround()!.prepare_fog(undefined, 0, 0, 0)?.free();
        return presentation.fogOccluders();
      } finally {
        presentation.free();
      }
    },
    building: (template) => index.placed.template.indexOf(index.placed.templates.indexOf(template)),
    free() {
      view.free();
      battle.free();
    },
  };
}

const shelling = (): LabEncounter => {
  const saved = loadEncounter("camera-lab", "shelling");
  return {
    units: saved.units as LabEncounter["units"],
    events: (saved.events ?? []) as LabEncounter["events"],
    scripts: (saved.scripts ?? []) as LabEncounter["scripts"],
  };
};
/** The last tick the encounter shells anything. */
const lastBurst = (encounter: LabEncounter) => Math.max(...encounter.events.map((e) => e.tick));

test("a side knows a building collapsed or gutted once it has seen it, and intact until then, however long ago it was", () => {
  const encounter = shelling();
  const battle = play(encounter);
  try {
    const [compound, tower] = [battle.building(COMPOUND), battle.building(TOWER)];
    const drawn = (side: "blue" | "red") => battle.drawn(side);
    // Before a round lands, both sides know the map.
    battle.to(30);
    expect(drawn("blue")).toEqual([]);
    expect(drawn("red")).toEqual([]);

    // Blue watched the shelling: the compound a ruin, the tower gutted. Red,
    // behind the slabs, still knows both intact.
    battle.to(lastBurst(encounter) + 30);
    const watched = [
      { building: Math.min(compound, tower), state: compound < tower ? "ruin" : "gutted" },
      { building: Math.max(compound, tower), state: compound < tower ? "gutted" : "ruin" },
    ];
    expect(drawn("blue")).toEqual(watched);
    expect(drawn("red")).toEqual([]);

    // What blue was published is the simulation's own end of each: every
    // part of the compound as remains at the one ruin height, the tower's
    // part as a shell at its full height.
    const blue = battle.sees("blue").knownProps;
    const rule = buildingCollapse(UNITS)!;
    const heights = (building: number) => {
      const parts = battle.index.parts[building];
      const known = knownStanding(parts, blue, new Set(parts.map((p) => p.id)));
      return {
        parts: parts.length,
        authored: Math.max(...parts.map((p) => p.baseZ + 2 * p.half[2])),
        known: known.map((box) => 2 * box.half[2]),
        kinds: [...new Set(known.map((box) => box.kind))],
      };
    };
    const fell = heights(compound);
    const remains = ruinHeight(fell.authored, rule);
    expect(fell.parts).toBe(3);
    expect(fell.kinds).toEqual(["ruin"]);
    expect(fell.known).toEqual([remains, remains, remains]);
    expect(remains).toBeLessThan(fell.authored / 2);
    const stands = heights(tower);
    expect(stands.kinds).toEqual(["gutted"]);
    expect(stands.known).toEqual([stands.authored]);

    // The fog's occluders follow the same knowledge: for blue the compound's
    // are its remains and the tower's still its full height; for red both
    // stand as the map has them.
    const tops = (side: "blue" | "red", building: number) => {
      const parts = battle.index.parts[building];
      const at = new Set(parts.map((p) => p.center.join()));
      return battle
        .occluders(side)
        .filter((o) => at.has([o.x, o.y].join()))
        .map((o) => o.top - o.base);
    };
    expect(tops("blue", compound)).toEqual([remains, remains, remains]);
    expect(tops("blue", tower)).toEqual([stands.authored]);
    expect(tops("red", compound)).toEqual([fell.authored, fell.authored, fell.authored]);
    expect(tops("red", tower)).toEqual([stands.authored]);

    // What a side knows destroyed smokes for it (the presentation's effects,
    // `crates/presentation`): ruins and gutted shells each have a look.
    expect(Object.keys(presentationSettings.effects.smoke)).toEqual(
      expect.arrayContaining(["gutted", "ruin"]),
    );

    // Long after, red has still not seen either: both intact for it.
    const walks = encounter.scripts[0].tick;
    battle.to(walks);
    expect(drawn("red")).toEqual([]);
    // Then it walks out from behind the slabs and sees them: it knows what
    // blue knows.
    let learned = 0;
    for (let tick = walks; tick < walks + 3000 && drawn("red").length < 2; tick += 30) {
      battle.to(tick + 30);
      learned = tick + 30;
    }
    expect(drawn("red")).toEqual(watched);
    expect(learned).toBeGreaterThan(walks);
    expect(drawn("blue")).toEqual(watched);
  } finally {
    battle.free();
  }
});

test("a side that sees part of a compound come down knows the whole building a ruin", () => {
  // A squad south of the U block sees its front part; its wings stand behind
  // that, on the far side.
  const saved = shelling();
  const compoundBursts = saved.events.filter((e) => "burst" in e && e.burst.point[1] < 200);
  const encounter: LabEncounter = {
    units: [
      { side: "blue", kind: "test_rifle", position: [330, 60], engagement: "return_fire_only" },
    ],
    events: compoundBursts,
    scripts: [],
  };
  const battle = play(encounter);
  try {
    const compound = battle.building(COMPOUND);
    const parts = battle.index.parts[compound];
    expect(parts.every((p) => p.yaw === 0)).toBe(true);
    /** The parts with no point of their plan in the squad's sight now (5 x 5
     *  points over each plan, in the fog's own cells). */
    const outOfSight = () => {
      const { fog } = battle.sees("blue");
      const visible = (x: number, y: number) => {
        const cell = Math.floor(y / fog.cellM) * fog.nx + Math.floor(x / fog.cellM);
        return ((fog.bits[cell >> 5] >>> (cell & 31)) & 1) === 1;
      };
      const steps = [-1, -0.5, 0, 0.5, 1];
      return parts.filter(
        (p) =>
          !steps.some((u) =>
            steps.some((v) => visible(p.center[0] + u * p.half[0], p.center[1] + v * p.half[1])),
          ),
      );
    };
    // The front is in sight; a wing is not, before the shelling or after it.
    battle.to(30);
    const hidden = outOfSight();
    expect(hidden).not.toContain(parts[0]);
    expect(hidden.length).toBeGreaterThan(0);

    battle.to(lastBurst(encounter) + 30);
    expect(outOfSight()).toEqual(hidden);
    const known = battle.sees("blue").knownProps;
    expect(battle.drawn("blue")).toEqual([{ building: compound, state: "ruin" }]);
    // Every part is published, the hidden one too: the simulation ends a
    // building whole, and seeing a part of it reveals all of it.
    expect(known.map((k) => k.authoredProp).sort()).toEqual(parts.map((p) => p.id).sort());
    expect(known.every((k) => k.kind === "ruin")).toBe(true);
  } finally {
    battle.free();
  }
});
