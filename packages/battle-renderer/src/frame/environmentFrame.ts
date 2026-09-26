// The environment frame: the one owner of the battle's light on the GPU. From
// `presentation.light` it builds the sky (its LUT and background), the PMREM
// environment light, the sun and its cascades, and hands every world material
// the same bind group, `shade` and `sampleSunShadow`. Each frame, `prepare`
// poses the sky and fits the cascades to the camera and the map.
//
// Post's settings (exposure, grade, bloom) come from the same light, through
// `post`; the camera uniform's sun angles through `light`.
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { TgpuCommandEncoder, TgpuRenderPass } from "typegpu";
import { createTypegpuEnvironment } from "../world/environment";
import { createTypegpuSunShadow } from "../world/shadow";
import { postSettings, validateLight, type LightPresentation } from "../light/sceneLight";
import type { SkyRays } from "../shaders/physicalSky";
import { receiverRange, type MapBox } from "./receiverRange";
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
  let range: [number, number] = [0, 0];

  return {
    light,
    /** Unit vector toward the sun: the sky, the shading and the cascades all use it. */
    sunDirection: environment.sunDirection,
    /** Exposure, grade and bloom, for post. */
    post: postSettings(light),
    group: environment.group,
    shade: environment.shade,
    sampleSunShadow: environment.sampleSunShadow,
    /** Pose the sky and the environment for this camera, and fit the
     *  cascades over the part of `box` it sees. */
    prepare(camera: Camera3DParams, view: ArrayLike<number>, rays: SkyRays, box: MapBox | null) {
      environment.setView(view, camera.target);
      environment.sky.setRays(rays);
      range = receiverRange(camera, box);
      shadow.update(camera, range);
    },
    encodeBackground(raw: GPUCommandEncoder, target: GPUTextureView) {
      environment.sky.encodeBackground(raw, target);
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
        receiverRange: range,
        cascades: shadow.data.cascades.map((c) => ({
          extent: c.extent,
          texel: c.worldUnitsPerTexel,
        })),
      };
    },
  };
}
export type EnvironmentFrame = Awaited<ReturnType<typeof createEnvironmentFrame>>;
