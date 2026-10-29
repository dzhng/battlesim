// @vitest-environment node
// The browser's shaders, as a second client receives them: one compiled set
// for every battle's light and every canvas size, checked in as
// `packages/battle-renderer/pipelines/` (one WGSL file per pipeline and
// `pipelines.json`, each pipeline's settings and bind group layouts, and the
// sizes the passes allocate by).
// `UPDATE_PIPELINES=1` rewrites the export from the browser's code.
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { createBattleFrame } from "@packages/battle-renderer/src/frame/battleFrame";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import { resolveFogLook, resolveLight } from "@web/battle/sim/presentation";
import { benchWorld } from "../../apps/battle-lab/src/workbench/benchWorld";
import { gameLight } from "../../apps/battle-lab/src/gameLight";
import { gameFogGeometry, gameFogStyle } from "../../apps/battle-lab/src/gameFog";
import {
  gameOverlayGlow,
  gamePaint,
  gameXrayMinHiddenFragmentFraction,
} from "../../apps/battle-lab/src/gameOverlay";
import { gameGlass, gameModelDetail } from "../../apps/battle-lab/src/gameModels";
import { FLIPBOOK_SIZE } from "@packages/battle-renderer/src/effects/flipbooks";
import { GRASS_CLUMP_BYTES } from "@packages/battle-renderer/src/frame/grassClump";
import { SCENE_COLOUR_MIPS } from "@packages/battle-renderer/src/frame/targets";
import { BLOOM_KERNEL_RADII } from "@packages/battle-renderer/src/shaders/post";
import { SKY_LUT_HEIGHT, SKY_LUT_WIDTH } from "@packages/battle-renderer/src/light/skyParameters";
import {
  installGpuGlobals,
  recordingGpuDevice,
  type RecordedStage,
  type Recording,
} from "./support/recordingDevice";

installGpuGlobals(FLIPBOOK_SIZE);

/** Everything the frame compiles when built under `light` for a canvas of
 *  `width`×`height`. */
async function record(light: LightPresentation, width: number, height: number) {
  const { device, recording } = recordingGpuDevice();
  const frame = await createBattleFrame(device, "bgra8unorm", {
    light: await resolveLight(light),
    fogGeometry: gameFogGeometry,
    fogLook: resolveFogLook(gameFogStyle),
    overlayGlow: gameOverlayGlow,
    paint: gamePaint,
    xrayMinHiddenFragmentFraction: gameXrayMinHiddenFragmentFraction,
    models: gameModelDetail,
    glass: gameGlass,
    world: benchWorld(null),
    instances: [],
    width,
    height,
  });
  frame.dispose();
  return recording;
}

/** Another battle's light: every number a map could set differs. */
const dusk: LightPresentation = {
  ...gameLight,
  sun_azimuth: gameLight.sun_azimuth + 1.3,
  sun_elevation: 0.21,
  sky: {
    ...gameLight.sky,
    turbidity: gameLight.sky.turbidity + 2.5,
    radiance: gameLight.sky.radiance * 1.4,
    clouds: {
      ...gameLight.sky.clouds,
      coverage: 0.7,
      height_m: 2500,
      scale_m: 900,
      softness: 0.2,
      radiance: 3,
      shade: [0.5, 0.45, 0.6],
    },
  },
  haze: {
    ...gameLight.haze,
    distance_scale: gameLight.haze.distance_scale * 2,
    clear_radius_m: 150,
    overview_from_m: 300,
    range_near_m: 50,
    range_far_m: 4000,
    range_power: 1.7,
    range_strength: 0.3,
    sun_tint: [1, 0.6, 0.3],
    sun_strength: 0.4,
    sun_power: 9,
    mist_color: [0.6, 0.6, 0.7],
    mist_height_m: [2, 30],
    mist_distance_m: [100, 900],
    mist_color_strength: 0.5,
    mist_opacity: 0.3,
  },
  bloom: { strength: 0.9, radius: 0.6, threshold: 2.5, smooth_width: 0.7 },
  exposure: gameLight.exposure * 1.5,
};

