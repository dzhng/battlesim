// The water: its surface over the rivers' cells (the terrain material's
// `waterSurface`), blended over the opaque world, reading depth and writing
// none. `worldPass.ts` draws it, in the order `encode` gives.
import { tgpu, d, std } from "typegpu";
import game from "@fixtures/game.json";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { EnvironmentFrame } from "./environmentFrame";
import { meshAttribs, meshVertex, WORLD_VARYING } from "./geometry";
import {
  fogCoverage,
  fogIsGround,
  fogTerm,
  groundPaint,
  paintedAlbedo,
  paintedSeen,
  paintGlow,
} from "./fogTerm";
import { waterNormal, waterSurface } from "./terrainMaterial";
import { FRAME_MSAA, WORLD_OUT, worldTargets } from "./targets";
const terrainTuning = game.presentation.terrain;

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The water's pipeline, lit by `environment`. */
export function createWaterPipeline(root: Root, environment: EnvironmentFrame) {
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
  return root
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
}
