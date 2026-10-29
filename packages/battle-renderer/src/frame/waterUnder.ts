// What the water shows under it: the world before the water
// (`FrameTargets.sceneColour`) and how far down it lies, seen straight
// through the surface, or displaced by its slope (refraction, RedGPU
// `lakeFragment.wgsl:380-413`). The tier sets which (`WaterDetail`): Low
// keeps the depth colour alone (decision 18), so it reads the copy at the
// pixel and builds no mips and no guard.
import { tgpu, d, common, type TgpuCommandEncoder } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";
import { fogLayout } from "./fogTerm";
import { FRAME_MSAA, GUARD_FORMAT, HDR_FORMAT, type FrameTargets } from "./targets";
import { terrainLayout } from "./terrainLayout";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The farthest the water reads down to the bed, metres: past it the bed's
 *  light is all but gone, and the cleared depth (sky) is no distance. */
const BED_FAR_M = 50;

/** The view's NDC at the centre of pixel `at`. */
export const ndcAt = tgpu
  .fn(
    [d.vec2i],
    d.vec2f,
  )(/* wgsl */ `(at: vec2i) -> vec2f {
    let cam = typegpuCameraLayout.$.cam;
    let centre = vec2f(at) + 0.5;
    return vec2f(centre.x / cam.width * 2.0 - 1.0, 1.0 - centre.y / cam.height * 2.0);
  }`)
  .$uses({ typegpuCameraLayout });

/** The mean distance from the water's surface point `world` down the view
 *  to the world's depth at pixel `at`, over the depth's samples, as the
 *  colour is their resolve: one sample alone puts a terrain triangle's edge
 *  into the shallows as a line. */
const bedDistance = tgpu
  .fn(
    [d.vec2i, d.vec3f],
    d.f32,
  )(/* wgsl */ `(at: vec2i, world: vec3f) -> f32 {
    let cam = typegpuCameraLayout.$.cam;
    let ndc = ndcAt(at);
    var far = 0.0;
    for (var s = 0; s < ${FRAME_MSAA}; s++) {
      let p = cam.invViewProj * vec4f(ndc, textureLoad(fogLayout.$.depth, at, s), 1.0);
      far += min(length(p.xyz / p.w - world), ${BED_FAR_M}.0);
    }
    return far / ${FRAME_MSAA}.0;
  }`)
  .$uses({ fogLayout, ndcAt, typegpuCameraLayout });

/** How far down the bed lies straight under fragment `clip` of the water's
 *  surface point `world`: where the water fades in from its edge. */
export const waterDepth = tgpu.fn(
  [d.vec4f, d.vec3f],
  d.f32,
)((clip, world) => {
  "use gpu";
  return bedDistance(d.vec2i(clip.xy), world);
});

/** The texels of a level `size` texels big a bilinear tap at texel
 *  coordinate `u` (centres at whole numbers) may weigh, as (first x, first
 *  y, last x, last y): the two each side straddle, and a third where `u` is
 *  so near a whole number that the sampler's rounding may pick it. */
const tapped = tgpu.fn(
  [d.vec2f, d.vec2f],
  d.vec4i,
)(/* wgsl */ `(u: vec2f, size: vec2f) -> vec4i {
    let last = size - 1.0;
    let first = clamp(floor(u - 1.0 / 256.0), vec2f(0.0), last);
    let end = clamp(floor(u + 1.0 / 256.0) + 1.0, vec2f(0.0), last);
    return vec4i(vec2i(first), vec2i(end));
  }`);

/** What lies under fragment `clip` of the water's surface point `world`,
 *  of normal `n` in look `look`, `straight` metres down: the world's colour
 *  before the water there, and how far down it lies. Seen through the
 *  sloped surface, the bed lies displaced along the slope by about the
 *  depth it is seen through, a look's `refraction.x` of it, over at most
 *  `.y` metres, and never past `.z` of the screen's height. The shift is
 *  dropped where the point it lands on lies in front of the water (a bank,
 *  a pier) or is seen less than the water's own pixel (`fogLayout.seenUnder`),
 *  so nothing unseen shows through and the fog's edge keeps no seam. The bed
 *  blurs a mip for every `.w` metres of water it is seen through. */
