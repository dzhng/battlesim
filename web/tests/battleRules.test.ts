import { gunzipSync } from "node:zlib";
// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { loadWasm, loadSimModule } from "@web/battle/sim/module";
import { loadPresentation } from "@web/battle/sim/presentation";
import { composeScenarioCatalog } from "@web/battle/catalog/compose";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import { presentationSettings } from "@web/presentationSettings";
import { createBattleAudio } from "@apps/battle-lab/src/soundFeed";
import { Fielding, Presentation } from "@wasm/renderer_browser";
import { SoundFrame } from "@packages/battle-audio/src/soundFrame";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import { FakeSink } from "./support/soundSink";
import { ghostAloft } from "@apps/battle-lab/src/unitGhosts";

const runtime = new URL("../../assets/runtime/", import.meta.url);

test("consecutive wasm battles field and sound their own resolved encounter rules", async () => {
  const wasm = await loadWasm();
  const sim = await loadSimModule();
  await loadPresentation();
  const base = await resolveCatalogSet("test");
  const runtimeCatalog = JSON.parse(readFileSync(new URL("catalog.json", runtime), "utf8"));
  const map = JSON.stringify({
    size: [1000, 600],
    fog_cell_m: 8,
    height_grid_m: 4,
    slope_cutoff_deg: 35,
  });
  const frames: { frame: SoundFrame; sink: FakeSink }[] = [];
  const app = {
    createBattle(options: ConstructorParameters<typeof SoundFrame>[0]) {
      const sink = new FakeSink();
      const frame = new SoundFrame({ ...options, catalog: gameSounds }, sink);
      frames.push({ frame, sink });
      return frame;
    },
  } as unknown as AppAudio;
  for (const changed of [false, true]) {
    const weapon = changed ? "encounter_gun" : "tank_ap";
    const overrides = {
      units: {
        test_tank: {
          mounts: [{ id: "cannon", weapons: [weapon] }],
          sensors: {
            ground_m: 3000,
            low_air_m: 3000,
            high_air_m: 3000,
            sight_shape: { front: 1, side: 1, rear: 1 },
          },
        },
        test_supply: { mounts: [], body: { hull: { hp: 1000000 } } },
      },
      ...(changed
        ? {
            weapons: {
              tank_ap: { appearance: "tank_he" },
              encounter_gun: { extends: "tank_ap", name: "Encounter gun" },
            },
            tick_hz: 20,
            air: { low_air: { cruise_agl_m: 25 } },
          }
        : {}),
    };
    const scenario = JSON.parse(
      wasm.encounter_scenario(
        map,
        JSON.stringify({
          units: [
            { side: "blue", kind: "test_tank", position: [100, 200] },
            {
              side: "red",
              kind: "test_supply",
              position: [250, 200],
              engagement: "return_fire_only",
            },
          ],
          events: [],
          scripts: [],
          overrides,
        }),
        JSON.stringify(base.rules),
      ),
    );
    const session = composeScenarioCatalog(scenario.rules, wasm.resolve_catalog);
    const hz = session.rules.tick_hz;
    const battle = sim.createBattle(JSON.stringify(scenario), 7);
    const presentation = new Presentation(battle.observation_layout(), hz);
    const fielding = new Fielding(
      JSON.stringify(presentationSettings),
      JSON.stringify(session.rules),
      JSON.stringify(session.units.view),
      JSON.stringify(runtimeCatalog),
      JSON.stringify(["test_tank", "test_supply"]),
    );
    let sound: SoundFrame | undefined;
    try {
      const files = fielding
        .contents()
        .map((hash) =>
          gunzipSync(
            readFileSync(new URL(`${runtimeCatalog.gzip[hash].hash}/bundle.bin`, runtime)),
          ),
        );
      const sections = JSON.parse(
        fielding.resolve(
          Buffer.concat(files),
          Uint32Array.from(files, (f) => f.length),
        ),
      );
      const look = (
        changed ? "tank_he" : base.weapons.tank_ap.appearance
      ) as keyof typeof presentationSettings.weapon_looks;
      expect(sections.units.types.test_tank.mounts[0].weapons[0].kind).toBe(weapon);
      expect(sections.effects.presentation.looks.weapons[weapon]).toBe(look);
      sections.tables.forEach((name: string, i: number) =>
        presentation.add_clip_table(name, fielding.table(i)),
      );
      presentation.load_world(
        JSON.stringify({
          battle: {
            units: sections.units,
            effects: sections.effects,
            side: "blue",
            tick_hz: hz,
            soldier: {
              radius_m: session.rules.physics.soldier_radius_m,
              height_m: session.rules.physics.soldier_height_m,
            },
            xray: { own: [0, 0, 0, 0], selected: [0, 0, 0, 0], ally: [0, 0, 0, 0] },
            blocked: [0, 0, 0, 0],
          },
          sun_azimuth: 0,
          sun_elevation: 1,
        }),
      );
      createBattleAudio(hz, app, 7, session.weapons);
      const active = frames.at(-1)!;
      sound = active.frame;
      const expected = presentationSettings.weapon_looks[look].sound.shot.near;
      for (
        let tick = 0;
        tick < 300 && !active.sink.started.some((v) => v.sound === expected);
        tick++
      ) {
        const length = battle.publish("blue");
        const words = new Uint32Array(sim.memory.buffer, battle.publication_ptr(), length).slice();
        const time = battle.tick() / hz;
        presentation.push_publication(words, time);
        active.sink.time = time;
        sound.notePresented(presentation);
        sound.update(
          time,
          time,
          { vehicles: [], soldiers: [] },
          { position: [100, 200, 2], forward: [1, 0, 0] },
        );
        battle.step();
      }
      expect(active.sink.started.map((v) => v.sound)).toContain(expected);
      expect(ghostAloft(session.units, session.rules.air)("test_transport_heli")).toBe(
        changed ? 25 : 30,
      );
    } finally {
      sound?.reset();
      fielding.free();
      presentation.free();
      battle.free();
    }
  }
});
