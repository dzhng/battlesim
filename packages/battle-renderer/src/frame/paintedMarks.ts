// Painted ground marks (27e follow-ups, the user's "literally on the ground
// and glowing a bit"): orders, unit and area circles, cover pips, travel
// chevrons, supply and suppression rings, the objective zone and the map's
// border, drawn in the HDR world pass as ground decals, not in the overlay.
// So they are:
// - lit like ground paint: the sun's shadow falls on them (`sampleSunShadow`);
// - fogged like the ground under them, though only `fog_keep` of the way, so
//   a mark in unseen ground still reads through the fog style;
// - under every effect: smoke, dust, fire and flashes draw over them;
// - hidden by what stands in front, depth-tested against the world's depth;
// - glowing by an emissive term the world's bloom carries (`emissive`),
//   which is what shines through smoke and shadow.
//
// Grass blades would speckle a mark lying under them (slice 27), so each
// mark is drawn pulled toward the eye along its own view ray by the grass's
// reach: the pixel it lands on is unchanged, and its depth clears the blades
// but not a hull, a wall or a ridge in front. Its light and fog are taken
// where it lies. Blended over the lit world, colour and fog mask alike; it
// never writes depth.
import { tgpu, d, std } from "typegpu";
import type { Mesh } from "../mesh";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import type { EnvironmentFrame } from "./environmentFrame";
import { fogCoverage, fogIsGround, fogTerm } from "./fogTerm";
import {
  identityInstance,
  meshAttribs,
  MeshSlot,
  WORLD_VARYING,
  type CameraGroup,
} from "./geometry";
import { FRAME_MSAA, WORLD_OUT, worldTargets } from "./targets";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** How painted ground marks look (`presentation.overlay`: `glow.ground` is
 *  `emissive`, `paint` the rest). */
export interface PaintStyle {
  /** The paint's reflectance, as a share of its colour (linear): paint is
   *  ground-dark, not a lamp, so shade shows on it. */
  albedo: number;
  /** The glow: the mark's own colour, linear, times this, added to its lit
   *  paint. */
  emissive: number;
  /** How much of the fog a mark in unseen ground takes, 0 none, 1 the
   *  ground's. */
  fog_keep: number;
  /** How far toward the eye a mark is drawn, measured up from the ground: a
   *  grass blade's height, so the blades never speckle it. */
  grass_reach_m: number;
}

export function validatePaintStyle(style: PaintStyle): PaintStyle {
  if (
    !(style.albedo > 0 && style.albedo <= 1) ||
    !(style.emissive >= 0 && style.emissive <= 8) ||
    !(style.fog_keep >= 0 && style.fog_keep <= 1) ||
    !(style.grass_reach_m >= 0 && style.grass_reach_m <= 4)
  )
    throw new Error(
      `painted marks: albedo in (0, 1], emissive in [0, 8], fog_keep in [0, 1], grass_reach_m in [0, 4], got ${JSON.stringify(style)}`,
    );
  return style;
}

const PaintUniform = d
  .struct({ albedo: d.f32, emissive: d.f32, fogKeep: d.f32, reach: d.f32 })
  .$name("PaintStyle");
const paintLayout = tgpu.bindGroupLayout({
  style: { uniform: PaintUniform, visibility: ["vertex", "fragment"] },
});

/** A mark's world position drawn toward the eye past the grass (`reach`
 *  metres of it, measured up): along its view ray, so it lands on the same
 *  pixel. */
const pulledClip = tgpu.fn(
  [d.vec3f],
  d.vec4f,
)((world) => {
  "use gpu";
  const toEye = std.normalize(std.sub(typegpuCameraLayout.$.cam.eye, world));
  const reach = paintLayout.$.style.reach;
  // Held to twice the reach: at a low camera a longer pull would carry the
  // far arc of a vehicle's marker onto its own skirt and wheels (the
  // blades may fleck a mark seen that low).
  const pull = std.min(reach / std.max(toEye.z, 0.1), reach * 2);
  const drawn = std.add(world, std.mul(toEye, pull));
  return std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(drawn, 1));
});

const vertexIn = {
  position: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  placement: d.vec4f,
  tint: d.vec4f,
};
const vertexOut = {
  clip: d.builtin.position,
  world: WORLD_VARYING,
  march: d.vec3f,
  color: d.vec4f,
};

