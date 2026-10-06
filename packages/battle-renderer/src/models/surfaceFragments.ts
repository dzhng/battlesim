// The fragment stages of the surfaces that are not plain opaque
// (`surfaceParts.ts`), over the models layer's vertex stage, material rows
// and textures (`modelLayer.ts`):
//
// - a cutout cuts itself where depth is written, in the frame's prepass and
//   in the sun's cascades; its colour is the opaque stage's, at the samples
//   the prepass kept;
// - glass is lit like any surface and laid over what is behind it;
// - a room is a cell of the interior atlas, shown unlit.
import { tgpu, d, std } from "typegpu";
import { INTERIOR_ATLAS } from "@packages/scene-assets/src/schema";
import type { EnvironmentFrame } from "../frame/environmentFrame";
import { fogCoverage } from "../frame/fogTerm";
import { FRAME_MSAA, WORLD_OUT } from "../frame/targets";
import { typegpuCameraLayout } from "../world/camera";
import { modelSeen } from "./modelFog";
import { MATERIAL_ROWS, modelLayout, modelSurface, modelVaryings } from "./modelLayer";

/** A material's coverage value at a fragment (`Coverage`, scene-assets): the
 *  base colour's alpha times the normal texture's, 1 without one. */
const coverageValue = tgpu.fn(
  [d.u32, d.vec2f],
  d.f32,
)((material, uv) => {
  "use gpu";
  const row = material * MATERIAL_ROWS;
  const layer = modelLayout.$.materials[row + 4].y;
  const image = std.textureSample(
    modelLayout.$.surface,
    modelLayout.$.tiled,
    uv,
    d.i32(std.max(layer, 0)),
  );
  let value = modelLayout.$.materials[row].w;
  if (layer >= 0) {
    value = value * image.w;
  }
  return value;
});

/** The same named geometry as a unit, shown as a translucent placement cue.
 *  Presentation reuses the x-ray RGBA lanes: ghost runs never enter x-ray. */
export function createGhostFragment(environment: EnvironmentFrame) {
  return tgpu.fragmentFn({ in: modelVaryings, out: d.vec4f })((v) => {
    "use gpu";
    const coverage = coverageValue(v.material, v.uv);
    const cutoff = modelLayout.$.materials[v.material * MATERIAL_ROWS + 4].x;
    if (cutoff > 0 && coverage < cutoff) std.discard();
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) n = std.neg(n);
    const surface = modelSurface(v.color, v.material, v.track, v.uv, n, v.tangent);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const shaded = environment.shade(
      v.xray.xyz,
      d.vec3f(0),
      surface.roughness,
      0,
      surface.metallic,
      surface.occlusion,
      surface.normal,
      v.world,
      sun,
      eye,
    );
    return d.vec4f(shaded.xyz, v.xray.w * coverage);
  });
}
/**
 * How much of a cutout's surface is there at a fragment, 0..1: its coverage
 * value, scaled so that the material's cutoff is half there. Sampled through
 * the mip chain, so a surface too far away to resolve its holes thins by the
 * share of it that is there, instead of vanishing or closing up at one
 * distance.
 */
const cutoutCoverage = tgpu.fn(
  [d.u32, d.vec2f],
  d.f32,
)((material, uv) => {
  "use gpu";
  const cutoff = modelLayout.$.materials[material * MATERIAL_ROWS + 4].x;
  return std.saturate((coverageValue(material, uv) * 0.5) / std.max(cutoff, 0.001));
});
/** Interleaved gradient noise in 0..1 at a pixel (Jimenez): what a cutout's
 *  partial coverage is dithered against. */
const pixelNoise = tgpu.fn(
  [d.vec2f],
  d.f32,
)(/* wgsl */ `(pixel: vec2f) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2f(0.06711056, 0.00583715))));
  }`);
/**
 * The samples of a multisampled pixel a cutout covers: as many of them as its
 * coverage is of the pixel. `edge` is how fast the coverage changes across
 * the pixel. Where it changes fast (an edge the pixel resolves) the count is
 * rounded, a clean step of the frame's antialiasing; where it is an even
 * part (holes too small to resolve) the remainder is dithered by `noise`, so
 * a far grille is an even veil and not bands where its coverage crosses a
 * step.
 */
