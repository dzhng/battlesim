// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { buildingUnderRay } from "@apps/battle-lab/src/useStaticWorld";
import {
  initSync,
  WorldView,
  world_layout,
  Battle,
  materialize_template,
  template_catalogue_json,
} from "@wasm/game_wasm.js";
import {
  readWorldExports,
  type WorldExports,
  type WorldLayout,
} from "@packages/battle-renderer/src/worldMesh";
import { ObservationDecoder, type ObservationLayout } from "../src/battle/sim/observation";
let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

test("the public picker and delivered replacements share one physical building owner", () => {
  const descriptor = JSON.parse(
    readFileSync(
      new URL("../../fixtures/parity/templates/asymmetric.json", import.meta.url),
      "utf8",
    ),
  );
  const geometry = JSON.parse(
    materialize_template(
      JSON.stringify(descriptor),
      JSON.stringify({ translation: [400, 300, 0], yaw: 0 }),
    ),
  );
  const hash = JSON.parse(template_catalogue_json(JSON.stringify([descriptor]))).hash;
  const map = {
    size: [800, 600],
    fog_cell_m: 8,
    height_grid_m: 4,
    slope_cutoff_deg: 35,
    template_catalog_hash: hash,
    buildings: [
      {
        owner: 0,
        kind: "building",
        category: descriptor.category,
        regional_family: descriptor.regional_family,
        parts: [
          { part: "main", prop: 0 },
          { part: "wing", prop: 1 },
        ],
        geometry,
      },
    ],
    props: [{ id: 2, kind: "tooth", center: [700, 550], yaw: 0, half_extents: [0.6, 0.6, 0.6] }],
  };
  const rules = structuredClone(GAME_RULES);
  const body = (
    rules.catalog as {
      props?: Record<string, { body: { hp: number; hp_scale?: "fixed" | "building_floor_bands" } }>;
    }[]
  ).find((d) => d.props?.building)!.props!.building.body;
  body.hp = 1000;
  body.hp_scale = "fixed";
  rules.weapons.tank_he.structural_damage = 100;
  rules.weapons.tank_he.blast_radius_m = 20;
  const view = new WorldView(JSON.stringify(map), JSON.stringify(rules));
  const battle = new Battle(
    JSON.stringify({
      map,
      rules,
      units: [
        { side: "blue", kind: "rifle", position: [370, 300], engagement: "return_fire_only" },
      ],
      events: Array.from({ length: 11 }, (_, i) => ({
        tick: i + 1,
        burst: { point: [409, 301], weapon: "tank_he" },
      })),
      scripts: [],
    }),
    11,
  );
  try {
    const layout = JSON.parse(world_layout(JSON.stringify(rules))) as WorldLayout;
    const exports: WorldExports = readWorldExports(view);
    expect(exports.buildings).toEqual({
      catalogueHash: hash,
      buildings: [
        {
          owner: 0,
          kind: "building",
          templateId: descriptor.id,
          category: descriptor.category,
          regionalFamily: descriptor.regional_family,
          parts: [
            { part: "main", prop: 0 },
            { part: "wing", prop: 1 },
          ],
        },
      ],
    });
    expect(Array.from(exports.props).slice(0, 20)).toEqual([
      0,
      0,
      layout.propKinds.indexOf("building"),
      400,
      300,
      0,
      4,
      3,
      4,
      0,
      1,
      0,
      layout.propKinds.indexOf("building"),
      406,
      301,
      0,
      2,
      2,
      4,
      0,
    ]);
    expect(view.raycast(420, 301, 2, -1, 0, 0, 30)[7]).toBe(1);
    expect(
      buildingUnderRay({ view, layout, exports }, { origin: [420, 301, 2], dir: [-1, 0, 0] }),
    ).toBe(0);
    const seatedRules = structuredClone(rules);
    seatedRules.garrison.enter_exit_s = 0.25;
    const seated = new Battle(
      JSON.stringify({
        map,
        rules: seatedRules,
        units: [
          { side: "blue", kind: "rifle", position: [410, 301], engagement: "return_fire_only" },
        ],
        events: [],
        scripts: [{ tick: 1, side: "blue", order: { kind: "garrison", units: [0], building: 1 } }],
      }),
      11,
    );
    try {
      const layout = JSON.parse(seated.observation_layout()) as ObservationLayout;
      const decoder = new ObservationDecoder(layout);
      let state = null;
      for (let tick = 0; tick < 600; tick++) {
        seated.step();
        const len = seated.publish("blue");
        state = decoder.decode(new Float32Array(memory.buffer, seated.publication_ptr(), len))!
          .own[0].garrison;
        if (state?.phase === "inside") break;
      }
      expect(state).toEqual({
        building: 0,
        phase: "inside",
        progress: 1,
        center: [402, 300],
        half: [6, 3],
      });
    } finally {
      seated.free();
    }
    const observations = JSON.parse(battle.observation_layout()) as ObservationLayout;
    const decoder = new ObservationDecoder(observations);
    const publish = (side: "blue" | "red") => {
      const len = battle.publish(side);
      return decoder.decode(new Float32Array(memory.buffer, battle.publication_ptr(), len))!;
    };
    const initial = publish("blue");
    const frozen = JSON.stringify(initial);
    for (let tick = 0; tick < 12; tick++) battle.step();
    const collapsed = publish("blue");
    expect(
      collapsed.knownProps.map((p) => ({
        id: p.id,
        building: p.building,
        owner: p.structureOwner,
        source: p.authoredProp,
        replaces: p.replaces,
        center: p.center,
        half: p.half,
      })),
    ).toEqual([
      { id: 3, building: 0, owner: 3, source: 0, replaces: 0, center: [400, 300], half: [4, 3, 1] },
      { id: 4, building: 0, owner: 3, source: 1, replaces: 1, center: [406, 301], half: [2, 2, 1] },
    ]);
    const retained = JSON.stringify(collapsed);
    publish("red");
    battle.resync_observation();
    publish("blue");
    expect(JSON.stringify(initial)).toBe(frozen);
    expect(JSON.stringify(collapsed)).toBe(retained);
  } finally {
    view.free();
    battle.free();
  }
});
