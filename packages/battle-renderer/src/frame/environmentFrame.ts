// The environment frame: the one owner of the battle's light on the GPU. From
// `presentation.light` it builds the sky (its LUT and background), the PMREM
// environment light, the sun and its cascades, and hands every world material
// the same bind group, `shade` and `sampleSunShadow`. Each frame, `prepare`
// poses the sky and fits the cascades to the camera and the map.
//
// Post's settings (exposure, grade, bloom) come from the same light, through
// `post`; the camera uniform's sun angles through `light`.
import { vec2 } from "math";
import type { Box3 } from "math/shapes";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { TgpuCommandEncoder, TgpuRenderPass } from "typegpu";
import { createTypegpuEnvironment } from "../world/environment";
import { createTypegpuSunShadow } from "../world/shadow";
import { postSettings, validateLight, type LightPresentation } from "../light/sceneLight";
import type { SkyRays } from "../shaders/physicalSky";
import { receiverRange } from "./receiverRange";
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
  const environment = await createTypegpuEnvironment(device, light, undefined, FRAME_MSAA, shadow);
  registry.adopt(environment.dispose);
  const range = vec2.create();

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
    sampleSunShadow: environment.sampleSunShadow,
    /** Pose the sky and the environment for this camera, and fit the
     *  cascades over the part of `box` it sees. */
    prepare(camera: Camera3DParams, view: ArrayLike<number>, rays: SkyRays, box: Box3 | null) {
      environment.setView(view, camera.target);
      environment.sky.setRays(rays);
      receiverRange(range, camera, box);
      shadow.update(camera, range);
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
        cascades: shadow.data.cascades.map((c) => ({
          extent: c.extent,
          texel: c.worldUnitsPerTexel,
        })),
      };
    },
  };
}
export type EnvironmentFrame = Awaited<ReturnType<typeof createEnvironmentFrame>>;
