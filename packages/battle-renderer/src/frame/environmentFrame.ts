// The environment frame: the one owner of the battle's light on the GPU. From
// the resolved light (`presentation.light` and its records) it builds the sky
// (its LUT and background), the PMREM environment light, the sun and its
// cascades, and hands every world material the same bind group, `shade` and
// `sampleSunShadow`. Each frame, `prepare` poses the sky and takes the
// cascades the presentation fitted to the camera and the map,
// and `setCastLights` hands every material the effects' lights
// (`light/castLights.ts`). The wind is the camera's (`cameraWind`).
import type { TgpuCommandEncoder, TgpuRenderPass } from "typegpu";
import { createTypegpuEnvironment } from "../world/environment";
import { createTypegpuSunShadow } from "../world/shadow";
import { validateLight, type ResolvedLight } from "../light/sceneLight";
import { CAST_LIGHTS_BYTES } from "../light/castLights";
import { CAST_LIGHT_WORDS, CAST_LIGHTS_MAX, View } from "../records.gen";
import { FRAME_MSAA } from "./targets";
import type { GpuRegistry } from "./registry";

export async function createEnvironmentFrame(
  device: GPUDevice,
  registry: GpuRegistry,
  light: ResolvedLight,
) {
  validateLight(light.settings);
  const shadow = createTypegpuSunShadow(device, light.settings);
  registry.adopt(shadow.dispose);
  const environment = await createTypegpuEnvironment(device, light, FRAME_MSAA, shadow);
  registry.adopt(environment.dispose);
  const castImage = new Float32Array(CAST_LIGHTS_BYTES / 4);
  let castCount = 0;

  return {
    /** The authored light (the shadows, the backdrop and the scenery read it). */
    light: light.settings,
    group: environment.group,
    /** The light as raw resources (uniform, PMREM) for raw passes. */
    raw: environment.raw,
    shade: environment.shade,
    /** `shade` in two: a surface's light, then the air over a colour. */
    surfaceLight: environment.surfaceLight,
    air: environment.air,
    unlit: environment.unlit,
    sampleSunShadow: environment.sampleSunShadow,
    /** Pose the sky and the environment for the frame's `View` record, and
     *  take the cascades the presentation fitted to it (`shadow.rs`). */
    prepare(view: Float32Array, sunShadow: Float32Array, casters: Float32Array, count: number) {
      const at = (word: number) => [...view.subarray(word, word + 3)] as [number, number, number];
      environment.setView(view.subarray(View.view, View.view + 16), at(View.target));
      environment.sky.setRays({
        origin: at(View.sky_origin),
        dx: at(View.sky_dx),
        dy: at(View.sky_dy),
      });
      shadow.update(sunShadow, casters, count);
    },
    /** This frame's cast lights as the presentation chose them (`CastLight`
     *  rows, at most `CAST_LIGHTS_MAX`) and their facing wrap. */
    setCastLights(rows: Float32Array, count: number, wrap: number) {
      castCount = Math.min(count, CAST_LIGHTS_MAX);
      castImage[0] = castCount;
      castImage[1] = wrap;
      castImage.set(rows.subarray(0, castCount * CAST_LIGHT_WORDS), 4);
      environment.setCastLights(castImage, castCount);
    },
    encodeBackground(
      raw: GPUCommandEncoder,
      target: GPUTextureView,
      resolveTarget?: GPUTextureView,
    ) {
      environment.sky.encodeBackground(raw, target, resolveTarget);
    },
    /** One depth pass per cascade; `draw` submits the casters. */
    encodeShadows(
      encoder: TgpuCommandEncoder,
      draw: (
        pass: TgpuRenderPass,
        cameraGroup: (typeof shadow.cameraGroups)[number],
        cascade: number,
      ) => void,
    ) {
      shadow.encode(encoder, (pass, cascade) => draw(pass, shadow.cameraGroups[cascade], cascade));
    },
    stats() {
      return { ...shadow.stats(), castLights: castCount };
    },
  };
}
export type EnvironmentFrame = Awaited<ReturnType<typeof createEnvironmentFrame>>;
