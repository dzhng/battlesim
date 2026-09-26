// Adapted from ~/dev/game battle-renderer/src/shadowData.ts (reuse manifest):
// cascade mode only (the single fitted map and its 700-line fit are dropped),
// no crowd culling views, N cascades, splits over the receiver range, and a
// normal bias and PCF radius that scale with each cascade's texel, all from
// `presentation.light.cascades`.
import { invert, type Mat4 } from "@packages/renderer-core/src/mat4";
import { CSM_CASCADES } from "./light/shadowPolicy";
import { cascadeFits, type CascadeFit } from "./light/cascadePolicy";
import {
  SUN_CASCADE_RECORD_FLOATS,
  SUN_SHADOW_BLOCK_FLOATS,
  SUN_SHADOW_CONTROL_OFFSET,
} from "./shaders/shadow";
import {
  FINITE_CAMERA_FAR_FALLBACK,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
import { sunDirection, type CascadeSettings, type LightPresentation } from "./light/sceneLight";

export type NativeShadowMode = "csm";

/** Floats in one caster camera uniform (192 bytes): the 48-float world camera
 *  layout, posed as the light instead of the eye. */
export const SHADOW_CAMERA_FLOATS = 48;

export interface NativeShadowCascade {
  index: number;
  camera: Float32Array<ArrayBuffer>;
  view: Mat4;
  projection: Mat4;
  viewProjection: Mat4;
  near: number;
  extent: number;
  worldUnitsPerTexel: number;
}

export interface NativeShadowData {
  mode: NativeShadowMode;
  mapSize: number;
  cascades: readonly NativeShadowCascade[];
  receiver: Float32Array<ArrayBuffer>;
  cappedFar: number;
  splitNear: number;
  breaks: readonly number[];
}

function casterCamera(
  viewProjection: Mat4,
  position: readonly number[],
  target: readonly number[],
  near: number,
  far: number,
  mapSize: number,
): Float32Array<ArrayBuffer> {
  const inverse = invert(viewProjection);
  if (!inverse) throw Error("Singular shadow projection");
  const camera = new Float32Array(SHADOW_CAMERA_FLOATS);
  camera.set(viewProjection);
  camera.set(inverse, 16);
  camera.set(position.slice(0, 3), 32);
  camera[35] = near;
  camera.set(target.slice(0, 2), 36);
  camera[38] = camera[39] = mapSize;
  camera[43] = far;
  return camera;
}

function writeRecord(
  receiver: Float32Array<ArrayBuffer>,
  index: number,
  viewProjection: ArrayLike<number>,
  depthBias: number,
  normalBias: number,
  radius: number,
  interval: readonly [number, number],
): void {
  const at = index * SUN_CASCADE_RECORD_FLOATS;
  receiver.set(viewProjection, at);
  receiver.set([depthBias, normalBias, radius, 0], at + 16);
  receiver.set([interval[0], interval[1], 0, 0], at + 20);
}

function writeInactiveRecord(receiver: Float32Array<ArrayBuffer>, index: number): void {
  const at = index * SUN_CASCADE_RECORD_FLOATS;
  receiver.fill(0, at, at + SUN_CASCADE_RECORD_FLOATS);
  for (const diagonal of [0, 5, 10, 15]) receiver[at + diagonal] = 1;
  receiver.set([1, 1, 0, 0], at + 20);
}

/** The PCF radius every cascade gets, in its own texels: the fixture's
 *  penumbra width over the texel, so near and far cascades blur the same
 *  world distance. At least one texel (the filter's floor) and at most four
 *  (five taps grow noisy beyond). */
export function cascadePcfRadius(settings: CascadeSettings, worldUnitsPerTexel: number): number {
  return Math.min(4, Math.max(1, settings.softness_m / worldUnitsPerTexel));
}

/** Normal offset in world metres: a few of the cascade's own texels for each
 *  texel of PCF radius, with a floor. The filter's taps reach `radius` texels
 *  across a receiver that slopes away from the light, so the offset grows with
 *  them; otherwise the wider near-cascade kernel self-shadows lit ground in a
 *  fine noise. Near cascades still keep contact, far ones stay free of acne. */
export function cascadeNormalBias(
  settings: CascadeSettings,
  worldUnitsPerTexel: number,
  radius: number,
): number {
  return Math.max(
    settings.normal_bias_min_m,
    settings.normal_bias_texels * radius * worldUnitsPerTexel,
  );
}

/** The frame's cascades for this camera: fits, caster cameras and the
 *  receiver block. */
export function cascadeFrameData(
  settings: CascadeSettings,
  camera: Camera3DParams,
  sun: readonly [number, number, number],
  receiver: readonly [number, number],
): NativeShadowData {
  const frame = cascadeFits({
    camera,
    resolvedFar: FINITE_CAMERA_FAR_FALLBACK,
    unitSunDirection: sun,
    receiverNear: receiver[0],
    receiverFar: receiver[1],
    settings,
  });
  const block = new Float32Array(SUN_SHADOW_BLOCK_FLOATS);
  const cascades = frame.cascades.map((fit: CascadeFit) => {
    const radius = cascadePcfRadius(settings, fit.worldUnitsPerTexel);
    writeRecord(
      block,
      fit.index,
      fit.viewProjection,
      fit.depthBias,
      cascadeNormalBias(settings, fit.worldUnitsPerTexel, radius),
      radius,
      fit.interval,
    );
    return {
      index: fit.index,
      camera: casterCamera(
        fit.viewProjection,
        fit.position,
        fit.target,
        fit.near,
        fit.far,
        frame.mapSize,
      ),
      view: fit.view,
      projection: fit.projection,
      viewProjection: fit.viewProjection,
      near: fit.near,
      extent: fit.extent,
      worldUnitsPerTexel: fit.worldUnitsPerTexel,
    };
  });
  for (let i = cascades.length; i < CSM_CASCADES; i++) writeInactiveRecord(block, i);
  block.set([frame.cappedFar, cascades.length, frame.near, 0], SUN_SHADOW_CONTROL_OFFSET);
  return {
    mode: "csm",
    mapSize: frame.mapSize,
    cascades,
    receiver: block,
    cappedFar: frame.cappedFar,
    splitNear: frame.near,
    breaks: frame.breaks,
  };
}

function coldFrameData(settings: CascadeSettings): NativeShadowData {
  const receiver = new Float32Array(SUN_SHADOW_BLOCK_FLOATS);
  for (let i = 0; i < CSM_CASCADES; i++) writeInactiveRecord(receiver, i);
  receiver.set([settings.max_far_m, 0, 0, 0], SUN_SHADOW_CONTROL_OFFSET);
  return {
    mode: "csm",
    mapSize: settings.map_size,
    cascades: [],
    receiver,
    cappedFar: settings.max_far_m,
    splitNear: 0,
    breaks: [],
  };
}

export class NativeShadowFrame {
  private readonly sun: readonly [number, number, number];
  private current: NativeShadowData;

  constructor(
    private readonly light: LightPresentation,
    readonly mode: NativeShadowMode,
    private readonly upload: (data: NativeShadowData) => void,
  ) {
    this.sun = sunDirection(light);
    this.current = coldFrameData(light.cascades);
    this.upload(this.current);
  }

  get data(): NativeShadowData {
    return this.current;
  }

  update(camera: Camera3DParams, receiver: readonly [number, number]): NativeShadowData {
    const next = cascadeFrameData(this.light.cascades, camera, this.sun, receiver);
    if (sameCascadeFrame(this.current, next)) return this.current;
    this.current = next;
    this.upload(next);
    return next;
  }
}

function sameCascadeFrame(a: NativeShadowData, b: NativeShadowData): boolean {
  if (a.cascades.length !== b.cascades.length) return false;
  for (let i = 0; i < SUN_SHADOW_BLOCK_FLOATS; i++)
    if (a.receiver[i] !== b.receiver[i]) return false;
  return true;
}
