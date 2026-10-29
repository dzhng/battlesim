// The water surface's shape (presentation, not rules; a `WaterLook` row):
// the baked normal map at three scales, drifting with the wind, that the
// sky and the sun reflect in. How it is lit, and how much of the bed shows
// through it, is the water's pass's (`waterPass.ts`). Its numbers are its
// row's, which the water's pass reads too.
import { tgpu, d, std } from "typegpu";
import { cameraWind, typegpuCameraLayout } from "../world/camera";
import { terrainLayout } from "./terrainLayout";
import { WATER_NORMALS_SIZE } from "./waterNormals";

/** The water's normal map at `xy` on a layer of `scale` tiles a metre,
 *  turned by (cos, sin) `turn` off the map's axes and carried downwind
 *  (`wind`, metres) as a normal leaning by `strength`, flattening as a
 *  pixel (`footprint` metres) spans more of its texels than one: the map
 *  has no mips, so a layer finer than the pixel would only alias. */
const waterLayer = tgpu.fn(
  [d.vec2f, d.f32, d.vec2f, d.vec2f, d.f32, d.f32],
  d.vec3f,
)((xy, scale, turn, wind, strength, footprint) => {
  "use gpu";
  const held = 1 - std.smoothstep(1.5, 6, footprint * scale * WATER_NORMALS_SIZE);
  const at = std.sub(xy, wind);
  const uv = std.mul(d.vec2f(at.x * turn.x + at.y * turn.y, at.y * turn.x - at.x * turn.y), scale);
  const slope = std.textureSampleLevel(
    terrainLayout.$.waterNormals,
    terrainLayout.$.waterTiles,
    uv,
    0,
  ).xy;
  // The map's slope is along its own turned axes: turn it back to the map's.
  const along = d.vec2f(slope.x * turn.x - slope.y * turn.y, slope.x * turn.y + slope.y * turn.x);
  return std.normalize(d.vec3f(std.mul(along, -strength * held), 1));
});

/** Two tangent-space normals as one: reoriented normal mapping, `detail`
 *  laid over `base` as if it were on its slope. */
export const blendRnm = tgpu.fn(
  [d.vec3f, d.vec3f],
  d.vec3f,
)((base, detail) => {
  "use gpu";
  const t = std.add(base, d.vec3f(0, 0, 1));
  const u = std.mul(detail, d.vec3f(-1, -1, 1));
  return std.normalize(std.sub(std.mul(t, std.dot(t, u) / t.z), u));
});

/** How far downwind (metres) the wind has carried the water's surface by
 *  the camera's clock: a paused battle's water stands still. */
export const waterCarried = tgpu.fn(
  [],
  d.vec2f,
)(() => {
  "use gpu";
  const wind = cameraWind();
  return std.mul(wind.toward.xy, typegpuCameraLayout.$.cam.time * wind.driftMps);
});

/** The water's normal at `xy` in look `look` (RedGPU
 *  `lakeFragment.wgsl:280-346`): the baked map at three scales, the macro and
 *  detail layers turned off the primary's, each drifting downwind at its own
 *  share of the wind's drift, blended with RNM, and flattening where a pixel
 *  spans more than a ripple (`footprint` metres), so far water lies calm. */
export const waterNormal = tgpu.fn(
  [d.vec2f, d.f32, d.u32],
  d.vec3f,
)((xy, footprint, look) => {
  "use gpu";
  const w = terrainLayout.$.params.waterLooks[look];
  const carried = waterCarried();
  const primary = waterLayer(
    xy,
    w.normals.x,
    d.vec2f(1, 0),
    std.mul(carried, w.drifts.x),
    w.normals.w,
    footprint,
  );
  const macro = waterLayer(
    xy,
    w.normals.y,
    d.vec2f(0.825, 0.565),
    std.mul(carried, w.drifts.y),
    w.normals.w,
    footprint,
  );
  const detail = waterLayer(
    xy,
    w.normals.z,
    d.vec2f(0.454, -0.891),
    std.mul(carried, w.drifts.z),
    w.normals.w,
    footprint,
  );
  const n = blendRnm(blendRnm(macro, primary), detail);
  const shown = 1 - std.smoothstep(w.fades.y, w.fades.z, footprint);
  return std.normalize(std.mix(d.vec3f(0, 0, 1), n, shown));
});
