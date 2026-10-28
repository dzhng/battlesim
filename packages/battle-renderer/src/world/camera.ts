import { tgpu } from "typegpu";
import { Camera, Wind } from "../records.gen";

/** The camera every world, shadow and overlay shader binds at group 0. */
export const typegpuCameraLayout = tgpu
  .bindGroupLayout({ cam: { uniform: Camera, visibility: ["vertex", "fragment"] } })
  .$idx(0);

/** The one wind (`Camera.wind`), on the camera's clock (`Camera.time`):
 *  grass, water and trees move in it, in every pass, shadows included. */
export const cameraWind = tgpu
  .fn(
    [],
    Wind,
  )(/* wgsl */ `() -> Wind { return typegpuCameraLayout.$.cam.wind[0]; }`)
  .$uses({ typegpuCameraLayout });
