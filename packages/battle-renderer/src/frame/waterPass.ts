// The water: its surface over the rivers' cells (the terrain material's
// `waterSurface`), blended over the world and its glass, reading depth and
// writing none, in a pass of its own. `worldPass.ts` draws it, in the order
// `encode` gives.
import { tgpu, d, std, type TgpuBindGroup, type TgpuCommandEncoder } from "typegpu";
import game from "@fixtures/game.json";
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
import { waterNormal, waterSurface } from "./waterSurface";
import { FRAME_MSAA, WORLD_OUT, worldTargets, type FrameTargets } from "./targets";
const terrainTuning = game.presentation.terrain;

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
  /** Ripples that break the sky's and sun's reflection, the bed through it
   *  at the shore. It takes the ground paint as the ground does (its layer
   *  is painted): a mark over the water shows on it, not drowned under it. */
  const waterFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const footprint = std.length(std.fwidth(v.world.xy));
    const surface = waterSurface(v.world.xy, footprint);
    const n = waterNormal(v.world.xy, footprint);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const paint = groundPaint(v.world);
    const lit = environment.shade(
      paintedAlbedo(surface.xyz, paint),
      d.vec3f(0),
      terrainTuning.water.roughness,
      0,
      0,
      1,
      n,
      v.world,
      sun,
      eye,
    );
    const shaded = std.add(
      std.mul(lit.xyz, std.mix(terrainTuning.water.shadow, 1, sun)),
      paintGlow(paint),
    );
    const seen = paintedSeen(fogTerm(v.world, n, v.clip.xy, fogIsGround()), paint);
    const alpha = std.max(surface.w, paint.w);
    return { color: d.vec4f(shaded, alpha), fog: fogCoverage(seen, alpha) };
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
  return {
    pipeline,
    /** The water's pass, over the samples the world pass kept (it stores
     *  them), resolving the lit world and the fog mask again; the `mesh`
     *  drawn with the camera's, the fog's and the terrain's `groups`. The
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