export const waterUnder = tgpu
  .fn(
    [d.vec4f, d.vec3f, d.vec3f, d.f32, d.u32],
    d.vec4f,
  )(/* wgsl */ `(clip: vec4f, world: vec3f, n: vec3f, straight: f32, look: u32) -> vec4f {
    let here = vec2i(clip.xy);
    if (terrainLayout.$.params.view.z == 1u) {
      return vec4f(textureLoad(fogLayout.$.sceneColour, here, 0).rgb, straight);
    }
    let r = terrainLayout.$.params.waterLooks[look].refraction;
    let cam = typegpuCameraLayout.$.cam;
    let screen = vec2f(cam.width, cam.height);
    let shift = vec3f(n.xy * (min(straight, r.y) * r.x), 0.0);
    let a = cam.viewProj * vec4f(world, 1.0);
    let b = cam.viewProj * vec4f(world + shift, 1.0);
    var px = (b.xy / b.w - a.xy / a.w) * vec2f(0.5, -0.5) * screen;
    px *= min(1.0, r.z * cam.height / max(length(px), 1e-6));
    let to = vec2i(clamp(clip.xy + px, vec2f(0.0), screen - 1.0));
    let ahead = cam.invViewProj * vec4f(ndcAt(to), textureLoad(fogLayout.$.depth, to, 0), 1.0);
    let behind = length(ahead.xyz / ahead.w - cam.eye) - length(world - cam.eye);
    let unseen = textureLoad(fogLayout.$.seenUnder, to, 0).x
      - textureLoad(fogLayout.$.seenUnder, here, 0).x;
    let keep = smoothstep(0.0, 0.15, behind) * (1.0 - smoothstep(0.0, 0.5, unseen));
    let at = clip.xy + px * keep;
    let far = bedDistance(vec2i(at), world);
    let level = min(far / r.w, f32(textureNumLevels(fogLayout.$.sceneColour) - 1));
    // The blurred tap averages its footprint's unfogged colours, and a
    // swamp's bed keeps its paint. Where any of that footprint is seen less
    // than the water's own pixel, or is marked, the bed is read at the
    // pixel, unshifted and unblurred: nothing unseen shows through, and the
    // bed's mark lies under the surface's, so it shows once. The guard's
    // level holds the worst of each texel's footprint (\`guardMip\`), so its
    // texels under the tap, at either level the sampler may pick, say it
    // exactly.
    let mine = textureLoad(fogLayout.$.seenUnder, here, 0).x;
    var worst = vec2f(0.0, textureLoad(fogLayout.$.paint, here, 0).a);
    for (var m = u32(floor(level)); m <= u32(ceil(level)); m++) {
      let size = vec2f(textureDimensions(fogLayout.$.seenUnder, m));
      let taps = tapped(at / screen * size - 0.5, size);
      for (var y = taps.y; y <= taps.w; y++) {
        for (var x = taps.x; x <= taps.z; x++) {
          worst = max(worst, textureLoad(fogLayout.$.seenUnder, vec2i(x, y), m).xy);
        }
      }
    }
    if (worst.x > mine || worst.y > 0.0) {
      return vec4f(textureLoad(fogLayout.$.sceneColour, here, 0).rgb, straight);
    }
    let colour = textureSampleLevel(fogLayout.$.sceneColour, fogLayout.$.sceneTaps, at / screen, level);
    return vec4f(colour.rgb, far);
  }`)
  .$uses({ bedDistance, fogLayout, ndcAt, tapped, terrainLayout, typegpuCameraLayout });

/** A mip of the world's copy from the one before it (`source`). */
const sceneMipLayout = tgpu.bindGroupLayout({
  source: { texture: d.texture2d(d.f32), visibility: ["fragment"] },
  taps: { sampler: "filtering", visibility: ["fragment"] },
});

/** Each texel the mean of the four it covers in the level before: a
 *  bilinear tap at their shared corner. */
const sceneMip = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pos: vec2f) -> vec4f {
    let size = max(vec2u(1u), textureDimensions(sceneMipLayout.$.source) / 2u);
    return textureSampleLevel(sceneMipLayout.$.source, sceneMipLayout.$.taps, pos / vec2f(size), 0.0);
  }`)
  .$uses({ sceneMipLayout });

/** The guard's first level (\`FrameTargets.fogMaskUnder\`) from the fog mask
 *  and the ground paint before the water. */
const guardSeedLayout = tgpu.bindGroupLayout({
  mask: { texture: d.texture2d(d.f32), visibility: ["fragment"] },
  paint: { texture: d.texture2d(d.f32), visibility: ["fragment"] },
});

/** How unseen pixel \`pos\` is, and how much of it the paint covers. */
const guardSeed = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pos: vec2f) -> vec4f {
    let p = vec2i(pos);
    let unseen = textureLoad(guardSeedLayout.$.mask, p, 0).x;
    return vec4f(unseen, textureLoad(guardSeedLayout.$.paint, p, 0).a, 0.0, 0.0);
  }`)
  .$uses({ guardSeedLayout });

