// Ground paint (the user's "literally on the ground and glowing a bit", then
// "it should just paint over the grass"): the order marks whose colour role
// the fixture's scheme puts in the world layer (`resolveOrderScheme`; under
// `yellow-orders`, the selection and a selected squad's soldiers), a blocked
// route, travel chevrons, supply, suppression and impact rings, the objective
// zone and the map's border are paint on the ground, like spray paint on a
// field. Not a decal over the world, and not overlay.
//
// Each frame the marks are drawn from the camera into the frame's `paint`
// target, at the ground, depth-tested against the prepass's ground-only half
// (terrain, props, backdrop and trees; before any unit or grass), so a mark
// lands on the pixel of the ground surface it lies on and nowhere a ridge,
// wall or trunk stands in front. The painted ground layers (terrain, its
// grass, the backdrop, the water, a prop movers stand on such as a bridge
// deck: `FogLayer.painted`, the one place that says which) read it at their
// own pixel (a blade at its root's), and take its colour
// as their albedo and its emissive as their light, lit and shadowed as their
// own surface, fogged as ground (taking only `fog_keep` of it). So:
// - a blade in a stroke is painted, never speckling it;
// - a hull, a wall or a soldier is never painted: it doesn't read the paint;
// - smoke and every effect draw over it;
// - it costs one screen-size target and one small pass.
import { tgpu, d, std, type TgpuCommandEncoder } from "typegpu";
import type { Mesh } from "../mesh";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import { PAINT_RANGE } from "./fogTerm";
import { identityInstance, meshAttribs, MeshSlot, type CameraGroup } from "./geometry";
import { FRAME_MSAA, OVERLAY_FORMAT, type FrameTargets } from "./targets";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** How ground paint looks (`presentation.overlay`: `glow.ground` is
 *  `emissive`, `paint` the rest). */
export interface PaintStyle {
  /** The paint's reflectance, as a share of its colour (linear): paint is
   *  ground-dark, not a lamp, so shade shows on it. */
  albedo: number;
  /** The glow: the paint's colour, linear, times this, added to the lit
   *  surface it lies on. */
  emissive: number;
  /** How much of the fog painted ground takes, 0 none, 1 the ground's. */
  fog_keep: number;
  /** The paint's light up the grass over it: its strength at a blade's
   *  root, and the height it falls off over. */
  grass_glow: number;
  grass_falloff_m: number;
  /** How far the paint's colour is pushed from its grey (1 as given): the
   *  tone mapper and grade pull a bright hue toward white. */
  saturation: number;
}

export function validatePaintStyle(style: PaintStyle): PaintStyle {
  if (
    !(style.albedo > 0 && style.albedo <= 1) ||
    !(style.emissive >= 0 && style.emissive <= 8) ||
    !(style.fog_keep >= 0 && style.fog_keep <= 1) ||
    !(style.grass_glow >= 0 && style.grass_glow <= 8) ||
    !(style.grass_falloff_m > 0 && style.grass_falloff_m <= 4) ||
    !(style.saturation >= 0 && style.saturation <= 3)
  )
    throw new Error(
      `ground paint: albedo in (0, 1], emissive in [0, 8], fog_keep in [0, 1], grass_glow in [0, 8], grass_falloff_m in (0, 4], saturation in [0, 3], got ${JSON.stringify(style)}`,
    );
  return style;
}

const vertexIn = {
  position: d.vec3f,
  normal: d.vec3f,
  color: d.vec4f,
  placement: d.vec4f,
  tint: d.vec4f,
};
const vertexOut = { clip: d.builtin.position, march: d.vec3f, color: d.vec4f };

/** How far toward the eye, along its own view ray, a mark on the ground is
 *  drawn: its pixel is unchanged, and its depth clears the ground it lies on
 *  (the ground-only depth), though not a ridge or a wall in front. The one
 *  owner of how paint clears the ground: the marks lie on it, with no lift. */
const PULL_M = 1;

/** A mark vertex, on the ground, drawn `PULL_M` toward the eye along its
 *  view ray. */
const clipOf = (position: d.v3f) => {
  "use gpu";
  const toEye = std.normalize(std.sub(typegpuCameraLayout.$.cam.eye, position));
  const drawn = std.add(position, std.mul(toEye, PULL_M));
  return std.mul(typegpuCameraLayout.$.cam.viewProj, d.vec4f(drawn, 1));
};

