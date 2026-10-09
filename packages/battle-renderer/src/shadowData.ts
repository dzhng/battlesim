// Adapted from ~/dev/game battle-renderer/src/shadowData.ts:
// cascade mode only (the single fitted map and its 700-line fit are dropped),
// no crowd culling views, N cascades, splits over the receiver range, and a
// normal bias and PCF radius that scale with each cascade's texel, all from
// `presentation.light.cascades`.
import { mat4, vec3, type Mat4, type Vec2, type Vec3 } from "math";
import { CSM_CASCADES } from "./light/shadowPolicy";
import { cascadeFits, type CascadeFrame } from "./light/cascadePolicy";
import {
  SUN_CASCADE_RECORD_FLOATS,
  SUN_SHADOW_BLOCK_FLOATS,
  SUN_SHADOW_CONTROL_OFFSET,
} from "./shaders/shadow";
import {
  createGpuMat4,
  FINITE_CAMERA_FAR_FALLBACK,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
import { CAMERA_UNIFORM_FLOATS } from "@packages/renderer-core/src/cameraUniform";
import { sunDirection, type CascadeSettings, type LightPresentation } from "./light/sceneLight";

export interface NativeShadowCascade {
  index: number;
  /** The world camera uniform, posed as this cascade's light. */
  camera: Float32Array<ArrayBuffer>;
  extent: number;
  worldUnitsPerTexel: number;
}

export interface NativeShadowData {
  cascades: readonly NativeShadowCascade[];
  /** The receiver block (`SunShadow`). */
  receiver: Float32Array<ArrayBuffer>;
}

const _caster_inverse = createGpuMat4();

/** The world camera uniform (`cameraUniform.ts`'s layout) posed as a cascade's
 *  light: its matrices, eye and near, and a map-sized viewport. */
function casterCamera(
  viewProjection: Mat4,
  position: Vec3,
  near: number,
  mapSize: number,
): Float32Array<ArrayBuffer> {
  const inverse = mat4.invert(_caster_inverse, viewProjection);
  if (!inverse) throw Error("Singular shadow projection");
  const camera = new Float32Array(CAMERA_UNIFORM_FLOATS);
  camera.set(viewProjection);
  camera.set(inverse, 16);
  vec3.toBuffer(camera, position, 32);
  camera[35] = near;
  camera[36] = camera[37] = mapSize;
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

/** The frame's cascade fits for this camera, over the receiver range
 *  `receiver` (near, far view depth). */
export function cascadeFrameFit(
  settings: CascadeSettings,
  camera: Camera3DParams,
  sun: Vec3,
  receiver: Vec2,
): CascadeFrame {
  return cascadeFits({
    camera,
    resolvedFar: FINITE_CAMERA_FAR_FALLBACK,
    unitSunDirection: sun,
    receiverNear: receiver[0],
    receiverFar: receiver[1],
    settings,
  });
}

/** The fitted frame's caster cameras and receiver block. */
function cascadeFrameData(settings: CascadeSettings, frame: CascadeFrame): NativeShadowData {
  const block = new Float32Array(SUN_SHADOW_BLOCK_FLOATS);
  const cascades = frame.cascades.map((fit) => {
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
      camera: casterCamera(fit.viewProjection, fit.position, fit.near, frame.mapSize),
      extent: fit.extent,
      worldUnitsPerTexel: fit.worldUnitsPerTexel,
    };
  });
  for (let i = cascades.length; i < CSM_CASCADES; i++) writeInactiveRecord(block, i);
  block.set([frame.cappedFar, 0, frame.near, 0], SUN_SHADOW_CONTROL_OFFSET);
  return { cascades, receiver: block };
}

function coldFrameData(settings: CascadeSettings): NativeShadowData {
  const receiver = new Float32Array(SUN_SHADOW_BLOCK_FLOATS);
  for (let i = 0; i < CSM_CASCADES; i++) writeInactiveRecord(receiver, i);
  receiver.set([settings.max_far_m, 0, 0, 0], SUN_SHADOW_CONTROL_OFFSET);
  return { cascades: [], receiver };
}

export class NativeShadowFrame {
  private readonly sun: Vec3;
  private current: NativeShadowData;

  constructor(
    private readonly light: LightPresentation,
    private readonly upload: (data: NativeShadowData) => void,
  ) {
    this.sun = sunDirection(light);
    this.current = coldFrameData(light.cascades);
    this.upload(this.current);
  }

  get data(): NativeShadowData {
    return this.current;
  }

  update(camera: Camera3DParams, receiver: Vec2): NativeShadowData {
    const settings = this.light.cascades;
    const next = cascadeFrameData(settings, cascadeFrameFit(settings, camera, this.sun, receiver));
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
