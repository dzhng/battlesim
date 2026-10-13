import { tgpu } from "typegpu";
import { Camera } from "../records.gen";

/** The camera every world, shadow and overlay shader binds at group 0. */
export const typegpuCameraLayout = tgpu
  .bindGroupLayout({ cam: { uniform: Camera, visibility: ["vertex", "fragment"] } })
  .$idx(0);