/** A guard level from the one before it (\`source\`). */
const guardMipLayout = tgpu.bindGroupLayout({
  source: { texture: d.texture2d(d.f32), visibility: ["fragment"] },
});

/** Each texel the worst of every texel its colour mip's tap
 *  (\`sceneMip\`) may weigh in the level before. */
const guardMip = tgpu
  .fn(
    [d.vec2f],
    d.vec4f,
  )(/* wgsl */ `(pos: vec2f) -> vec4f {
    let before = vec2f(textureDimensions(guardMipLayout.$.source));
    let size = max(vec2f(1.0), floor(before / 2.0));
    let taps = tapped(pos / size * before - 0.5, before);
    var worst = vec2f(0.0);
    for (var y = taps.y; y <= taps.w; y++) {
      for (var x = taps.x; x <= taps.z; x++) {
        worst = max(worst, textureLoad(guardMipLayout.$.source, vec2i(x, y), 0).xy);
      }
    }
    return vec4f(worst, 0.0, 0.0);
  }`)
  .$uses({ guardMipLayout, tapped });

/** The pipelines that draw the world copy's mips and its guard's, and
 *  their pass. */
export function createSceneMips(root: Root) {
  const pipeline = root
    .createRenderPipeline({
      vertex: common.fullScreenTriangle,
      fragment: tgpu.fragmentFn({ in: { pos: d.builtin.position }, out: d.vec4f })((v) => {
        "use gpu";
        return sceneMip(v.pos.xy);
      }),
      targets: { format: HDR_FORMAT },
    })
    .$name("sceneColourMip");
  const seed = root
    .createRenderPipeline({
      vertex: common.fullScreenTriangle,
      fragment: tgpu.fragmentFn({ in: { pos: d.builtin.position }, out: d.vec4f })((v) => {
        "use gpu";
        return guardSeed(v.pos.xy);
      }),
      targets: { format: GUARD_FORMAT },
    })
    .$name("waterGuardSeed");
  const guard = root
    .createRenderPipeline({
      vertex: common.fullScreenTriangle,
      fragment: tgpu.fragmentFn({ in: { pos: d.builtin.position }, out: d.vec4f })((v) => {
        "use gpu";
        return guardMip(v.pos.xy);
      }),
      targets: { format: GUARD_FORMAT },
    })
    .$name("waterGuardMip");
  const taps = root.device.createSampler({
    label: "scene-colour-mips",
    magFilter: "linear",
    minFilter: "linear",
  });
  const level = (texture: GPUTexture, mip: number) =>
    texture.createView({ baseMipLevel: mip, mipLevelCount: 1 });
  /** Each level of the copy and the guard after the first, the groups that
   *  read the one before it, and the guard's seed, by the copy. */
  const levelsOf = ({ sceneColour, fogMaskUnder, fogMask, paint }: FrameTargets) => ({
    colour: Array.from({ length: sceneColour.mipLevelCount - 1 }, (_, i) => ({
      view: level(sceneColour, i + 1),
      group: root.createBindGroup(sceneMipLayout, { source: level(sceneColour, i), taps }),
    })),
    seed: fogMaskUnder
      ? {
          view: level(fogMaskUnder, 0),
          group: root.createBindGroup(guardSeedLayout, {
            mask: fogMask.createView(),
            paint: paint.createView(),
          }),
        }
      : null,
    guard: fogMaskUnder
      ? Array.from({ length: fogMaskUnder.mipLevelCount - 1 }, (_, i) => ({
          view: level(fogMaskUnder, i + 1),
          group: root.createBindGroup(guardMipLayout, { source: level(fogMaskUnder, i) }),
        }))
      : [],
  });
  const levels = new WeakMap<GPUTexture, ReturnType<typeof levelsOf>>();
  return {
    pipelines: [pipeline, seed, guard],
    /** Each mip of \`targets.sceneColour\` from the one before, and the
     *  guard (\`targets.fogMaskUnder\`) from the fog mask and the paint. */
    encode(encoder: TgpuCommandEncoder, targets: FrameTargets) {
      let mips = levels.get(targets.sceneColour);
      if (!mips) levels.set(targets.sceneColour, (mips = levelsOf(targets)));
      const target = (view: GPUTextureView) =>
        ({ view, loadOp: "clear", storeOp: "store" }) as const;
      for (const { view, group } of mips.colour)
        pipeline.with(encoder).with(group).withColorAttachment(target(view)).draw(3);
      if (mips.seed)
        seed
          .with(encoder)
          .with(mips.seed.group)
          .withColorAttachment(target(mips.seed.view))
          .draw(3);
      for (const { view, group } of mips.guard)
        guard.with(encoder).with(group).withColorAttachment(target(view)).draw(3);
    },
  };
}
