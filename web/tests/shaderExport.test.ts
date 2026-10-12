// @vitest-environment node
// The browser's shaders, as a second client receives them: one compiled set
// for every battle's light and every canvas size.
import { describe, expect, it } from "vitest";
import { createBattleFrame } from "@packages/battle-renderer/src/frame/battleFrame";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import { benchWorld } from "../../apps/battle-lab/src/workbench/benchWorld";
import { gameLight } from "../../apps/battle-lab/src/gameLight";
import { gameFogGeometry, gameFogStyle } from "../../apps/battle-lab/src/gameFog";
import {
  gameOverlayGlow,
  gamePaint,
  gameXrayMinHiddenFragmentFraction,
} from "../../apps/battle-lab/src/gameOverlay";
import {
  gameBuildingStyle,
  gameGlass,
  gameModelDetail,
} from "../../apps/battle-lab/src/gameModels";
import { FLIPBOOK_SIZE } from "@packages/battle-renderer/src/effects/flipbooks";
import { installGpuGlobals, recordingGpuDevice } from "./support/recordingDevice";

installGpuGlobals(FLIPBOOK_SIZE);

/** Everything the frame compiles when built under `light` for a canvas of
 *  `width`×`height`. */
async function record(light: LightPresentation, width: number, height: number) {
  const { device, recording } = recordingGpuDevice();
  const frame = await createBattleFrame(device, "bgra8unorm", {
    light,
    fogGeometry: gameFogGeometry,
    fogStyle: gameFogStyle,
    overlayGlow: gameOverlayGlow,
    paint: gamePaint,
    xrayMinHiddenFragmentFraction: gameXrayMinHiddenFragmentFraction,
    models: gameModelDetail,
    glass: gameGlass,
    buildings: gameBuildingStyle,
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
});
