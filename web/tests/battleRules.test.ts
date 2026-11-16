import { audioMix } from "@web/audioMix";
import { shippedRules } from "@web/shippedRules";
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
import { frameViews } from "@packages/battle-renderer/src/records.gen";
import { Fielding, Presentation, weapon_looks } from "@wasm/renderer_browser";
import {
  SoundFrame,
  battleSounds,
  type SoundFrameOptions,
} from "@packages/battle-audio/src/soundFrame";
import type { AppAudio } from "@packages/battle-audio/src/appAudio";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import { FakeSink } from "./support/soundSink";
import { ghostAloft } from "@apps/battle-lab/src/unitGhosts";

const runtime = new URL("../../assets/runtime/", import.meta.url);

test("wasm fielding admits an unmounted blast-only row and refuses mounted copies by name", async () => {
  await loadPresentation();
  const original = presentationSettings.weapon_looks.tank_he;
  const settings = JSON.stringify({
    ...presentationSettings,
    weapon_looks: {
      ...presentationSettings.weapon_looks,
      blast_only: { blast: original.blast, sound: { blast: original.sound.blast } },
    },
  });
  const { rules } = await resolveCatalogSet("test");
  const weapons = {
    ...rules.weapons,
    ambient_blast: { extends: "tank_he", name: "Ambient blast", appearance: "blast_only" },
  };
  expect(JSON.parse(weapon_looks(settings, JSON.stringify(weapons))).weapons.ambient_blast).toBe(
    "blast_only",
  );
  for (const [catalog, mounted] of [
    [{ units: [], soldiers: {} }, false],
    [{ units: [{ id: "gunner", mounts: [{ weapons: ["tank_ap", "ambient_blast"] }] }] }, true],
    [{ units: [], soldiers: { gunner: { mounts: [{ weapons: ["ambient_blast"] }] } } }, true],
  ] as const) {
    const fielding = new Fielding(
      settings,
      JSON.stringify({ ...rules, weapons }),
      JSON.stringify(catalog),
      JSON.stringify({ sides: { blue: [0, 0, 0], red: [0, 0, 0] } }),
      "[]",
    );
    try {
      const resolve = () => fielding.resolve();
      if (mounted)
        expect(resolve).toThrow("weapons.ambient_blast.appearance: missing firing tracer");
      else expect(resolve).not.toThrow();
    } finally {
      fielding.free();
    }
  }
});

test("consecutive wasm battles field and sound their own resolved encounter rules", async () => {
  const wasm = await loadWasm();
  const sim = await loadSimModule();
  const { memory } = await loadPresentation();
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
    const settings = structuredClone(presentationSettings);
    // Different emitted light makes the two admitted routes observable even
    // when the fixture's tank looks happen to share the same shot sound.
    settings.weapon_looks.tank_he.flash.cast.color = [0.8, 0.3, 0.1];
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
              encounter_gun: { extends: "tank_ap", name: "Encounter gun", appearance: "tank_he" },
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
    const session = composeScenarioCatalog(scenario.rules, wasm);
    const hz = session.rules.tick_hz;
    const battle = sim.createBattle(JSON.stringify(scenario), 7);
    const presentation = new Presentation(battle.observation_layout(), hz);
    const fielding = new Fielding(
      JSON.stringify(settings),
      JSON.stringify(session.rules),
      JSON.stringify(session.units.view),
      JSON.stringify(runtimeCatalog),
      JSON.stringify(["test_tank", "test_supply"]),
    );
    let sound: SoundFrame | undefined;
    try {
      const pending = JSON.parse(fielding.pending_contents()) as {
        bundle: string;
        skeleton?: string;
      }[];
      for (const content of pending) {
        const files = [content.bundle, ...(content.skeleton ? [content.skeleton] : [])].map(
          (hash) =>
            gunzipSync(
              readFileSync(new URL(`${runtimeCatalog.gzip[hash].hash}/bundle.bin`, runtime)),
            ),
        );
        fielding.absorb(
          JSON.stringify(content),
          new Uint8Array(Buffer.concat(files)),
          Uint32Array.from(files, (file) => file.length),
        );
      }
      const sections = fielding.resolve();
      const look = (
        changed ? "tank_he" : base.weapons.tank_ap.appearance
      ) as keyof typeof presentationSettings.weapon_looks;
      // A changed mounted weapon and its look must reach the rendered flash,
      // rather than merely appearing in an exported description.
      presentation.load_world(
        JSON.stringify({
          battle: {
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
        sections.share(),
      );
      sections.free();
      createBattleAudio(app, 7, session);
      const active = frames.at(-1)!;
      sound = active.frame;
      const expected = presentationSettings.weapon_looks[look].sound.shot.near;
      const row = settings.weapon_looks[look];
      if (!("flash" in row) || !row.flash.cast) throw new Error("test look needs a cast flash");
      const total = row.flash.cast.color.reduce((sum, v) => sum + v, 0);
      const expectedColor = row.flash.cast.color.map((v) => v / total);
      const colors: number[][] = [];
      const sawFlash = () =>
        colors.some((c) => c.every((v, i) => Math.abs(v - expectedColor[i]) < 1e-5));
      for (
        let tick = 0;
        tick < 300 && (!active.sink.started.some((v) => v.sound === expected) || !sawFlash());
        tick++
      ) {
        const length = battle.publish("blue");
        const words = new Uint32Array(sim.memory.buffer, battle.publication_ptr(), length).slice();
        const time = battle.tick() / hz;
        presentation.push_publication(words, time);
        const ptr = presentation.frame(
          time,
          new Float64Array([100, 170, 20, 150, 200, 0, 0, 0, 1, 1, 0.1, 640, 480]),
          new Uint32Array(),
          false,
          undefined,
          "",
          "",
          0,
        );
        const lights = frameViews(memory.buffer, ptr).lights;
        for (let i = 0; i < lights.count; i++) {
          const rgb = Array.from(lights.f32.subarray(i * 8 + 4, i * 8 + 7));
          const intensity = rgb.reduce((sum, v) => sum + v, 0);
          if (intensity > 0) colors.push(rgb.map((v) => v / intensity));
        }
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
      expect(colors).toContainEqual(expectedColor.map((v) => expect.closeTo(v, 5)));
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

test("battle sound admits unmounted blast-only rows and refuses them by name when mounted", async () => {
  await loadPresentation();
  const original = presentationSettings.weapon_looks.tank_he;
  const options: SoundFrameOptions = {
    tickHz: 30,
    presentation: audioMix,
    catalog: gameSounds,
    looks: {
      ...presentationSettings,
      weapon_looks: {
        ...presentationSettings.weapon_looks,
        blast_only: { blast: original.blast, sound: { blast: original.sound.blast } },
      },
    },
    weapons: {
      ...shippedRules.weapons,
      ambient_blast: { extends: "tank_he", name: "Ambient blast", appearance: "blast_only" },
    },
    unitCatalog: { units: [], soldiers: {} },
  };
  expect(() => battleSounds(options)).not.toThrow();
  for (const catalog of [
    { units: [{ mounts: [{ weapons: ["tank_ap", "ambient_blast"] }] }] },
    { units: [], soldiers: { gunner: { mounts: [{ weapons: ["ambient_blast"] }] } } },
  ]) {
    options.unitCatalog = catalog;
    expect(() => battleSounds(options)).toThrow(
      "weapons.ambient_blast.appearance: missing firing tracer",
    );
  }
});