/** A still mark: no march. */
const paintVertex = tgpu.vertexFn({ in: vertexIn, out: vertexOut })((v) => {
  "use gpu";
  return { clip: pulledClip(v.position), world: v.position, march: d.vec3f(0), color: v.color };
});
/** A marching mark: its normal carries (phase in cycles, cycles a second,
 *  amplitude) (`orderOverlay.ts` `travelChevrons`). */
const paintMarchVertex = tgpu.vertexFn({ in: vertexIn, out: vertexOut })((v) => {
  "use gpu";
  return { clip: pulledClip(v.position), world: v.position, march: v.normal, color: v.color };
});

export function createPaintedMarks(
  root: Root,
  registry: GpuRegistry,
  environment: EnvironmentFrame,
  initial: PaintStyle,
) {
  const fragment = tgpu.fragmentFn({
    in: { clip: d.builtin.position, world: WORLD_VARYING, march: d.vec3f, color: d.vec4f },
    out: WORLD_OUT,
  })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const style = paintLayout.$.style;
    const up = d.vec3f(0, 0, 1);
    const colour = std.pow(std.max(v.color.xyz, d.vec3f(0)), d.vec3f(2.2));
    const sun = environment.sampleSunShadow(v.world, up, v.clip.xy);
    const paint = std.mul(colour, style.albedo);
    const lit = environment.shade(paint, d.vec3f(0), 0.85, 0, 0, 1, up, v.world, sun, eye);
    const glow = std.mul(colour, style.emissive);
    // The march: peaks when `time · cycles − phase` is whole, dims by up to
    // `amplitude` between (a still mark's march is zero: never dimmed).
    const beat = std.fract(typegpuCameraLayout.$.cam.time * v.march.y - v.march.x);
    const pulse = 0.5 + 0.5 * std.cos(beat * 6.2831853);
    const alpha = v.color.w * (1 - v.march.z + v.march.z * pulse);
    const seen = fogTerm(v.world, up, v.clip.xy, fogIsGround());
    const kept = std.mix(1, seen, style.fogKeep);
    return { color: d.vec4f(std.add(lit.xyz, glow), alpha), fog: fogCoverage(kept, alpha) };
  });
  const base = {
    attribs: meshAttribs,
    fragment,
    primitive: { topology: "triangle-list", cullMode: "none" },
    targets: worldTargets({
      color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
      alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
    }),
    depthStencil: battleWorldDepth("read"),
    multisample: { count: FRAME_MSAA },
  } as const;
  const still = root.createRenderPipeline({ ...base, vertex: paintVertex });
  const marching = root.createRenderPipeline({ ...base, vertex: paintMarchVertex });
  const identity = identityInstance(root, registry);
  const meshes = {
    still: new MeshSlot(root, registry, identity),
    marching: new MeshSlot(root, registry, identity),
  };
  const uniform = registry.own(root.createBuffer(PaintUniform).$usage("uniform"));
  const group = root.createBindGroup(paintLayout, { style: uniform });
  let style = validatePaintStyle(initial);
  let shown = true;
  const write = () =>
    uniform.write({
      albedo: style.albedo,
      emissive: style.emissive,
      fogKeep: style.fog_keep,
      reach: style.grass_reach_m,
    });
  write();

  return {
    ready: () => Promise.all([still.initAsync(), marching.initAsync()]),
    set(painted: Mesh, marchingMarks: Mesh) {
      meshes.still.set(painted);
      meshes.marching.set(marchingMarks);
    },
    setStyle(next: PaintStyle) {
      style = validatePaintStyle(next);
      write();
    },
    /** Lab diagnostics: draw none while off (paired frames isolate them). */
    setShown(on: boolean) {
      shown = on;
    },
    /** Over the lit world in `pass`, the ground's fog group bound. */
    encode(pass: unknown, cameraGroup: CameraGroup, fogGround: unknown) {
      if (!shown) return;
      for (const [pipeline, mesh] of [
        [still, meshes.still],
        [marching, meshes.marching],
      ] as const)
        mesh.draw(
          pipeline
            .with(pass as never)
            .with(cameraGroup)
            .with(environment.group)
            .with(fogGround as never)
            .with(group) as never,
        );
    },
    stats: () => ({ ...style, shown, vertices: meshes.still.vertices + meshes.marching.vertices }),
  };
}
export type PaintedMarks = ReturnType<typeof createPaintedMarks>;
