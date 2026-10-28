// What the water shows under it: the world before the water
// (`FrameTargets.sceneColour`) and how far down it lies, seen straight
// through the surface, or displaced by its slope (refraction, RedGPU
// `lakeFragment.wgsl:380-413`). The tier sets which (`WaterDetail`): Low
// keeps the depth colour alone (decision 18), so it reads the copy at the
// pixel and builds no mips and no fog mask copy.
import { tgpu, d, common, type TgpuCommandEncoder } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";
import { fogLayout } from "./fogTerm";
import { FRAME_MSAA, HDR_FORMAT, type FrameTargets } from "./targets";
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
    let colour = textureSampleLevel(fogLayout.$.sceneColour, fogLayout.$.sceneTaps, at / screen, level);
    return vec4f(colour.rgb, far);
  }`)
  .$uses({ bedDistance, fogLayout, ndcAt, terrainLayout, typegpuCameraLayout });

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

/** The pipeline that draws the world copy's mips, and its pass. */
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
  const taps = root.device.createSampler({
    label: "scene-colour-mips",
    magFilter: "linear",
    minFilter: "linear",
  });
  /** Each mip after the first, and the group that reads the one before
   *  it, by the copy they belong to. */
  const levels = new WeakMap<GPUTexture, ReturnType<typeof levelsOf>>();
  const level = (texture: GPUTexture, mip: number) =>
    texture.createView({ baseMipLevel: mip, mipLevelCount: 1 });
  const levelsOf = (texture: GPUTexture) =>
    Array.from({ length: texture.mipLevelCount - 1 }, (_, i) => ({
      view: level(texture, i + 1),
      group: root.createBindGroup(sceneMipLayout, { source: level(texture, i), taps }),
    }));
  return {
    pipeline,
    /** Each mip of `targets.sceneColour` from the one before. */
    encode(encoder: TgpuCommandEncoder, { sceneColour }: FrameTargets) {
      let mips = levels.get(sceneColour);
      if (!mips) levels.set(sceneColour, (mips = levelsOf(sceneColour)));
      for (const { view, group } of mips)
        pipeline
          .with(encoder)
          .with(group)
          .withColorAttachment({ view, loadOp: "clear", storeOp: "store" })
          .draw(3);
    },
  };
}
