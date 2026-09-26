// The HDR world: the sun's cascades, a depth prepass, then the sky and the lit
// world into the multisampled rgba16float target. Every world material shades
// with the ported environment (PBR, PMREM, sun shadow, aerial haze) and then
// applies FogTerm, so fog lands before post like any other light.
//
// The depth prepass writes the frame's 4× MSAA depth before any colour, so
// slice 14's tile cull can read the scene's depth (sample 0) ahead of the
// colour pass (spike 02, landmine 6). The colour pass then shades each opaque
// surface at the depth the prepass left.
//
// Three kinds of world geometry, all lit, fogged, graded and shadow-casting:
// - the static world (terrain, props);
// - the proxies (units);
// - the structures layer: what the side knows stands (buildings, remembered
//   ruins and wrecks).
// Plus the backdrop past the map edge: lit and hazed like the world, but
// unfogged, unshadowed and casting nothing.
import { tgpu, d, std, type TgpuCommandEncoder } from "typegpu";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { Mesh } from "../mesh";
import type { FogField, SceneInstance, WorldMeshes } from "../scene";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth, BATTLE_DEPTH_ATTACHMENT } from "../worldDepth";
import type { SkyRays } from "../shaders/physicalSky";
import type { EnvironmentFrame } from "./environmentFrame";
import { backdropMesh } from "./backdrop";
import {
  identityInstance,
  type CameraGroup,
  meshAttribs,
  meshVertex,
  MeshSlot,
  ProxyInstances,
} from "./geometry";
import { createFogSource, fogIsGround, fogTerm, unseenLook } from "./fogTerm";
import { mapBox, type MapBox } from "./receiverRange";
import { FRAME_MSAA, HDR_FORMAT, type FrameTargets } from "./targets";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** Our vertex colours are sRGB display values; lighting wants linear albedo. */
const srgbToLinear = tgpu.fn(
  [d.vec3f],
  d.vec3f,
)((c) => {
  "use gpu";
  return std.pow(std.max(c, d.vec3f(0)), d.vec3f(2.2));
});

/** Selection glow added to a highlighted proxy. */
const HIGHLIGHT = [0.95, 0.8, 0.2] as const;
/** A rough dielectric: the flat box world has no material maps yet. */
const ROUGHNESS = 0.85;

