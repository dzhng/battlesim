/** Atlas metadata returned by the shared runtime bake. */
import type { Vec3 } from "math";
export interface ImpostorSpec {
  /** Views round the model, the first looking at its front (+X). */
  yaws: number;
  /** Camera pitches above the horizon, radians, one atlas row each. */
  pitches: readonly number[];
  /** Cell size in atlas pixels. */
  cell: number;
  /** Render scale before the box filter. */
  supersample: number;
}

export interface ImpostorAtlas {
  appearance: string;
  spec: ImpostorSpec;
  width: number;
  height: number;
  center: Vec3;
  radius: number;
  /** RGBA8, display-encoded albedo, alpha = coverage. */
  albedo: Uint8Array;
  /** RGBA8, model-space normal mapped to 0..1, alpha = coverage. */
  normal: Uint8Array;
  /** RGBA8, red = the side-tint mask (how much of a side's tint the surface
   *  takes), alpha = coverage. Albedo is baked untinted. */
  mask: Uint8Array;
  /** sha256 of the metadata and every atlas. */
  hash: string;
}
