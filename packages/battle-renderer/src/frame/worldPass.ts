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
// surface at the depth the prepass left. The prepass runs in two halves: the
// world without its units first, against which the own units' hidden parts
// are drawn into the overlay target (the x-ray), then the units; its depth is
// copied for the overlays before the grass adds its blades.
//
// Five kinds of world geometry, all lit, fogged, graded and shadow-casting:
// - the terrain: the simulation's ground triangles under the biome's
//   material and the observing side's learned scars (`scarTexture.ts`:
//   crater bowls and rims as shading only, scorch, tracks, trampling), and
//   the one layer FogTerm treats as ground;
// - the map's skirt (and, in the traversal view, the props' boxes);
// - the proxies (lab markers);
// - the models layer: appearance bundles, skinned, articulated and static
//   (`models/modelLayer.ts`), with their own vertex stage, and far models as
//   impostor cards (`models/impostorCards.ts`). Its static props are the
//   world's (the map's props that cannot fall) and the structures, what the
//   side knows stands: buildings, their ruins and wrecks, fitted to their
//   boxes. Each draw binds the fog group its class names (`modelFog`);
// - the forest's trees (`sceneryLayer.ts`), which draw the simulation's trunks.
// Plus the backdrop past the map edge and the hedgerows and copses on it:
// the same ground material (the patchwork runs on past the map), lit, hazed
// and fogged like the world (sight runs on past the edge over open ground),
// but unshadowed and casting nothing.
// Grass grows on the terrain (`grassPass.ts`): regrown by compute before the
// colour pass whenever the view moves, drawn after the opaque world, and
// marked as ground in the fog mask. The ground paint (`paintedMarks.ts`:
// orders, rings, the border) is drawn after the prepass's ground-only half,
// and the painted ground layers (terrain, grass, backdrop: fog's
// `paintedGround`) take it as their own surface.
import { tgpu, d, std, type TgpuCommandEncoder } from "typegpu";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { SceneInstance, WorldLayers } from "../scene";
import type { Mesh } from "../mesh";
import type { ModelInstance } from "../models/modelInstances";
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
import {
  fogCoverage,
  fogIsGround,
  fogTerm,
  groundPaint,
  paintedAlbedo,
  paintedSeen,
  paintGlow,
} from "./fogTerm";
import { createGrassPass } from "./grassPass";
import { createFogVisibility, type FogTiles } from "./fogVisibility";
import {
  createTerrainSource,
  groundDapple,
  groundScarsSeen,
  groundSurface,
  scarredNormal,
  scarredSurface,
  waterNormal,
  waterSurface,
  WATER_ROUGHNESS,
  WATER_SHADOW,
} from "./terrainMaterial";
import { createSceneryLayer } from "./sceneryLayer";
import { createPaintedMarks, validatePaintStyle, type PaintStyle } from "./paintedMarks";
import type { GroundMarks } from "./scarTexture";
import type { FogGeometryPresentation, FogInput } from "./fogInputs";
import type { Box3 } from "math/shapes";
import type { Mat4 } from "math";
import { mapBox } from "./receiverRange";
import {
  createModelFragments,
  modelAttribs,
  modelRecordLayout,
  modelVertex,
  modelXrayFragment,
  type ModelLayer,
} from "../models/modelLayer";
import { cardVertex, createCardFragment } from "../models/impostorCards";
import { FRAME_MSAA, OVERLAY_FORMAT, WORLD_OUT, worldTargets, type FrameTargets } from "./targets";
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

