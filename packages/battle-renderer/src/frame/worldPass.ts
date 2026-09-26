// The HDR world: the sun's cascades, a depth prepass, then the sky and the lit
// world into the multisampled rgba16float target. Every world material shades
// with the ported environment (PBR, PMREM, sun shadow, aerial haze) and writes
// FogTerm's answer into the fog mask beside its colour; the fog mask pass
// (`fogMaskPass.ts`) then gives unseen pixels the frame's FogStyle, so fog
// still lands before post like any other light. Units are drawn by
// identification and never fogged: their mask is empty.
//
// The depth prepass writes the frame's 4× MSAA depth before any colour, so
// FogVisibility's tile cull reads the scene's depth (sample 0) ahead of the
// colour pass (spike 02, landmine 6). The colour pass then shades each opaque
// surface at the depth the prepass left.
//
// Six kinds of world geometry, all lit, fogged, graded and shadow-casting:
// - the terrain: the simulation's ground triangles under the biome's
//   material, and the one layer FogTerm treats as ground;
// - the static props standing on it (buildings, walls, the skirt);
// - the proxies (units);
// - the structures layer: what the side knows stands (buildings, remembered
//   ruins and wrecks);
// - the models layer: appearance bundles, skinned, articulated and static
//   (`models/modelLayer.ts`), with their own vertex stage;
// - the forest's trees (`sceneryLayer.ts`), which draw the simulation's trunks.
// Plus the backdrop past the map edge and the hedgerows and copses on it:
// the same ground material (the patchwork runs on past the map), lit and
// hazed like the world, but unfogged, unshadowed and casting nothing.
// Grass grows on the terrain (`grassPass.ts`): regrown by compute before the
// colour pass whenever the view moves, drawn after the opaque world, fogged
// as ground.
import { tgpu, d, std, type TgpuCommandEncoder } from "typegpu";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { Mesh } from "../mesh";
import type { SceneInstance, WorldLayers } from "../scene";
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
  WORLD_VARYING,
  ProxyInstances,
} from "./geometry";
import { fogCoverage, fogIsGround, fogTerm } from "./fogTerm";
import { createGrassPass } from "./grassPass";
import { createFogVisibility, type FogTiles } from "./fogVisibility";
import { createTerrainSource, groundSurface } from "./terrainMaterial";
import { createSceneryLayer } from "./sceneryLayer";
import type { FogGeometryPresentation, FogInput } from "./fogInputs";
import type { Box3 } from "math/shapes";
import type { Mat4 } from "math";
import { mapBox } from "./receiverRange";
import {
  createModelFragments,
  modelAttribs,
  modelVertex,
  type ModelLayer,
} from "../models/modelLayer";
import { FRAME_MSAA, WORLD_OUT, worldTargets, type FrameTargets } from "./targets";
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
  fogGeometry: FogGeometryPresentation,
  models: ModelLayer,
) {
  const worldFragment = tgpu.fragmentFn({
    in: {
      clip: d.builtin.position,
      world: WORLD_VARYING,
      normal: d.vec3f,
      color: d.vec4f,
      highlight: d.f32,
    },
    out: WORLD_OUT,
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
    return {
      color: d.vec4f(std.add(lit.xyz, glow), v.color.w),
      fog: fogCoverage(seen, v.color.w),
    };
  });
  const varyings = {
    clip: d.builtin.position,
    world: WORLD_VARYING,
    normal: d.vec3f,
    color: d.vec4f,
    highlight: d.f32,
  };
  /** The biome's ground, under the vertex tint (its alpha the tint's weight). */
  const groundAlbedo = tgpu.fn(
    [d.vec3f, d.vec4f],
    d.vec4f,
  )((world, tint) => {
    "use gpu";
    const footprint = std.length(std.fwidth(world.xy));
    const surface = groundSurface(world, footprint);
    const albedo = std.mix(surface.xyz, srgbToLinear(tint.xyz), tint.w);
    return d.vec4f(albedo, std.mix(surface.w, ROUGHNESS, tint.w));
  });
  /** The terrain: FogTerm's ground. */
  const terrainFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const n = std.normalize(v.normal);
    const seen = fogTerm(v.world, n, v.clip.xy, fogIsGround());
    const surface = groundAlbedo(v.world, v.color);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const lit = environment.shade(
      surface.xyz,
      d.vec3f(0),
      surface.w,
      0,
      0,
      1,
      n,
      v.world,
      sun,
      eye,
    );
    return { color: d.vec4f(lit.xyz, 1), fog: fogCoverage(seen, 1) };
  });
  /** The backdrop: the same ground and light, never fogged or shadowed (an
   *  empty fog mask). */
  const backdropFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const surface = groundAlbedo(v.world, v.color);
    const up = std.normalize(v.normal);
    const lit = environment.shade(surface.xyz, d.vec3f(0), surface.w, 0, 0, 1, up, v.world, 1, eye);
    return { color: d.vec4f(lit.xyz, 1), fog: d.vec4f(0, 0, 0, 1) };
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
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  const translucent = root.createRenderPipeline({
    ...base,
    fragment: worldFragment,
    // The fog mask blends as the colour does: a canopy over unseen ground is
    // as unseen as its alpha lets the ground through.
    targets: worldTargets({
      color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
      alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
    }),
    depthStencil: battleWorldDepth("read"),
    multisample: { count: FRAME_MSAA },
  });
  const terrainPipeline = root.createRenderPipeline({
    ...base,
    fragment: terrainFragment,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  const backdropPipeline = root.createRenderPipeline({
    ...base,
    fragment: backdropFragment,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  const modelBase = {
    attribs: modelAttribs,
    vertex: modelVertex,
    primitive: { topology: "triangle-list", cullMode: "none" },
  } as const;
  const modelFragments = createModelFragments(environment);
  const modelPrepass = root.createRenderPipeline({
    ...modelBase,
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  const modelCaster = root.createRenderPipeline({
    ...modelBase,
    depthStencil: battleWorldDepth("read-write"),
  });
  const modelOpaque = root.createRenderPipeline({
    ...modelBase,
    fragment: modelFragments.lit,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  await Promise.all(
    [
      prepass,
      caster,
      opaque,
      translucent,
      terrainPipeline,
      backdropPipeline,
      modelPrepass,
      modelCaster,
      modelOpaque,
    ].map((pipeline) => pipeline.initAsync()),
  );
  /** The models layer's draws take the pipelines' binding methods as they are. */
  const drawModels = (bound: unknown) => models.draw(bound as Parameters<ModelLayer["draw"]>[0]);

  const fog = await createFogVisibility(root, registry, fogGeometry);
  const scenery = await createSceneryLayer(root, registry, environment);
  const terrain = createTerrainSource(root, registry);
  const grass = await createGrassPass(root, registry, environment, terrain);
  const identity = identityInstance(root, registry);
  const world = {
    ground: new MeshSlot(root, registry, identity),
    props: new MeshSlot(root, registry, identity),
    translucent: new MeshSlot(root, registry, identity),
  };
  const structures = new MeshSlot(root, registry, identity);
  const backdrop = new MeshSlot(root, registry, identity);
  const proxies = new ProxyInstances(root, registry);
  let box: Box3 | null = null;

  return {
    setWorld(next: WorldLayers) {
      world.ground.set(next.terrain.mesh);
      world.props.set(next.props);
      world.translucent.set(next.translucent);
      terrain.set(next.terrain);
      scenery.set(next.scenery);
      grass.setWorld(next.terrain, next.grass);
      box = mapBox(next.terrain.mesh);
      if (box) backdrop.set(backdropMesh(box, environment.light.backdrop.reach_m));
    },
    /** The installed grass appearances; null draws no grass. */
    setStructures(next: Mesh) {
      structures.set(next);
    },
    setInstances(next: readonly SceneInstance[]) {
      proxies.set(next);
    },
    setFog(next: FogInput | null) {
      fog.set(next);
    },
    /** The fog tile lists for a frame size, in that size's scope. */
    fogTiles: fog.sized,
    /** Fog's rebuilds and tile cull: after the depth prepass, before colour. */
    encodeFog(
      raw: GPUCommandEncoder,
      tiles: FogTiles,
      camera: Float32Array,
      width: number,
      height: number,
    ) {
      fog.encode(raw, tiles, camera, width, height);
    },
    fog,
    grassProbes: grass.probes,
    /** Pose the environment and the cascades for this frame's camera, choose
     *  the trees' detail and fit the grass for a viewport `height` pixels tall. */
    prepare(
      camera: Camera3DParams,
      view: ArrayLike<number>,
      viewProj: Mat4,
      rays: SkyRays,
      height: number,
    ) {
      environment.prepare(camera, view, rays, box);
      scenery.prepare(camera, height);
      grass.prepare(camera, viewProj, height);
    },
    encodeShadows(encoder: TgpuCommandEncoder) {
      scenery.beginFrame();
      environment.encodeShadows(encoder, (pass, cameraGroup) => {
        const bound = caster.with(pass).with(cameraGroup);
        world.ground.draw(bound);
        world.props.draw(bound);
        structures.draw(bound);
        proxies.draw(bound);
        drawModels(modelCaster.with(pass).with(cameraGroup));
        scenery.encodeShadows(pass, cameraGroup);
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
      world.ground.draw(bound);
      world.props.draw(bound);
      proxies.draw(bound);
      structures.draw(bound);
      drawModels(modelPrepass.with(pass).with(cameraGroup));
      backdrop.draw(bound);
      scenery.encodeDepth(pass, cameraGroup);
      pass.end();
    },
    /** The sky, then the terrain, props, proxies and structures at the
     *  prepass's depth, then the translucent world: lit into `lit`, with the
     *  fog mask beside it in `fogMask` (the sky's pixels empty: never fogged). */
    encode(
      encoder: TgpuCommandEncoder,
      raw: GPUCommandEncoder,
      targets: FrameTargets,
      cameraGroup: CameraGroup,
    ) {
      grass.encodeBuild(raw);
      const colorView = targets.hdrMsaa.createView();
      const litView = targets.lit.createView();
      // The sky resolves into `lit` too: where the world pass below draws
      // nothing (screen tiles of pure sky), Metal skips the tile, and its
      // resolve with it, so `lit` would keep an older frame's pixels there.
      environment.encodeBackground(raw, colorView, litView);
      const pass = encoder.beginRenderPass({
        label: "world",
        colorAttachments: [
          {
            view: colorView,
            resolveTarget: litView,
            loadOp: "load",
            storeOp: "discard",
          },
          {
            view: targets.fogMaskMsaa.createView(),
            resolveTarget: targets.fogMask.createView(),
            loadOp: "clear",
            storeOp: "discard",
            clearValue: [0, 0, 0, 0],
          },
        ],
        depthStencilAttachment: {
          view: targets.depth.createView(),
          depthLoadOp: "load",
          // The overlay pass depth-tests against the world.
          depthStoreOp: "store",
        },
      });
      const fogGroups = fog.groups();
      world.ground.draw(
        terrainPipeline
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogGroups.ground)
          .with(terrain.group),
      );
      const faces = opaque
        .with(pass)
        .with(cameraGroup)
        .with(environment.group)
        .with(fogGroups.faces);
      world.props.draw(faces);
      proxies.draw(
        opaque.with(pass).with(cameraGroup).with(environment.group).with(fogGroups.units),
      );
      structures.draw(faces);
      drawModels(
        modelOpaque.with(pass).with(cameraGroup).with(environment.group).with(fogGroups.faces),
      );
      scenery.encode(pass, cameraGroup, fogGroups.faces);
      grass.draw(pass, cameraGroup, fogGroups.ground);
      backdrop.draw(
        backdropPipeline.with(pass).with(cameraGroup).with(environment.group).with(terrain.group),
      );
      world.translucent.draw(
        translucent.with(pass).with(cameraGroup).with(environment.group).with(fogGroups.faces),
      );
      pass.end();
    },
    stats() {
      return {
        worldVertices: world.ground.vertices + world.props.vertices + world.translucent.vertices,
        structureVertices: structures.vertices,
        instances: proxies.count,
        shadow: environment.stats(),
        fog: fog.stats(),
        scenery: scenery.stats(),
        grass: grass.stats(),
      };
    },
  };
}
export type WorldPass = Awaited<ReturnType<typeof createWorldPass>>;