export async function createWorldPass(
  root: Root,
  registry: GpuRegistry,
  environment: EnvironmentFrame,
) {
  const worldFragment = tgpu.fragmentFn({
    in: {
      clip: d.builtin.position,
      world: d.vec3f,
      normal: d.vec3f,
      color: d.vec4f,
      highlight: d.f32,
    },
    out: d.vec4f,
  })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    let n = std.normalize(v.normal);
    if (std.dot(n, std.sub(eye, v.world)) < 0) {
      n = std.neg(n);
    }
    const albedo = srgbToLinear(v.color.xyz);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const lit = environment.shade(albedo, d.vec3f(0), ROUGHNESS, 0, 0, 1, n, v.world, sun, eye);
    const glow = std.mul(d.vec3f(HIGHLIGHT[0], HIGHLIGHT[1], HIGHLIGHT[2]), v.highlight * 0.7);
    const seen = fogTerm(v.world, n, v.clip.xy, fogIsGround());
    return d.vec4f(unseenLook(std.add(lit.xyz, glow), seen), v.color.w);
  });
  /** The backdrop: the same light and haze, never fogged or shadowed. */
  const backdropFragment = tgpu.fragmentFn({
    in: {
      clip: d.builtin.position,
      world: d.vec3f,
      normal: d.vec3f,
      color: d.vec4f,
      highlight: d.f32,
    },
    out: d.vec4f,
  })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const albedo = srgbToLinear(v.color.xyz);
    const up = std.normalize(v.normal);
    const lit = environment.shade(albedo, d.vec3f(0), ROUGHNESS, 0, 0, 1, up, v.world, 1, eye);
    return d.vec4f(lit.xyz, 1);
  });

  const base = {
    attribs: meshAttribs,
    vertex: meshVertex,
    primitive: { topology: "triangle-list", cullMode: "none" },
  } as const;
  // Depth only, with no fragment stage, so both keep early-Z.
  const prepass = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  const caster = root.createRenderPipeline({
    ...base,
    depthStencil: battleWorldDepth("read-write"),
  });
  const opaque = root.createRenderPipeline({
    ...base,
    fragment: worldFragment,
    targets: { format: HDR_FORMAT },
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  const translucent = root.createRenderPipeline({
    ...base,
    fragment: worldFragment,
    targets: {
      format: HDR_FORMAT,
      blend: {
        color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
        alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
      },
    },
    depthStencil: battleWorldDepth("read"),
    multisample: { count: FRAME_MSAA },
  });
  const backdropPipeline = root.createRenderPipeline({
    ...base,
    fragment: backdropFragment,
    targets: { format: HDR_FORMAT },
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  await Promise.all(
    [prepass, caster, opaque, translucent, backdropPipeline].map((pipeline) =>
      pipeline.initAsync(),
    ),
  );

  const fog = createFogSource(root, registry);
  const identity = identityInstance(root, registry);
  const world = {
    opaque: new MeshSlot(root, registry, identity),
    translucent: new MeshSlot(root, registry, identity),
  };
  const structures = new MeshSlot(root, registry, identity);
  const backdrop = new MeshSlot(root, registry, identity);
  const proxies = new ProxyInstances(root, registry);
  let box: MapBox | null = null;

  return {
    setWorld(next: WorldMeshes) {
      world.opaque.set(next.opaque);
      world.translucent.set(next.translucent);
      box = mapBox(next.opaque);
      if (box) backdrop.set(backdropMesh(box, environment.light.backdrop));
    },
    setStructures(next: Mesh) {
      structures.set(next);
    },
    setInstances(next: readonly SceneInstance[]) {
      proxies.set(next);
    },
    setFog(next: FogField | null) {
      fog.set(next);
    },
    /** Pose the environment and the cascades for this frame's camera. */
    prepare(camera: Camera3DParams, view: ArrayLike<number>, rays: SkyRays) {
      environment.prepare(camera, view, rays, box);
    },
    encodeShadows(encoder: TgpuCommandEncoder) {
      environment.encodeShadows(encoder, (pass, cameraGroup) => {
        const bound = caster.with(pass).with(cameraGroup);
        world.opaque.draw(bound);
        structures.draw(bound);
        proxies.draw(bound);
      });
    },
    /** The frame's depth: every opaque layer, before any colour. */
    encodeDepth(encoder: TgpuCommandEncoder, targets: FrameTargets, cameraGroup: CameraGroup) {
      const pass = encoder.beginRenderPass({
        label: "depth-prepass",
        colorAttachments: [],
        depthStencilAttachment: {
          view: targets.depth.createView(),
          depthClearValue: BATTLE_DEPTH_ATTACHMENT.clearValue,
          depthLoadOp: "clear",
          depthStoreOp: "store",
        },
      });
      const bound = prepass.with(pass).with(cameraGroup);
      world.opaque.draw(bound);
      proxies.draw(bound);
      structures.draw(bound);
      backdrop.draw(bound);
      pass.end();
    },
    /** The sky, then the opaque world, proxies and structures at the
     *  prepass's depth, then the translucent world. */
    encode(
      encoder: TgpuCommandEncoder,
      raw: GPUCommandEncoder,
      targets: FrameTargets,
      cameraGroup: CameraGroup,
    ) {
      const colorView = targets.hdrMsaa.createView();
      environment.encodeBackground(raw, colorView);
      const pass = encoder.beginRenderPass({
        label: "world",
        colorAttachments: [
          {
            view: colorView,
            resolveTarget: targets.hdr.createView(),
            loadOp: "load",
            storeOp: "discard",
          },
        ],
        depthStencilAttachment: {
          view: targets.depth.createView(),
          depthLoadOp: "load",
          // The overlay pass depth-tests against the world.
          depthStoreOp: "store",
        },
      });
      const lit = opaque.with(pass).with(cameraGroup).with(environment.group);
      world.opaque.draw(lit.with(fog.ground));
      const faces = lit.with(fog.faces);
      proxies.draw(faces);
      structures.draw(faces);
      backdrop.draw(backdropPipeline.with(pass).with(cameraGroup).with(environment.group));
      world.translucent.draw(
        translucent.with(pass).with(cameraGroup).with(environment.group).with(fog.faces),
      );
      pass.end();
    },
    stats() {
      return {
        worldVertices: world.opaque.vertices + world.translucent.vertices,
        structureVertices: structures.vertices,
        instances: proxies.count,
        shadow: environment.stats(),
      };
    },
  };
}
export type WorldPass = Awaited<ReturnType<typeof createWorldPass>>;