/** The x-ray's margin: see `modelXray`. */
const XRAY_DEPTH_BIAS = 1 << 16;
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
  paintStyle: PaintStyle,
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
    const plain = groundAlbedo(v.world, v.color);
    const footprint = std.length(std.fwidth(v.world.xy));
    // The side's learned scars, on the biome's ground (not under a tint).
    const biome = 1 - v.color.w;
    const scar = groundScarsSeen(v.world, eye, footprint);
    const surface = std.mix(plain, scarredSurface(plain, scar), biome);
    const shading = std.normalize(std.mix(n, scarredNormal(n, scar), biome));
    // The ground paint on it (its layer is painted).
    const paint = groundPaint(v.world);
    // Sun flecks through the crowns lift the canopy's whole shadow.
    const flecks = groundDapple(v.world.xy, footprint);
    const sun = std.max(environment.sampleSunShadow(v.world, n, v.clip.xy), flecks);
    const lit = environment.shade(
      paintedAlbedo(surface.xyz, paint),
      d.vec3f(0),
      surface.w,
      0,
      0,
      1,
      shading,
      v.world,
      sun,
      eye,
    );
    return {
      color: d.vec4f(std.add(lit.xyz, paintGlow(paint)), 1),
      fog: fogCoverage(paintedSeen(seen, paint), 1),
    };
  });
  /** The backdrop: the same ground, light and fog, never shadowed. */
  const backdropFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const surface = groundAlbedo(v.world, v.color);
    const up = std.normalize(v.normal);
    const paint = groundPaint(v.world);
    const albedo = paintedAlbedo(surface.xyz, paint);
    const lit = environment.shade(albedo, d.vec3f(0), surface.w, 0, 0, 1, up, v.world, 1, eye);
    // Fog runs on past the playable area: the sight maps continue over open
    // ground (no occluders or foliage) beyond the edge.
    const seen = fogTerm(v.world, up, v.clip.xy, fogIsGround());
    return {
      color: d.vec4f(std.add(lit.xyz, paintGlow(paint)), 1),
      fog: fogCoverage(paintedSeen(seen, paint), 1),
    };
  });

  /** The water surface (the terrain material's `waterSurface`): ripples that
   *  break the sky's and sun's reflection, the bed through it at the shore.
   *  It takes the ground paint as the ground does (its layer is painted): a
   *  mark over the water shows on it, not drowned under it. */
  const waterFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    const eye = typegpuCameraLayout.$.cam.eye;
    const footprint = std.length(std.fwidth(v.world.xy));
    const surface = waterSurface(v.world.xy);
    const n = waterNormal(v.world.xy, footprint);
    const sun = environment.sampleSunShadow(v.world, n, v.clip.xy);
    const paint = groundPaint(v.world);
    const lit = environment.shade(
      paintedAlbedo(surface.xyz, paint),
      d.vec3f(0),
      WATER_ROUGHNESS,
      0,
      0,
      1,
      n,
      v.world,
      sun,
      eye,
    );
    const shaded = std.add(std.mul(lit.xyz, std.mix(WATER_SHADOW, 1, sun)), paintGlow(paint));
    const seen = paintedSeen(fogTerm(v.world, n, v.clip.xy, fogIsGround()), paint);
    const alpha = std.max(surface.w, paint.w);
    return { color: d.vec4f(shaded, alpha), fog: fogCoverage(seen, alpha) };
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
  const water = root.createRenderPipeline({
    ...base,
    fragment: waterFragment,
    // The fog mask blends as the colour does: water over unseen ground is as
    // unseen as its alpha lets the ground through.
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
  // The x-ray: a unit's fragments behind the world's depth (without the
  // units), over the transparent overlay target. Max blending, so a hidden
  // arm behind a hidden torso never doubles the silhouette's alpha. The
  // bias pulls each fragment toward the eye by about 1/128 of its distance
  // (float reverse-Z: a constant bias of 2^16 is ~2^-7 of the depth), so a
  // body touching the ground (a prone man, a track's lower run) is not
  // x-rayed where it dips under the surface; a canopy or a wall hides by
  // metres.
  const modelXray = root.createRenderPipeline({
    ...modelBase,
    fragment: modelXrayFragment,
    targets: {
      format: OVERLAY_FORMAT,
      blend: {
        color: { operation: "max", srcFactor: "one", dstFactor: "one" },
        alpha: { operation: "max", srcFactor: "one", dstFactor: "one" },
      },
    },
    depthStencil: { ...battleWorldDepth("behind"), depthBias: XRAY_DEPTH_BIAS },
    multisample: { count: FRAME_MSAA },
  });
  // Cards are alpha-tested quads: they write depth in the colour pass.
  const modelCards = root.createRenderPipeline({
    attribs: modelRecordLayout.attrib,
    vertex: cardVertex,
    fragment: createCardFragment(environment),
    primitive: { topology: "triangle-list", cullMode: "none" },
    targets: worldTargets(),
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  await Promise.all(
    [
      prepass,
      caster,
      opaque,
      water,
      terrainPipeline,
      backdropPipeline,
      modelPrepass,
      modelCaster,
      modelOpaque,
      modelXray,
      modelCards,
    ].map((pipeline) => pipeline.initAsync()),
  );
  /** The models layer's draws take the pipelines' binding methods as they are. */
  const drawModels = (bound: unknown, fog?: Parameters<ModelLayer["draw"]>[1]) =>
    models.draw(bound as Parameters<ModelLayer["draw"]>[0], fog);
  const drawCards = (bound: unknown, fog: Parameters<ModelLayer["drawCards"]>[1]) =>
    models.drawCards(bound as Parameters<ModelLayer["drawCards"]>[0], fog);

  const fog = await createFogVisibility(root, registry, fogGeometry);
  const scenery = await createSceneryLayer(root, registry, environment);
  const terrain = createTerrainSource(root, registry);
  const grass = await createGrassPass(root, registry, environment, terrain);
  const paint = createPaintedMarks(root, registry);
  await paint.ready();
  fog.setPaintStyle(validatePaintStyle(paintStyle));
  const identity = identityInstance(root, registry);
  const world = {
    ground: new MeshSlot(root, registry, identity),
    props: new MeshSlot(root, registry, identity),
    water: new MeshSlot(root, registry, identity),
  };
  // The models layer's statics: the world's props, then the side's structures.
  let worldProps: readonly ModelInstance[] = [];
  let structures: readonly ModelInstance[] = [];
  const setStatics = () => models.setStatics([...worldProps, ...structures]);
  const backdrop = new MeshSlot(root, registry, identity);
  const proxies = new ProxyInstances(root, registry);
  let box: Box3 | null = null;

  return {
    setWorld(next: WorldLayers) {
      world.ground.set(next.terrain.mesh);
      world.props.set(next.props);
      world.water.set(next.water);
      terrain.set(next.terrain);
      scenery.set(next.scenery);
      grass.setWorld(next.terrain, next.grass);
      worldProps = next.structures;
      setStatics();
      box = mapBox(next.terrain.mesh);
      if (box) backdrop.set(backdropMesh(box, environment.light.backdrop.reach_m));
    },
    setStructures(next: readonly ModelInstance[]) {
      structures = next;
      setStatics();
    },
    /** Follow the side's learned ground: uploads what changed, and regrows
     *  the grass and fells the cleared trees when anything did. */
    setGround(next: GroundMarks | null): boolean {
      const changed = terrain.setGround(next);
      if (changed) {
        grass.regrow();
        scenery.setCleared(next);
      }
      return changed;
    },
    /** The painted ground marks: still, and marching on the clock. */
    setPainted(still: Mesh, marching: Mesh) {
      paint.set(still, marching);
    },
    setPaintShown(on: boolean) {
      paint.setShown(on);
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
        proxies.draw(bound);
        models.drawCasters(
          modelCaster.with(pass).with(cameraGroup) as unknown as Parameters<ModelLayer["draw"]>[0],
        );
        scenery.encodeShadows(pass, cameraGroup);
      });
    },
    /** The frame's depth: every opaque layer, before any colour. Between the
     *  world and its units, the own units' hidden parts are drawn into the
     *  overlay target (clearing it); at the end the depth is copied for the
     *  overlays. */
    encodeDepth(
      encoder: TgpuCommandEncoder,
      raw: GPUCommandEncoder,
      targets: FrameTargets,
      cameraGroup: CameraGroup,
    ) {
      const depthView = targets.depth.createView();
      const scene = encoder.beginRenderPass({
        label: "depth-prepass-world",
        colorAttachments: [],
        depthStencilAttachment: {
          view: depthView,
          depthClearValue: BATTLE_DEPTH_ATTACHMENT.clearValue,
          depthLoadOp: "clear",
          depthStoreOp: "store",
        },
      });
      const bound = prepass.with(scene).with(cameraGroup);
      world.ground.draw(bound);
      world.props.draw(bound);
      backdrop.draw(bound);
      scenery.encodeDepth(scene, cameraGroup);
      scene.end();

      // The ground paint, against the ground-only depth. Its target is
      // cleared first on its own: Metal skips a resolve on tiles a pass
      // draws nothing into, which would keep an older frame's paint.
      encoder
        .beginRenderPass({
          label: "ground-paint-clear",
          colorAttachments: [
            {
              view: targets.paint.createView(),
              loadOp: "clear",
              storeOp: "store",
              clearValue: [0, 0, 0, 0],
            },
          ],
        })
        .end();
      paint.encode(encoder, targets, depthView, cameraGroup);

      const xray = encoder.beginRenderPass({
        label: "unit-xray",
        colorAttachments: [
          {
            view: targets.overlayMsaa.createView(),
            loadOp: "clear",
            storeOp: "store",
            clearValue: [0, 0, 0, 0],
          },
        ],
        depthStencilAttachment: { view: depthView, depthReadOnly: true },
      });
      drawModels(modelXray.with(xray).with(cameraGroup), "units");
      xray.end();

      const units = encoder.beginRenderPass({
        label: "depth-prepass-units",
        colorAttachments: [],
        depthStencilAttachment: { view: depthView, depthLoadOp: "load", depthStoreOp: "store" },
      });
      proxies.draw(prepass.with(units).with(cameraGroup));
      drawModels(modelPrepass.with(units).with(cameraGroup));
      units.end();
      raw.copyTextureToTexture({ texture: targets.depth }, { texture: targets.overlayDepth }, [
        targets.width,
        targets.height,
      ]);
    },
    /** The sky, then the terrain, props, proxies and models at the
     *  prepass's depth, then the water: lit into `lit`, with the
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
          .with(fogGroups.paintedGround)
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
      // Each model draws through the fog group its class names (`modelFog`):
      // posed units never fogged, props face by face (an occluding
      // structure whole, in FogTerm), corpses as the
      // ground under them.
      const modelsLit = modelOpaque.with(pass).with(cameraGroup).with(environment.group);
      const cardsLit = modelCards.with(pass).with(cameraGroup).with(environment.group);
      for (const fog of ["units", "faces", "ground", "paintedFaces"] as const) {
        drawModels(modelsLit.with(fogGroups[fog]), fog);
        drawCards(cardsLit.with(fogGroups[fog]), fog);
      }
      scenery.encode(pass, cameraGroup, fogGroups.faces);
      grass.draw(pass, cameraGroup, fogGroups.paintedGround);
      backdrop.draw(
        backdropPipeline
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogGroups.paintedGround)
          .with(terrain.group),
      );
      world.water.draw(
        water
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogGroups.paintedFaces)
          .with(terrain.group),
      );
      pass.end();
    },
    stats() {
      return {
        worldVertices: world.ground.vertices + world.props.vertices + world.water.vertices,
        structures: structures.length,
        instances: proxies.count,
        shadow: environment.stats(),
        fog: fog.stats(),
        scenery: scenery.stats(),
        grass: grass.stats(),
        scars: terrain.scarStats(),
        paint: paint.stats(),
      };
    },
  };
}
export type WorldPass = Awaited<ReturnType<typeof createWorldPass>>;
