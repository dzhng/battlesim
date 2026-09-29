// Cast lights: the light combat throws on what is round it. A muzzle flash, a
// missile's motor, a tracer, a burst or a burning wreck is a short-lived point
// light, and every world material (ground, grass, scenery, models, water) and
// every lit smoke sprite takes the same lights through the environment's one
// `shade` (`world/environment.ts`), so a flash lights the ground, the grass,
// the house wall and the tank beside it alike.
//
// The effects (`effects/effectFrame.ts`) say which lights burn at the
// presentation clock: each has a published cause, as every effect does. This
// module owns their shape on both sides of the GPU boundary: the list the
// effects fill each frame (`CastLightList`, bounded), the uniform the world
// reads (`CastLights`, at most `CAST_LIGHTS_MAX`), and the choice of which to
// keep when more burn than it holds (`packCastLights`: the strongest as the
// camera sees them).
//
// The light model: irradiance `intensity · (1 − x²)² / (1 + CAST_FALLOFF·x²)`
// at x = d/r of its radius r, none past it (a bright heart and a long soft
// tail, as inverse square gives, windowed to zero at r so no pool has a
// rim), on the surface's albedo (partly greyed, held above a floor) as
// Lambert (over π, so an intensity reads against the sun's radiance), the
// facing wrapped by `wrap` (a pool on flat ground from a light a metre up
// still reaches its edge). No shadow: a light is short and near what it
// lights. Each light costs every lit fragment a loop step, so the uniform
// holds few, the strongest.
import { vec3 } from "math";
import { frustum, sphere, type Frustum } from "math/shapes";
import { d } from "typegpu";

/** How fast a light falls off inside its radius (the model above): 0 is the
 *  plain window, which reads as a spotlight's disc with a shoulder. */
export const CAST_FALLOFF = 6;
/** How much of a surface's own hue a cast light ignores, 0..1: its albedo
 *  is pulled this far toward its own grey before the light multiplies it,
 *  so a warm pool on green grass reads warm, not lime. */
export const CAST_ALBEDO_GREY = 0.35;
/** The least albedo a cast light sees (linear): dark grass takes a pool
 *  near what the pale road beside it does, so no pool stops at a verge. */
export const CAST_ALBEDO_FLOOR = 0.1;
/** Lights the world's uniform holds (fixed at module load: a uniform array). */
export const CAST_LIGHTS_MAX = 24;
/** Candidates the effects may offer in one frame; past it a light is dropped. */
const CAST_LIGHT_CANDIDATES = 1024;
/** Floats per candidate: x, y, z, radius, r, g, b (colour × intensity), unused. */
const CAST_LIGHT_FLOATS = 8;

/** One light as the GPU reads it: position and 1/radius², colour × intensity. */
const CastLight = d.struct({
  /** xyz, and 1/radius² (0 for a light that reaches nothing). */
  position: d.vec4f,
  /** rgb irradiance at its centre, unused. */
  color: d.vec4f,
});
/** The frame's lights: count and wrap, then the lights. */
export const CastLights = d.struct({
  /** x: the lights in use; y: the facing wrap; zw unused. */
  header: d.vec4f,
  lights: d.arrayOf(CastLight, CAST_LIGHTS_MAX),
});
/** `CastLights`' bytes holding its first `count` lights: the 16 B header
 *  and 32 B a light. */
export const castLightsBytes = (count: number) => 16 + count * 32;
/** `CastLights`' whole bytes. */
export const CAST_LIGHTS_BYTES = castLightsBytes(CAST_LIGHTS_MAX);

/** The lights burning this frame, as the effects offer them (candidates). */
export interface CastLightList {
  data: Float32Array<ArrayBuffer>;
  count: number;
  /** Lights that did not fit the candidate list this frame. */
  dropped: number;
  /** How far round a surface every light reaches past facing it, 0..1
   *  (`presentation.effects.cast_wrap`). */
  wrap: number;
  /** What cast each light (`flash:<kind>`, `round:<kind>`, `impact:<hit>`,
   *  `blast`, `fire:<kind>`): for probes and captures, not the GPU. */
  causes: string[];
}

export function createCastLightList(capacity = CAST_LIGHT_CANDIDATES): CastLightList {
  return {
    data: new Float32Array(capacity * CAST_LIGHT_FLOATS),
    count: 0,
    dropped: 0,
    wrap: 0,
    causes: new Array<string>(capacity).fill(""),
  };
}

