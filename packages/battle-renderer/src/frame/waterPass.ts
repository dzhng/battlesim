// The water: its surface over the standing water's cells (rivers, swamps and
// puddles: `ground::water::StandingWater`), shaped by `waterSurface.ts`,
// blended by its edge over the world and its glass, reading depth and writing
// none, in a pass of its own. `worldPass.ts` draws it, in the order `encode`
// gives.
import { tgpu, d, std, type TgpuBindGroup, type TgpuCommandEncoder } from "typegpu";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { EnvironmentFrame } from "./environmentFrame";
import { type CameraGroup, meshAttribs, meshVertex, WORLD_VARYING } from "./geometry";
import type { SpatialMeshDraws } from "./spatialMeshDraws";
import type { SubmittedDraws } from "./submitted";
import {
  fogCoverage,
  fogIsGround,
  fogTerm,
  groundPaint,
  paintedAlbedo,
  paintedSeen,
  paintGlow,
} from "./fogTerm";
import { terrainLayout } from "./terrainLayout";
import { waterCaustics } from "./waterCaustics";
import { createWaterLight } from "./waterLight";
import { waterNormal } from "./waterSurface";
import { waveNormal } from "./waveNormal";
import { createSceneMips, ndcAt, waterDepth, waterUnder } from "./waterUnder";
import { FRAME_MSAA, WORLD_OUT, worldTargets, type FrameTargets } from "./targets";

/** Rec. 709's weights: a linear colour's luminance is its dot with them. */
const LUMA = d.vec3f(0.2126, 0.7152, 0.0722);

/** The world point the pixel's centre shows at fragment `clip`'s depth. Its
 *  derivatives hold across the water's triangles, where the centroid
 *  `world` varying's step at a partly covered pixel. */