const EXPORT = new URL("../../packages/battle-renderer/pipelines/", import.meta.url);
const LISTING = "pipelines.json";
const STAGES = ["vertex", "fragment", "compute"] as const;

/** What the passes allocate by, beside their pipelines: the bloom's levels
 *  (one `bloomBlur<level>` pipeline each), the sky lookup's texels, the
 *  effect sheets' side, a grass clump's stride in bytes and the levels of
 *  the world's copy the water refracts (`sceneColourMip`). */
const SIZES = {
  bloomLevels: BLOOM_KERNEL_RADII.length,
  skyLut: [SKY_LUT_WIDTH, SKY_LUT_HEIGHT],
  effectSheet: FLIPBOOK_SIZE,
  grassClump: GRASS_CLUMP_BYTES,
  sceneColourMips: SCENE_COLOUR_MIPS,
};

/** The export's files by name: each pipeline's WGSL, and the listing. */
function exportOf(recording: Recording): Map<string, string> {
  const files = new Map<string, string>();
  const layouts: Record<string, unknown[]> = {};
  for (const { label, entries } of recording.layouts) {
    const named = entries.map(({ visibility, ...entry }) => ({
      ...entry,
      visibility: STAGES.filter((_, bit) => visibility & (1 << bit)),
    }));
    if (label in layouts && JSON.stringify(layouts[label]) !== JSON.stringify(named))
      throw new Error(`two different bind group layouts are named ${label}`);
    layouts[label] = named;
  }
  const pipelines = recording.pipelines.map(({ label, kind, layout, ...state }) => {
    if (!label || files.has(`${label}.wgsl`))
      throw new Error(`every pipeline needs its own name ($name): "${label}"`);
    const stages = [state.vertex, state.fragment, state.compute].filter(
      (s): s is RecordedStage => s !== undefined,
    );
    const [module] = new Set(stages.map((s) => s.module));
    if (stages.some((s) => s.module !== module))
      throw new Error(`${label}: its stages come from different modules`);
    const shader = `${label}.wgsl`;
    files.set(shader, recording.modules[module].code);
    const entry = ({ module: _, ...stage }: RecordedStage) => stage;
    return {
      name: label,
      kind,
      shader,
      layout: layout === "auto" ? layout : layout.map((i) => recording.layouts[i].label),
      ...state,
      vertex: state.vertex && entry(state.vertex),
      fragment: state.fragment && entry(state.fragment),
      compute: state.compute && entry(state.compute),
    };
  });
  files.set(LISTING, `${JSON.stringify({ sizes: SIZES, layouts, pipelines }, null, 2)}\n`);
  return files;
}

describe("the browser's shader set", () => {
  it("is one compiled set for every battle's light and every canvas size", async () => {
    const noon = await record(gameLight, 1280, 720);
    const evening = await record(dusk, 1003, 611);
    expect(evening.modules.length).toBe(noon.modules.length);
    noon.modules.forEach((module, i) =>
      expect(evening.modules[i].code, `${i}: ${module.label}`).toBe(module.code),
    );
    expect(evening.pipelines).toEqual(noon.pipelines);
  });

  it("is the checked-in export, so a second client compiles the browser's own shaders", async () => {
    const files = exportOf(await record(gameLight, 1280, 720));
    if (process.env.UPDATE_PIPELINES) {
      rmSync(EXPORT, { recursive: true, force: true });
      mkdirSync(EXPORT, { recursive: true });
      for (const [name, text] of files) writeFileSync(new URL(name, EXPORT), text);
    }
    const update = "the browser's shaders changed: UPDATE_PIPELINES=1 rewrites the export";
    expect(readdirSync(EXPORT).sort(), update).toEqual([...files.keys()].sort());
    for (const [name, text] of files)
      expect(readFileSync(new URL(name, EXPORT), "utf8"), `${name}: ${update}`).toBe(text);
  });
});
