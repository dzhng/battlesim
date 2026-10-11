// The environment frame: the one owner of the battle's light on the GPU. From
// `presentation.light` it builds the sky (its LUT and background), the PMREM
// environment light, the sun and its cascades, and hands every world material
// the same bind group, `shade` and `sampleSunShadow`. Each frame, `prepare`
// poses the sky and fits the cascades to the camera and the map, and
// `setCastLights` hands every material the effects' lights (`light/castLights.ts`).
//
// Post's settings (exposure, grade, bloom) come from the same light, through
// `post`.
import { vec2 } from "math";
import type { Box3 } from "math/shapes";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { TgpuCommandEncoder, TgpuRenderPass } from "typegpu";
import { createTypegpuEnvironment } from "../world/environment";
import { createTypegpuSunShadow } from "../world/shadow";
import { postSettings, validateLight, type LightPresentation } from "../light/sceneLight";
import type { SkyRays } from "../shaders/physicalSky";
import { receiverRange } from "./receiverRange";
import { CAST_LIGHTS_BYTES, CAST_LIGHTS_MAX } from "../light/castLights";
import { FRAME_MSAA } from "./targets";
import type { GpuRegistry } from "./registry";

export async function createEnvironmentFrame(
  device: GPUDevice,
  registry: GpuRegistry,
  light: LightPresentation,
) {
  validateLight(light);
  const shadow = createTypegpuSunShadow(device, light);
  registry.adopt(shadow.dispose);
  const environment = await createTypegpuEnvironment(device, light, FRAME_MSAA, shadow);
  registry.adopt(environment.dispose);
  const range = vec2.create();
  const castImage = new Float32Array(CAST_LIGHTS_BYTES / 4);
  let castCount = 0;

  return {
    light,
    /** Unit vector toward the sun: the sky, the shading and the cascades all use it. */
    sunDirection: environment.sunDirection,
    /** Exposure, grade and bloom, for post. */
    post: postSettings(light),
    group: environment.group,
    /** The light as raw resources (uniform, PMREM) for raw passes. */
    raw: environment.raw,
    shade: environment.shade,
    unlit: environment.unlit,
    sampleSunShadow: environment.sampleSunShadow,
    /** Pose the sky and the environment for this camera, and fit the
     *  cascades over the part of `box` it sees. */
    prepare(camera: Camera3DParams, view: ArrayLike<number>, rays: SkyRays, box: Box3 | null) {
      environment.setView(view, camera.target);
      environment.sky.setRays(rays);
      receiverRange(range, camera, box);
      shadow.update(camera, range);
    },
    /** This frame's cast lights as the presentation chose them (`CastLight`
     *  rows, at most `CAST_LIGHTS_MAX`) and their facing wrap. */
    setCastLights(rows: Float32Array, count: number, wrap: number) {
      castCount = Math.min(count, CAST_LIGHTS_MAX);
      castImage[0] = castCount;
      castImage[1] = wrap;
      castImage.set(rows.subarray(0, castCount * 8), 4);
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
      draw: (pass: TgpuRenderPass, cameraGroup: (typeof shadow.cameraGroups)[number]) => void,
    ) {
      shadow.encode(encoder, (pass, cascade) => draw(pass, shadow.cameraGroups[cascade]));
    },
    stats() {
      return {
        receiverRange: vec2.clone(range),
        castLights: castCount,
        cascades: shadow.data.cascades.length,
      };
    },
  };
}
export type EnvironmentFrame = Awaited<ReturnType<typeof createEnvironmentFrame>>;
