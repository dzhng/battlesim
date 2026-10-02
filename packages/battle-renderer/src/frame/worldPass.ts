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
// Six kinds of world geometry, all lit, fogged, graded and shadow-casting:
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
// - a town's buildings, as instances of their kits' modules
//   (`models/buildingLayer.ts`): static models in every respect, drawn by the
//   models layer's pipelines as the world's faces, and in the prepass's world
//   half, so a unit behind one is x-rayed;
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
import {
  createProjectedPoint,
  projectPoint,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";
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
import { createTerrainHeights } from "./terrainHeights";
import { createFogVisibility, type FogTiles } from "./fogVisibility";
import {
  createTerrainSource,
  groundCell,
  groundClasses,
  groundClassView,
  groundColour,
  groundDapple,
  GroundPaved,
  groundPaved,
  groundRuts,
  groundScarsSeen,
  groundSite,
  groundBank,
  groundWater,
  scarRegionContains,
  scarredNormal,
  scarredSurface,
  waterNormal,
  waterSurface,
  WATER_ROUGHNESS,
  WATER_SHADOW,
} from "./terrainMaterial";
import { createSceneryLayer } from "./sceneryLayer";
import { createPaintedMarks, validatePaintStyle, type PaintStyle } from "./paintedMarks";
import type { GroundMarks, ScarRegion } from "./scarTexture";
import type { FogGeometryPresentation, FogInput } from "./fogInputs";
import type { Box3 } from "math/shapes";
import { vec3, type Mat4 } from "math";
import { mapBox } from "./receiverRange";
import {
  createModelFragments,
  modelAttribs,
  modelRecordLayout,
  modelVertex,
  type ModelLayer,
} from "../models/modelLayer";
import {
  createGlassFragment,
  createRoomFragment,
  modelCutoutCaster,
  modelCutoutDepth,
  type GlassStyle,
} from "../models/surfaceFragments";
import {
  modelXrayFragment,
  modelXrayCountFragment,
  xrayCountLayout,
  xrayReadLayout,
  XRAY_DEPTH_BIAS,
  validateXrayFraction,
} from "../models/modelXray";
import { cardVertex, createCardFragment } from "../models/impostorCards";
import {
  FOG_MASK_FORMAT,
  FRAME_MSAA,
  HDR_FORMAT,
  OVERLAY_FORMAT,
  WORLD_OUT,
  worldTargets,
  type FrameTargets,
} from "./targets";
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
  paintStyle: PaintStyle,
  xrayMinHiddenFragmentFraction: number,
  glass: GlassStyle,
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
  /** The biome's ground, under the vertex tint (its alpha the tint's weight).
   *  `cell` is the point's `groundCell`, `paved` its `groundPaved`. */
  const groundAlbedo = tgpu.fn(
    [d.vec3f, d.vec4f, d.vec4u, GroundPaved],
    d.vec4f,
  )((world, tint, cell, paved) => {
    "use gpu";
    const footprint = std.length(std.fwidth(world.xy));
    const xy = world.xy;
    const site = groundSite(xy, cell, paved);
    const surface = groundColour(xy, footprint, site, paved, groundWater(xy, cell));
    const albedo = std.mix(surface.xyz, srgbToLinear(tint.xyz), tint.w);
    return d.vec4f(albedo, std.mix(surface.w, ROUGHNESS, tint.w));
  });
  /** What the class view's ground writes beside its bytes: seen ground,
   *  whether or not the frame has fog (the fog mask pass keeps only pixels
   *  that are wholly this). */
  const CLASS_GROUND = d.vec4f(0, 1, 1, 1);
  /** The terrain: FogTerm's ground. */
  const terrainFragment = tgpu.fragmentFn({ in: varyings, out: WORLD_OUT })((v) => {
    "use gpu";
    if (!scarRegionContains(v.world.xy)) std.discard();
    const eye = typegpuCameraLayout.$.cam.eye;
    const n = std.normalize(v.normal);
    const seen = fogTerm(v.world, n, v.clip.xy, fogIsGround());
    const footprint = std.length(std.fwidth(v.world.xy));
    // One lookup of the surface field serves the ground and its dapple.
    const cell = groundCell(v.world.xy, footprint);
    const paved = groundPaved(v.world.xy, cell);
    if (groundClassView()) {
      const xy = v.world.xy;
      const site = groundSite(xy, cell, paved);
      const classes = groundClasses(xy, footprint, site, groundWater(xy, cell));
      return { color: d.vec4f(classes, 1), fog: d.vec4f(CLASS_GROUND) };
    }
    const plain = groundAlbedo(v.world, v.color, cell, paved);
    // The side's learned scars, on the biome's ground (not under a tint).
    const biome = 1 - v.color.w;
    const scar = groundScarsSeen(v.world, eye, footprint);
    const surface = std.mix(plain, scarredSurface(plain, scar), biome);
    // A river's bank is lit as the cross-section it was cut to and its bed
    // flat, not as the grid's triangles (`groundBank`).
    const bank = groundBank(v.world.xy, cell, footprint);
    let ground = d.vec3f(n);
    if (bank.z > 0) {
      ground = std.normalize(std.mix(n, std.normalize(d.vec3f(-bank.x, -bank.y, 1)), bank.z));
    }
    // A road's ruts tilt it too: their sides catch the sun.
    const ruts = groundRuts(v.world.xy, footprint, paved);
    ground = std.normalize(std.sub(ground, std.mul(d.vec3f(ruts.xy, 0), ground.z * biome)));
    const shading = std.normalize(std.mix(ground, scarredNormal(ground, scar), biome));
    // The ground paint on it (its layer is painted).
    const paint = groundPaint(v.world);
    // Sun flecks through the crowns lift the canopy's whole shadow.
    const flecks = groundDapple(v.world.xy, footprint, cell);
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
    const cell = groundCell(v.world.xy, std.length(std.fwidth(v.world.xy)));
    // Past the map's edge there is no class to read: not ground, to that view.
    if (groundClassView()) {
      return { color: d.vec4f(0, 0, 0, 1), fog: d.vec4f(0, 0, 0, 1) };
    }
    const surface = groundAlbedo(v.world, v.color, cell, groundPaved(v.world.xy, cell));
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
    const surface = waterSurface(v.world.xy, footprint);
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
  // A cutout (a grille, a perforated sheet) cuts itself where depth is
  // written: in the prepass, sample by sample, and in the sun's cascades. The
  // colour pass then shades the samples the prepass kept, with the opaque
  // surfaces' own fragment stage, so one stage decides its silhouette.
  const modelPrepassCutout = root.createRenderPipeline({
    ...modelBase,
    fragment: modelCutoutDepth,
    depthStencil: battleWorldDepth("read-write"),
    multisample: { count: FRAME_MSAA },
  });
  const modelCasterCutout = root.createRenderPipeline({
    ...modelBase,
    fragment: modelCutoutCaster,
    depthStencil: battleWorldDepth("read-write"),
  });
  const modelCutout = root.createRenderPipeline({
    ...modelBase,
    fragment: modelFragments.lit,
    targets: worldTargets(),
    depthStencil: battleWorldDepth("kept"),
    multisample: { count: FRAME_MSAA },
  });
  // A room behind a window is as solid as a wall to depth and shadow (the
  // fragment-less pipelines draw it with the opaque surfaces); its colour is
  // its own stage's, a picture shown unlit.
  const modelRooms = root.createRenderPipeline({
    ...modelBase,
    fragment: createRoomFragment(environment),
    targets: worldTargets(),
    depthStencil: battleWorldDepth("prepassed"),
    multisample: { count: FRAME_MSAA },
  });
  // A blended surface (glass) is drawn after everything opaque, over it by
  // its coverage, as the water is: it reads depth and writes none, is in no
  // prepass and casts no shadow. Unlike the water it leaves the fog mask
  // alone: a pane is not the thing seen or unseen, what stands behind it is.
  const modelBlended = root.createRenderPipeline({
    ...modelBase,
    fragment: createGlassFragment(environment, glass),
    targets: {
      color: {
        format: HDR_FORMAT,
        blend: {
          color: { srcFactor: "src-alpha", dstFactor: "one-minus-src-alpha" },
          alpha: { srcFactor: "one", dstFactor: "one-minus-src-alpha" },
        },
      },
      fog: { format: FOG_MASK_FORMAT, writeMask: 0 },
    },
    depthStencil: battleWorldDepth("read"),
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
  const xrayMinimum = registry.own(
    root.createBuffer(d.f32, validateXrayFraction(xrayMinHiddenFragmentFraction)).$usage("uniform"),
  );
  const xrayCounts = registry.slot<GPUBuffer>();
  let xrayCapacity = 0;
  let xrayCoverageEnabled = true;
  const countGroupFor = (depth: GPUTexture) =>
    root.createBindGroup(xrayCountLayout, {
      counts: xrayCounts.current!,
      depth: depth.createView(),
    });
  const readGroupFor = () =>
    root.createBindGroup(xrayReadLayout, { counts: xrayCounts.current!, minimum: xrayMinimum });
  let xrayDepth: GPUTexture | null = null;
  let xrayCountGroup: ReturnType<typeof countGroupFor> | null = null;
  let xrayReadGroup: ReturnType<typeof readGroupFor> | null = null;
  const modelXrayCount = root.createRenderPipeline({
    ...modelBase,
    fragment: modelXrayCountFragment,
    targets: { format: OVERLAY_FORMAT, writeMask: 0 },
    multisample: { count: FRAME_MSAA },
  });
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
      modelPrepassCutout,
      modelCasterCutout,
      modelCutout,
      modelRooms,
      modelBlended,
      modelXray,
      modelXrayCount,
      modelCards,
    ].map((pipeline) => pipeline.initAsync()),
  );
  /** The models layer's draws take the pipelines' binding methods as they are. */
  type Bound = Parameters<ModelLayer["draw"]>[0];
  const drawModels = (
    bound: unknown,
    surface: Parameters<ModelLayer["draw"]>[1],
    fog?: Parameters<ModelLayer["draw"]>[2],
  ) => models.draw(bound as Bound, surface, fog);
  const drawCards = (bound: unknown, fog: Parameters<ModelLayer["drawCards"]>[1]) =>
    models.drawCards(bound as Parameters<ModelLayer["drawCards"]>[0], fog);

  const terrainHeights = createTerrainHeights(registry);
  const fog = await createFogVisibility(root, registry, fogGeometry, terrainHeights);
  const scenery = await createSceneryLayer(root, registry, environment);
  const terrain = createTerrainSource(root, registry);
  await terrain.ready();
  const grass = await createGrassPass(root, registry, environment, terrain, terrainHeights);
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
  let frameProjection: Mat4 | null = null;
  const corner = vec3.create(),
    projected = createProjectedPoint();
  // A projected convex region's extrema lie on its corners when entirely in
  // front of the eye. Regions crossing the eye retain the full viewport.
  const scissor = (
    r: ScarRegion,
    width: number,
    height: number,
  ): readonly [number, number, number, number] | null => {
    if (!box || !frameProjection) return [0, 0, width, height];
    let front = 0,
      loX = Infinity,
      loY = Infinity,
      hiX = -Infinity,
      hiY = -Infinity;
    for (const x of [r[0], r[2]])
      for (const y of [r[1], r[3]])
        for (const z of [box[2], box[5]]) {
          vec3.set(corner, x, y, z);
          projectPoint(projected, frameProjection, corner);
          if (projected.clipW <= 0) continue;
          front++;
          loX = Math.min(loX, projected.ndc[0]);
          hiX = Math.max(hiX, projected.ndc[0]);
          loY = Math.min(loY, projected.ndc[1]);
          hiY = Math.max(hiY, projected.ndc[1]);
        }
    if (!front) return null;
    if (front < 8) return [0, 0, width, height];
    const x = Math.max(0, Math.floor(((loX + 1) * width) / 2) - 2),
      y = Math.max(0, Math.floor(((1 - hiY) * height) / 2) - 2);
    const right = Math.min(width, Math.ceil(((hiX + 1) * width) / 2) + 2),
      bottom = Math.min(height, Math.ceil(((1 - loY) * height) / 2) + 2);
    return right > x && bottom > y ? [x, y, right - x, bottom - y] : null;
  };

  let classView = false;

  return {
    setXrayCoverageEnabled(on: boolean) {
      xrayCoverageEnabled = on;
      xrayMinimum.write(on ? xrayMinHiddenFragmentFraction : 0);
    },
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
    /** The painted marks that follow the pointer (`BattleFrame.setPointerMarks`). */
    setPointerPaint(marks: Mesh) {
      paint.setPointer(marks);
    },
    setPaintShown(on: boolean) {
      paint.setShown(on);
    },
    /** The grass takes its colour from the ground, so it regrows. */
    setFieldTextureShown(on: boolean) {
      if (terrain.setFieldTexture(on)) grass.regrow();
    },
    setTreesShown(on: boolean) {
      scenery.setTreesShown(on);
    },
    /** Draw the ground's classes in place of the lit world (the
     *  "ground-classes" frame view), or not. */
    setClassView(on: boolean) {
      classView = on;
      terrain.setClassView(on);
    },
    /** Draw the roads worn or plain; the grass on their shoulders regrows. */
    setRoadWearShown(on: boolean) {
      if (terrain.setRoadWear(on)) grass.regrow();
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
      frameProjection = viewProj;
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
        const raw = root.unwrap(pass);
        const modelCasters = modelCaster.with(pass).with(cameraGroup) as unknown as Bound;
        models.drawCasters(modelCasters, "solid");
        models.drawBuildingCasters(modelCasters, raw, "solid");
        const cutoutCasters = modelCasterCutout.with(pass).with(cameraGroup) as unknown as Bound;
        models.drawCasters(cutoutCasters, "cutout");
        models.drawBuildingCasters(cutoutCasters, raw, "cutout");
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
      models.drawBuildings(
        modelPrepass.with(scene).with(cameraGroup) as unknown as Bound,
        root.unwrap(scene),
        "solid",
      );
      models.drawBuildings(
        modelPrepassCutout.with(scene).with(cameraGroup) as unknown as Bound,
        root.unwrap(scene),
        "cutout",
      );
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

      const count = Math.max(1, models.drawnInstances);
      if (count > xrayCapacity) {
        xrayCapacity = Math.max(16, count * 2);
        xrayCounts.set(
          root.device.createBuffer({
            label: "model-xray-coverage",
            size: xrayCapacity * 8,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
          }),
        );
        xrayDepth = null;
      }
      raw.clearBuffer(xrayCounts.current!);
      if (xrayDepth !== targets.overlayDepth) {
        xrayDepth = targets.overlayDepth;
        xrayCountGroup = countGroupFor(targets.overlayDepth);
        xrayReadGroup = readGroupFor();
      }
      if (xrayCoverageEnabled && models.hasXrayMeshes) {
        // Preserve world-only depth before the units and grass add theirs.
        raw.copyTextureToTexture({ texture: targets.depth }, { texture: targets.overlayDepth }, [
          targets.width,
          targets.height,
        ]);
        const coverage = encoder.beginRenderPass({
          label: "unit-xray-coverage",
          colorAttachments: [
            {
              view: targets.overlayMsaa.createView(),
              loadOp: "load",
              storeOp: "discard",
            },
          ],
        });
        drawModels(
          modelXrayCount.with(coverage).with(cameraGroup).with(xrayCountGroup!),
          "opaque",
          "units",
        );
        coverage.end();
      }

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
      drawModels(modelXray.with(xray).with(cameraGroup).with(xrayReadGroup!), "opaque", "units");
      xray.end();

      const units = encoder.beginRenderPass({
        label: "depth-prepass-units",
        colorAttachments: [],
        depthStencilAttachment: { view: depthView, depthLoadOp: "load", depthStoreOp: "store" },
      });
      proxies.draw(prepass.with(units).with(cameraGroup));
      drawModels(modelPrepass.with(units).with(cameraGroup), "solid");
      drawModels(modelPrepassCutout.with(units).with(cameraGroup), "cutout");
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
      const grassBounds = grass.scarBounds();
      if (grassBounds) terrain.prepareScars(grassBounds);
      grass.encodeBuild(raw);
      const colorView = targets.hdrMsaa.createView();
      const litView = targets.lit.createView();
      // The sky resolves into `lit` too: where the world pass below draws
      // nothing (screen tiles of pure sky), Metal skips the tile, and its
      // resolve with it, so `lit` would keep an older frame's pixels there.
      environment.encodeBackground(raw, colorView, litView);
      const regions = terrain.scarRegions();
      let first = true;
      const fogGroups = fog.groups();
      if (regions.length) {
        // Queue writes to the shared cache must follow submission of its last
        // consumer. The same atlas is reused, never one resource per region.
        encoder.submit();
        for (const region of regions) {
          const rect = scissor(region, targets.width, targets.height);
          if (!rect) continue;
          terrain.prepareScars(region, true);
          const batch = root["~unstable"].createCommandEncoder({ label: "ground-region" });
          const groundPass = batch.beginRenderPass({
            label: "ground-region",
            colorAttachments: [
              { view: colorView, resolveTarget: litView, loadOp: "load", storeOp: "store" },
              {
                view: targets.fogMaskMsaa.createView(),
                resolveTarget: targets.fogMask.createView(),
                loadOp: first ? "clear" : "load",
                storeOp: "store",
                clearValue: [0, 0, 0, 0],
              },
            ],
            depthStencilAttachment: {
              view: targets.depth.createView(),
              depthLoadOp: "load",
              depthStoreOp: "store",
            },
          });
          groundPass.setScissorRect(...rect);
          world.ground.draw(
            terrainPipeline
              .with(groundPass)
              .with(cameraGroup)
              .with(environment.group)
              .with(fogGroups.paintedGround)
              .with(terrain.group),
          );
          groundPass.end();
          batch.submit();
          first = false;
        }
        encoder = root["~unstable"].createCommandEncoder({ label: "world-rest" });
        raw = root.unwrap(encoder);
      } else terrain.prepareScars();
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
            loadOp: first ? "clear" : "load",
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
      if (!regions.length)
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
      const cutoutsLit = modelCutout.with(pass).with(cameraGroup).with(environment.group);
      const roomsShown = modelRooms.with(pass).with(cameraGroup).with(environment.group);
      const cardsLit = modelCards.with(pass).with(cameraGroup).with(environment.group);
      for (const fog of ["units", "faces", "ground", "paintedFaces"] as const) {
        drawModels(modelsLit.with(fogGroups[fog]), "opaque", fog);
        drawModels(cutoutsLit.with(fogGroups[fog]), "cutout", fog);
        drawModels(roomsShown.with(fogGroups[fog]), "room", fog);
        drawCards(cardsLit.with(fogGroups[fog]), fog);
      }
      // Buildings are faces: an occluding structure takes fog whole.
      models.drawBuildings(
        modelsLit.with(fogGroups.faces) as unknown as Bound,
        root.unwrap(pass),
        "opaque",
      );
      models.drawBuildings(
        cutoutsLit.with(fogGroups.faces) as unknown as Bound,
        root.unwrap(pass),
        "cutout",
      );
      models.drawBuildings(
        roomsShown.with(fogGroups.faces) as unknown as Bound,
        root.unwrap(pass),
        "room",
      );
      scenery.encode(pass, cameraGroup, fogGroups.faces);
      // The class view reads the ground itself: nothing that grows on it or
      // lies blended over it.
      if (!classView) grass.draw(pass, cameraGroup, fogGroups.paintedGround);
      backdrop.draw(
        backdropPipeline
          .with(pass)
          .with(cameraGroup)
          .with(environment.group)
          .with(fogGroups.paintedGround)
          .with(terrain.group),
      );
      if (!classView)
        world.water.draw(
          water
            .with(pass)
            .with(cameraGroup)
            .with(environment.group)
            .with(fogGroups.paintedFaces)
            .with(terrain.group),
        );
      // Glass last, over everything: in the order the layer packs it, not by
      // depth (two panes of one glass blend nearly the same either way round).
      if (!classView) {
        const glassLit = modelBlended.with(pass).with(cameraGroup).with(environment.group);
        for (const fog of ["units", "faces", "ground", "paintedFaces"] as const)
          drawModels(glassLit.with(fogGroups[fog]), "blended", fog);
        models.drawBuildings(
          glassLit.with(fogGroups.faces) as unknown as Bound,
          root.unwrap(pass),
          "blended",
        );
      }
      pass.end();
      return { encoder, raw };
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