/** A still mark: no march. */
const paintVertex = tgpu.vertexFn({ in: vertexIn, out: vertexOut })((v) => {
  "use gpu";
  return { clip: clipOf(v.position), march: d.vec3f(0), color: v.color };
});
/** A marching mark: its normal carries (phase in cycles, cycles a second,
 *  amplitude) (`orderOverlay.ts` `travelChevrons`). */
const paintMarchVertex = tgpu.vertexFn({ in: vertexIn, out: vertexOut })((v) => {
  "use gpu";
  return { clip: clipOf(v.position), march: v.normal, color: v.color };
});

/** A mark's colour and coverage; a marching one pulses on the clock, peaking
 *  when `time · cycles − phase` is whole and dimming by up to `amplitude`
 *  between (a still mark's march is zero: never dimmed). */
const paintFragment = tgpu.fragmentFn({
  in: { march: d.vec3f, color: d.vec4f },
  out: d.vec4f,
})((v) => {
  "use gpu";
  const beat = std.fract(typegpuCameraLayout.$.cam.time * v.march.y - v.march.x);
  const pulse = 0.5 + 0.5 * std.cos(beat * 6.2831853);
  // Colour over the paint's range, so a mark can glow past its hue's
  // full value (`PAINT_RANGE`).
  return d.vec4f(
    std.mul(v.color.xyz, 1 / PAINT_RANGE),
    v.color.w * (1 - v.march.z + v.march.z * pulse),
  );
});

export function createPaintedMarks(root: Root, registry: GpuRegistry) {
  const base = {
    attribs: meshAttribs,
    fragment: paintFragment,
    primitive: { topology: "triangle-list", cullMode: "none" },
    // Premultiplied over a transparent clear, as the overlays draw.
    targets: {
      format: OVERLAY_FORMAT,
      blend: {
        color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
        alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
      },
    },
    depthStencil: battleWorldDepth("read"),
    multisample: { count: FRAME_MSAA },
  } as const;
  const still = root.createRenderPipeline({ ...base, vertex: paintVertex });
  const marching = root.createRenderPipeline({ ...base, vertex: paintMarchVertex });
  const identity = identityInstance(root, registry);
  const meshes = {
    still: new MeshSlot(root, registry, identity),
    marching: new MeshSlot(root, registry, identity),
    /** Marks that follow the pointer (the range ruler), set apart so a
     *  pointer move uploads only them. */
    pointer: new MeshSlot(root, registry, identity),
  };
  let shown = true;

  return {
    ready: () => Promise.all([still.initAsync(), marching.initAsync()]),
    set(painted: Mesh, marchingMarks: Mesh) {
      meshes.still.set(painted);
      meshes.marching.set(marchingMarks);
    },
    setPointer(marks: Mesh) {
      meshes.pointer.set(marks);
    },
    /** Lab diagnostics: paint nothing while off (paired frames isolate it). */
    setShown(on: boolean) {
      shown = on;
    },
    /** Draw the marks at the ground into `targets.paint` (through the
     *  overlay's multisampled target, which the x-ray clears after), against
     *  `depth`: the prepass's ground-only half. Always clears the paint. */
    encode(
      encoder: TgpuCommandEncoder,
      targets: FrameTargets,
      depth: GPUTextureView,
      cameraGroup: CameraGroup,
    ) {
      const pass = encoder.beginRenderPass({
        label: "ground-paint",
        colorAttachments: [
          {
            view: targets.overlayMsaa.createView(),
            resolveTarget: targets.paint.createView(),
            loadOp: "clear",
            storeOp: "discard",
            clearValue: [0, 0, 0, 0],
          },
        ],
        depthStencilAttachment: { view: depth, depthReadOnly: true },
      });
      if (shown)
        for (const [pipeline, mesh] of [
          [still, meshes.still],
          [marching, meshes.marching],
          [still, meshes.pointer],
        ] as const)
          mesh.draw(pipeline.with(pass as never).with(cameraGroup) as never);
      pass.end();
    },
    stats: () => ({
      shown,
      vertices: meshes.still.vertices + meshes.marching.vertices + meshes.pointer.vertices,
    }),
  };
}
export type PaintedMarks = ReturnType<typeof createPaintedMarks>;