const coveredSamples = tgpu.fn(
  [d.f32, d.f32, d.f32],
  d.u32,
)(/* wgsl */ `(coverage: f32, edge: f32, noise: f32) -> u32 {
    let threshold = mix(noise, 0.5, clamp(edge * 2.0, 0.0, 1.0));
    let samples = u32(clamp(floor(coverage * ${FRAME_MSAA}.0 + threshold), 0.0, ${FRAME_MSAA}.0));
    return (1u << samples) - 1u;
  }`);
const cutoutVaryings = {
  clip: d.builtin.position,
  uv: d.vec2f,
  material: d.interpolate("flat", d.u32),
};
/** A cutout's depth in the frame's prepass: the samples it covers. The colour
 *  pass shades exactly those (`battleWorldDepth("kept")`), so nothing but
 *  this stage decides the silhouette. */
export const modelCutoutDepth = tgpu.fragmentFn({
  in: cutoutVaryings,
  out: d.builtin.sampleMask,
})((v) => {
  "use gpu";
  const coverage = cutoutCoverage(v.material, v.uv);
  return coveredSamples(coverage, std.fwidth(coverage), pixelNoise(v.clip.xy));
});
/** A cutout's depth in one of the sun's cascades: whether it covers the
 *  cascade's texel, its coverage dithered, so what the cascade cannot resolve
 *  casts its share of shade once the shadow is filtered. */
export const modelCutoutCaster = tgpu.fragmentFn({
  in: cutoutVaryings,
  out: d.builtin.sampleMask,
})((v) => {
  "use gpu";
  return d.u32(cutoutCoverage(v.material, v.uv) > pixelNoise(v.clip.xy));
});

/** `presentation.glass`: how a blended surface (a pane of glass)
 *  takes light. */
export interface GlassStyle {
  /** How far a pane's shading normal turns toward the eye: 0 shades it as
   *  modelled, 1 as if seen half as obliquely. It bounds what a pane mirrors
   *  at a grazing angle, so a window seen along a street stays a dark
   *  opening and never a pale plate of horizon. */
  turn: number;
  /** The brightest a pane's own light gets, as a multiple of a white matte
   *  surface in sun shadow (the environment's `unlit`): the sun's glint off
   *  it never outshines a wall. */
  glint: number;
}
export function validateGlass(glass: GlassStyle): GlassStyle {
  if (!(glass?.turn >= 0 && glass.glint > 0))
    throw new Error(
      `presentation.glass: turn must be ≥ 0 and glint > 0, got ${JSON.stringify(glass)}`,
    );
  return glass;
}

/**
 * A blended surface's fragment: the pane lit like any surface (the one shade
 * function: sun, sky, cast lights, haze), over what is behind it by its
 * coverage value. Nothing refracts and nothing glows, and the fog mask stays
 * what the surface behind the pane wrote. Its light is bounded by `style` so
 * that glass is a dark opening from every side.
 */
export function createGlassFragment(environment: EnvironmentFrame, style: GlassStyle) {
  const { turn, glint } = validateGlass(style);
  return tgpu.fragmentFn({ in: modelVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const view = std.normalize(std.sub(eye, v.world));
    let n = std.normalize(v.normal);
    if (std.dot(n, view) < 0) {
      n = std.neg(n);
    }
    const surface = modelSurface(v.color, v.material, v.track, v.uv, n, v.tangent);
    const albedo = std.mul(surface.albedo, std.mix(d.vec3f(1), v.tint, surface.tint));
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const shaded = environment.shade(
      albedo,
      d.vec3f(0),
      surface.roughness,
      0,
      surface.metallic,
      surface.occlusion,
      std.normalize(std.add(surface.normal, std.mul(view, turn))),
      v.world,
      sun,
      eye,
    );
    const ceiling = std.mul(environment.unlit(d.vec3f(1), v.world, eye).xyz, glint);
    // Its pipeline leaves the fog mask as it is: what is seen is what
    // stands behind the pane.
    return {
      color: d.vec4f(std.min(shaded.xyz, ceiling), coverageValue(v.material, v.uv)),
      fog: d.vec4f(0),
    };
  });
}

/** Which room a window shows, from where its model stands: bits of a hash of
 *  the position's own bits, so it is the same room from every camera, at every
 *  tier and in whatever order the frame packs its models. */
