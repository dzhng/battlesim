// Display-space overlays: orders, contacts, tracers, rings and marks. They draw
// after post, depth-tested against the world without its grass (so a route
// lies over the blades, never speckled by them), into their own target (the
// units' x-ray already in it, over a transparent clear), and composite
// premultiplied over post's output. So they are
// never fogged, graded or tone mapped: their colours are the values their
// builders chose, shaded exactly as the old one-shader frame shaded them
// (landmine 13).
import { tgpu, d, std, common } from "typegpu";
import type { WorldMeshes } from "../scene";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth } from "../worldDepth";
import {
  identityInstance,
  meshAttribs,
  meshVertex,
  MeshSlot,
  WORLD_VARYING,
  type CameraGroup,
} from "./geometry";
import { FRAME_MSAA, OVERLAY_FORMAT, type FrameTargets } from "./targets";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** The flat light overlays have always had: one fixed key and an ambient. */
const OVERLAY_SUN = [0.5, -0.55, 0.67] as const;

const overlayFragment = tgpu.fragmentFn({
  in: { world: WORLD_VARYING, normal: d.vec3f, color: d.vec4f, highlight: d.f32 },
  out: d.vec4f,
})((v) => {
  "use gpu";
  const toEye = std.sub(typegpuCameraLayout.$.cam.eye, v.world);
  let n = std.normalize(v.normal);
  if (std.dot(n, toEye) < 0) {
    n = std.neg(n);
  }
  const key = std.normalize(d.vec3f(OVERLAY_SUN[0], OVERLAY_SUN[1], OVERLAY_SUN[2]));
  const light = std.max(std.dot(n, key), 0);
  const shaded = std.mul(v.color.xyz, 0.3 + 0.75 * light);
  const lit = std.add(shaded, std.mul(d.vec3f(0.95, 0.8, 0.2), v.highlight * 0.7));
  return d.vec4f(lit, v.color.w);
});

const overlaySource = tgpu.bindGroupLayout({
  overlay: { texture: d.texture2d(), visibility: ["fragment"] },
});

/** One overlay texel, already premultiplied by its resolve. */
const compositeFragment = tgpu.fragmentFn({
  in: { pos: d.builtin.position },
  out: d.vec4f,
})((v) => {
  "use gpu";
  return std.textureLoad(overlaySource.$.overlay, d.vec2i(v.pos.xy), 0);
});

// Drawn over a transparent clear, blending leaves colour premultiplied by
// alpha, which is what the resolve averages and the composite expects.
const premultiplying = {
  color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
  alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
} as const;
const over = {
  color: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
  alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
} as const;

export async function createOverlayPass(
  root: Root,
  registry: GpuRegistry,
  displayFormat: GPUTextureFormat,
) {
  const base = {
    attribs: meshAttribs,
    vertex: meshVertex,
    fragment: overlayFragment,
    primitive: { topology: "triangle-list", cullMode: "none" },
    targets: { format: OVERLAY_FORMAT, blend: premultiplying },
    multisample: { count: FRAME_MSAA },
  } as const;
  const opaque = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read-write"),
  });
  const translucent = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read"),
  });
  const composite = root.createRenderPipeline({
    vertex: common.fullScreenTriangle,
    fragment: compositeFragment,
    targets: { format: displayFormat, blend: over },
  });
  await Promise.all([opaque.initAsync(), translucent.initAsync(), composite.initAsync()]);

  const identity = identityInstance(root, registry);
  const meshes = {
    opaque: new MeshSlot(root, registry, identity),
    translucent: new MeshSlot(root, registry, identity),
  };

  return {
    set(next: WorldMeshes) {
      meshes.opaque.set(next.opaque);
      meshes.translucent.set(next.translucent);
    },
    /** The bind group that lets the composite read these targets' overlay. */
    sourceFor(targets: FrameTargets) {
      return root.createBindGroup(overlaySource, { overlay: targets.overlay.createView() });
    },
    /** Draw the overlays against the world's depth, over the x-ray the
     *  prepass left in the target, then lay them over `output`. */
    encode(
      encoder: GPUCommandEncoder,
      targets: FrameTargets,
      source: ReturnType<typeof root.createBindGroup>,
      cameraGroup: CameraGroup,
      output: GPUTextureView,
    ) {
      const pass = encoder.beginRenderPass({
        label: "overlay",
        colorAttachments: [
          {
            view: targets.overlayMsaa.createView(),
            resolveTarget: targets.overlay.createView(),
            loadOp: "load",
            storeOp: "discard",
          },
        ],
        depthStencilAttachment: {
          view: targets.overlayDepth.createView(),
          depthLoadOp: "load",
          depthStoreOp: "discard",
        },
      });
      meshes.opaque.draw(opaque.with(pass).with(cameraGroup));
      meshes.translucent.draw(translucent.with(pass).with(cameraGroup));
      pass.end();
      composite
        .with(encoder)
        .with(source)
        .withColorAttachment({ view: output, loadOp: "load", storeOp: "store" })
        .draw(3);
    },
  };
}
export type OverlayPass = Awaited<ReturnType<typeof createOverlayPass>>;