/** Offer a light at (x, y, z) of `radius` metres, `color` × `intensity`;
 *  nothing when it is dark or reaches nothing. */
export function offerCastLight(
  list: CastLightList,
  x: number,
  y: number,
  z: number,
  radius: number,
  color: readonly number[],
  intensity: number,
  cause: string,
) {
  if (!(intensity > 0 && radius > 0)) return;
  if (list.count * CAST_LIGHT_FLOATS >= list.data.length) {
    list.dropped++;
    return;
  }
  const o = list.count * CAST_LIGHT_FLOATS;
  const v = list.data;
  v[o] = x;
  v[o + 1] = y;
  v[o + 2] = z;
  v[o + 3] = radius;
  v[o + 4] = color[0] * intensity;
  v[o + 5] = color[1] * intensity;
  v[o + 6] = color[2] * intensity;
  v[o + 7] = 0;
  list.causes[list.count++] = cause;
}

/** The lights offered, as read back (probes and tests): where, how far,
 *  their colour × intensity, and what cast each. */
export function offeredCastLights(list: CastLightList) {
  return Array.from({ length: list.count }, (_, i) => {
    const r = Array.from(list.data.subarray(i * CAST_LIGHT_FLOATS, (i + 1) * CAST_LIGHT_FLOATS));
    return { at: r.slice(0, 3), radius: r[3], rgb: r.slice(4, 7), cause: list.causes[i] };
  });
}

/** A light's weight for the cut: how much of the view it lights, its
 *  strength over its reach, dimmed by its distance from the camera's focus
 *  in camera distances (a light far off lights few pixels). */
function weight(v: Float32Array, o: number, focus: readonly number[], distance: number): number {
  const strength = Math.max(v[o + 4], v[o + 5], v[o + 6]) * v[o + 3] * v[o + 3];
  const dx = v[o] - focus[0];
  const dy = v[o + 1] - focus[1];
  const dz = v[o + 2] - focus[2];
  const far = (dx * dx + dy * dy + dz * dz) / Math.max(distance * distance, 1);
  return strength / (1 + far);
}

const _pack_order = new Int32Array(CAST_LIGHT_CANDIDATES);
const _pack_weight = new Float32Array(CAST_LIGHT_CANDIDATES);

const _pack_sphere = sphere.create();

/** Pack `list` into `out` (a `CastLights` image, `CAST_LIGHTS_BYTES`): every
 *  light that reaches into the view (`sides`, its side planes), while they
 *  fit, else the `CAST_LIGHTS_MAX` of most weight for a camera on `focus`
 *  from `distance` metres. Returns the lights packed. */
export function packCastLights(
  out: Float32Array,
  list: CastLightList,
  sides: Frustum,
  focus: readonly number[],
  distance: number,
): number {
  const order = _pack_order;
  const s = _pack_sphere;
  let n = 0;
  for (let i = 0; i < Math.min(list.count, CAST_LIGHT_CANDIDATES); i++) {
    const o = i * CAST_LIGHT_FLOATS;
    vec3.fromBuffer(s.center, list.data, o);
    s.radius = list.data[o + 3];
    if (frustum.sidesIntersectsSphere(sides, s)) order[n++] = i;
  }
  const kept = Math.min(n, CAST_LIGHTS_MAX);
  if (n > CAST_LIGHTS_MAX) {
    for (let k = 0; k < n; k++)
      _pack_weight[order[k]] = weight(list.data, order[k] * CAST_LIGHT_FLOATS, focus, distance);
    order.subarray(0, n).sort((a, b) => _pack_weight[b] - _pack_weight[a]);
  }
  out[0] = kept;
  out[1] = list.wrap;
  out[2] = 0;
  out[3] = 0;
  const v = list.data;
  for (let k = 0; k < kept; k++) {
    const o = order[k] * CAST_LIGHT_FLOATS;
    const w = 4 + k * 8;
    const r = v[o + 3];
    out[w] = v[o];
    out[w + 1] = v[o + 1];
    out[w + 2] = v[o + 2];
    out[w + 3] = 1 / (r * r);
    out[w + 4] = v[o + 4];
    out[w + 5] = v[o + 5];
    out[w + 6] = v[o + 6];
    out[w + 7] = 0;
  }
  return kept;
}