const roomPick = tgpu.fn(
  [d.vec3f],
  d.u32,
)(/* wgsl */ `(anchor: vec3f) -> u32 {
    let bits = bitcast<vec3u>(anchor);
    var h = 0u;
    for (var i = 0; i < 3; i++) {
      // PCG's output permutation, chained over the three coordinates.
      let state = (h ^ bits[i]) * 747796405u + 2891336453u;
      let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
      h = (word >> 22u) ^ word;
    }
    return h;
  }`);
/**
 * Where a room's fragment looks in its sheet: the layer's uv and its mip
 * level. `uv` is the room's box unfolded round its back wall (the unit
 * square; the floor, ceiling and side walls reach one unit out to the open
 * face), so the point's place across the box and its depth into it are read
 * straight off it, and the atlas's pinhole projection (`INTERIOR_ATLAS`,
 * scene-assets) is exact at every fragment. `pick` chooses the cell and
 * whether it is mirrored. The lookup stays half a texel of the level it reads
 * inside its cell, down to the level where a cell is one texel
 * (`cellTexels` across at level 0).
 */
const roomLookup = tgpu.fn(
  [d.vec2f, d.u32, d.f32],
  d.vec3f,
)(/* wgsl */ `(uv: vec2f, pick: u32, cellTexels: f32) -> vec3f {
    let over = max(max(-uv, uv - vec2f(1.0)), vec2f(0.0));
    let depth = 1.0 - max(over.x, over.y);
    let across = clamp(uv, vec2f(0.0), vec2f(1.0)) - vec2f(0.5);
    let shrink = ${INTERIOR_ATLAS.pinhole_m}.0 / (${INTERIOR_ATLAS.pinhole_m}.0 + ${INTERIOR_ATLAS.depth_m} * depth);
    var at = vec2f(0.5) + across * shrink;
    if (((pick >> 16u) & 1u) == 1u) { at.x = 1.0 - at.x; }
    let footprint = max(length(dpdx(at)), length(dpdy(at))) * cellTexels;
    let level = clamp(log2(max(footprint, 0.000001)), 0.0, log2(cellTexels));
    let inset = 0.5 * exp2(ceil(level)) / cellTexels;
    at = clamp(at, vec2f(inset), vec2f(1.0 - inset));
    let cell = pick % ${INTERIOR_ATLAS.cells}u;
    let corner = vec2f(f32(cell % ${INTERIOR_ATLAS.columns}u), f32(cell / ${INTERIOR_ATLAS.columns}u));
    return vec3f((corner + at) / ${INTERIOR_ATLAS.columns}.0, level);
  }`);

/**
 * A room's fragment: its cell of the interior atlas (the material's albedo
 * layer, given by the bake), shown as the finished picture it is. It takes
 * the scene's exposure, the air and the fog, and nothing else: no sun, no
 * shadow, no cast light, and no light of its own.
 */
export function createRoomFragment(environment: EnvironmentFrame) {
  return tgpu.fragmentFn({ in: modelVaryings, out: WORLD_OUT })((v) => {
    "use gpu";
    const row = v.material * MATERIAL_ROWS;
    const layer = modelLayout.$.materials[row + 2].x;
    const cellTexels =
      d.f32(std.textureDimensions(modelLayout.$.albedo).x) / INTERIOR_ATLAS.columns;
    const at = roomLookup(v.uv, roomPick(v.anchor), cellTexels);
    const picture = std.textureSampleLevel(
      modelLayout.$.albedo,
      modelLayout.$.tiled,
      at.xy,
      d.i32(std.max(layer, 0)),
      at.z,
    );
    // A room with no sheet (a preview: the bake binds it) is its flat colour.
    let colour = d.vec3f(modelLayout.$.materials[row].xyz);
    if (layer >= 0) {
      colour = d.vec3f(picture.xyz);
    }
    // A tint-masked room takes its row's tint: how a building dims its rooms.
    colour = std.mul(colour, std.mix(d.vec3f(1), v.tint, modelLayout.$.materials[row + 1].z));
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    const seen = modelSeen(v.world, n, v.anchor, v.clip.xy);
    return {
      color: d.vec4f(environment.unlit(colour, v.world, eye).xyz, 1),
      fog: fogCoverage(seen, 1),
    };
  });
}