const atPixel = tgpu
  .fn(
    [d.vec4f],
    d.vec3f,
  )(/* wgsl */ `(clip: vec4f) -> vec3f {
    let p = typegpuCameraLayout.$.cam.invViewProj * vec4f(ndcAt(vec2i(clip.xy)), clip.z, 1.0);
    return p.xyz / p.w;
  }`)
  .$uses({ ndcAt, typegpuCameraLayout });

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The water's pipeline, lit by `environment`, and its pass. */
export function createWater(root: Root, environment: EnvironmentFrame) {
  const varyings = {
    clip: d.builtin.position,
    world: WORLD_VARYING,
    normal: d.vec3f,
    color: d.vec4f,
    highlight: d.f32,
  };
  const { waterReflection, waterTransmission } = createWaterLight(environment);
  /** The water: the bed under it (the world before the water, seen
   *  through the surface, `waterUnder`) dimmed per channel by the light the
   *  water takes over the distance down to it (`exp(-e·d)`, as RedGPU's
   *  extinction, `lakeFragment.wgsl:415-433`, with each look's own `e`),
   *  its body's colour, lit, in place of what it takes, and the sky's and
   *  the sun's reflection over both (`waterLight.ts`), with the air between
   *  it and the eye once. What comes up from under the surface loses what
   *  the surface reflects. The body turns from the look's edge colour to its
   *  channel colour as less light survives. It takes the ground paint as the ground does (its layer
   *  is painted): a mark lies over the water by its coverage, once, the bed
   *  under it unpainted. */
  const waterFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const footprint = std.length(std.fwidth(v.world.xy));
    // The cell's look rides its vertex colour (`mesh::water`).
    const look = d.u32(std.round(v.color.x));
    const w = terrainLayout.$.params.waterLooks[look];
    // A puddle stands still: no wind ripples it, so the sky lies in it
    // whole; a wake still does (decision 29).
    const still = std.select(
      waterNormal(v.world.xy, footprint, look),
      d.vec3f(0, 0, 1),
      v.color.w > 0,
    );
    const n = waveNormal(v.world.xy, still);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const paint = groundPaint(v.world);
    // A puddle carves nothing, so its depth is its shape's (`mesh::water`
    // carries its centre and radius): deepest at its middle, nothing at
    // its rim, along the view. Other water's is the ground's.
    const puddle = v.color.w > 0;
    const middle = std.length(std.sub(v.world.xy, v.color.yz)) / std.max(v.color.w, 1e-4);
    const up = std.max(std.normalize(std.sub(eye, v.world)).z, 0.2);
    const shaped = (w.fades.w * std.max(1 - middle * middle, 0)) / up;
    const depth = std.select(waterDepth(v.clip, v.world), shaped, puddle);
    const bed = waterUnder(v.clip, v.world, n, depth, look);
    const through = std.exp(std.mul(w.surface.xyz, -std.select(bed.w, depth, puddle)));
    // The waves' light focused on the bed where the sun reaches the water.
    const lit = std.mul(bed.xyz, 1 + waterCaustics(v.world, depth, footprint, look) * sun);
    const body = std.mix(w.edge.xyz, w.channel.xyz, 1 - std.dot(through, LUMA));
    // The surface's light before the air: its reflection; the body's own
    // light in place of what the water takes (a lit surface's less its
    // reflection, as a dielectric's); a mark wholly covering where the paint
    // lies.
    const reflected = waterReflection(n, v.world, sun, eye, look);
    const dry = environment.surfaceLight(
      d.vec3f(0),
      d.vec3f(0),
      w.roughness.x,
      0,
      0,
      1,
      n,
      v.world,
      sun,
      eye,
    );
    const own = std.sub(
      environment.surfaceLight(body, d.vec3f(0), w.roughness.x, 0, 0, 1, n, v.world, sun, eye),
      dry,
    );
    const mark = environment.surfaceLight(
      paintedAlbedo(body, std.div(paint, std.max(paint.w, 1e-4))),
      d.vec3f(0),
      w.roughness.x,
      0,
      0,
      1,
      n,
      v.world,
      sun,
      eye,
    );
    const water = std.add(std.mul(own, std.sub(d.vec3f(1), through)), reflected);
    const seenUp = std.mul(through, waterTransmission(n, std.normalize(std.sub(eye, v.world))));
    const shadowed = std.mix(w.surface.w, 1, sun);
    // The air over it once: the bed's copy already lies behind its own air,
    // so the bed's share of the air is taken off again.
    const surfaceInAir = environment.air(std.mul(water, shadowed), v.world, eye).xyz;
    const bedAir = std.mul(environment.air(d.vec3f(0), v.world, eye).xyz, seenUp);
    const wet = std.add(std.sub(surfaceInAir, bedAir), std.mul(lit, seenUp));
    // The edge by depth (RedGPU `lakeFragment.wgsl:373-375`): the water fades
    // in over its look's shore depth, or over a few pixels where that is
    // narrower, wider as the surface is seen edge-on, so it never aliases.
    const facing = std.max(0.35, std.dot(n, std.normalize(std.sub(eye, v.world))));
    const pixel = std.length(std.fwidth(atPixel(v.clip).xy));
    const fade = std.max(w.fades.x, (pixel * 3) / facing);
    // A puddle, far shallower than a few pixels, fades in over its rim
    // instead: a sixth of its radius, or a pixel and a half if wider.
    const rim = (1 - middle) * v.color.w;
    const ring = std.max(v.color.w / 6, pixel * 1.5);
    const fill = std.select(depth / fade, rim / ring, puddle);
    const edge = std.smoothstep(0, 1, std.clamp(fill, 0, 1));
    // The mark lies over the water by its own coverage, whole at the bank
    // too, where the water fades and the bed under it is unpainted.
    const marked = std.mul(environment.air(std.mul(mark, shadowed), v.world, eye).xyz, paint.w);
    const cover = paint.w + edge * (1 - paint.w);
    const colour = std.div(
      std.add(std.add(marked, paintGlow(paint)), std.mul(wet, edge * (1 - paint.w))),
      std.max(cover, 1e-4),
    );
    const seen = paintedSeen(fogTerm(v.world, n, v.clip.xy, fogIsGround()), paint);
    return { color: d.vec4f(colour, cover), fog: fogCoverage(seen, cover) };
  });
  const pipeline = root
    .createRenderPipeline({
      attribs: meshAttribs,
      vertex: meshVertex,
      primitive: { topology: "triangle-list", cullMode: "none" },
      fragment: waterFragment,
      // The fog mask blends as the colour does: water over unseen ground is as
      // unseen as its alpha lets the ground through.
      targets: worldTargets({
        color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
        alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
      }),
      depthStencil: battleWorldDepth("read"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("worldWater");
  const mips = createSceneMips(root);
  return {
    /** What it compiles: the water's, and its bed's mips'. */
    pipelines: [pipeline, ...mips.pipelines],
    /** The water's pass, over the samples the world pass kept (it stores
     *  them), resolving the lit world and the fog mask again; the `mesh`
     *  drawn with the camera's, the water's fog group (`FogGroups.water`)
     *  and the terrain's `groups`. The
     *  world pass resolves too: Metal skips the resolve of a tile this pass
     *  leaves empty. Storing the samples cost less than run-to-run noise
     *  against single-sampled water (decision 25). Water behind a pane draws
     *  over it: the cost of keeping glass in the world pass. */
    encode(
      encoder: TgpuCommandEncoder,
      targets: FrameTargets,
      mesh: SpatialMeshDraws,
      [camera, fog, terrain]: readonly [CameraGroup, TgpuBindGroup, TgpuBindGroup],
      submitted: SubmittedDraws,
    ) {
      // The world as it stands is what the water shows of its bed, and,
      // where it refracts, how seen each pixel of it is.
      const size = [targets.width, targets.height];
      const raw = root.unwrap(encoder);
      raw.copyTextureToTexture({ texture: targets.lit }, { texture: targets.sceneColour }, size);
      if (targets.fogMaskUnder) mips.encode(encoder, targets);
      const pass = encoder.beginRenderPass({
        label: "water",
        colorAttachments: [
          {
            view: targets.hdrMsaa.createView(),
            resolveTarget: targets.lit.createView(),
            loadOp: "load",
            storeOp: "discard",
          },
          {
            view: targets.fogMaskMsaa.createView(),
            resolveTarget: targets.fogMask.createView(),
            loadOp: "load",
            storeOp: "discard",
          },
        ],
        depthStencilAttachment: { view: targets.depth.createView(), depthReadOnly: true },
      });
      mesh.draw(
        pipeline.with(pass).with(camera).with(environment.group).with(fog).with(terrain),
        submitted,
      );
      pass.end();
    },
  };
}
