// World orchestration: cascades, depth, x-ray, lit surfaces and separate high air.
// Terrain scar regions submit before rewriting the shared atlas. Ground-only
// depth precedes x-ray, then unit depth precedes color; fog and post compose later.
// Material, visibility and resource contracts live in the renderer README and
// the individual layer owners below.
import { createSubmittedFrame, type SubmittedDraws } from "./submitted";
import { SpatialMeshDraws } from "./spatialMeshDraws";
import { drawTerrainRegions } from "./terrainRegions";
import { tgpu, d, std, type TgpuCommandEncoder } from "typegpu";
import type { SceneInstance, WorldLayers } from "../scene";
import type { Mesh } from "../mesh";
import { typegpuCameraLayout } from "../world/camera";
import { battleWorldDepth, BATTLE_DEPTH_ATTACHMENT } from "../worldDepth";
import { CameraAt } from "../records.gen";
import type { EnvironmentFrame } from "./environmentFrame";
import { backdrop_mesh, map_box, scar_scissor } from "@wasm/presentation/presentation_wasm.js";
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
import { createGroundFragments, ROUGHNESS, srgbToLinear } from "./groundFragments";
import { createGrassPass } from "./grassPass";
import { createWater } from "./waterPass";
import { createGhostPass } from "./ghostPass";
import { createTerrainHeights } from "./terrainHeights";
import { createFogVisibility, type FogShaders, type FogTiles } from "./fogVisibility";
import { createTerrainSource } from "./terrainSource";
import { createSceneryLayer, type SceneryWalkStats } from "./sceneryLayer";
import type { PresentedGround } from "../terrain/terrainSurface";
import { createPaintedMarks, validatePaintStyle, type PaintStyle } from "./paintedMarks";
import type { GroundMarks, ScarRegion } from "./scarTexture";
import type { Mat4 } from "math";
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
  type WaterDetail,
} from "./targets";
import type { GpuRegistry } from "./registry";

type Root = ReturnType<typeof tgpu.initFromDevice>;

/** Selection glow added to a highlighted proxy. */
const HIGHLIGHT = [0.95, 0.8, 0.2] as const;

export async function createWorldPass(
  root: Root,
  registry: GpuRegistry,
  environment: EnvironmentFrame,
  fogGeometry: FogShaders,
  models: ModelLayer,
  paintStyle: PaintStyle,
  xrayMinHiddenFragmentFraction: number,
  glass: GlassStyle,
  grassDensity: number,
  detail: WaterDetail,
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
  const ground = createGroundFragments(environment);

  const base = {
    attribs: meshAttribs,
    vertex: meshVertex,
    primitive: { topology: "triangle-list", cullMode: "none" },
  } as const;
  // Depth only, with no fragment stage, so both keep early-Z.
  const prepass = root
    .createRenderPipeline({
      ...base,
      depthStencil: battleWorldDepth("read-write"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("worldPrepass");
  const caster = root
    .createRenderPipeline({
      ...base,
      depthStencil: battleWorldDepth("read-write"),
    })
    .$name("worldCaster");
  const opaque = root
    .createRenderPipeline({
      ...base,
      fragment: worldFragment,
      targets: worldTargets(),
      depthStencil: battleWorldDepth("prepassed"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("worldOpaque");
  const water = createWater(root, environment);
  const terrainPipeline = root
    .createRenderPipeline({
      ...base,
      fragment: ground.terrain,
      targets: worldTargets(),
      depthStencil: battleWorldDepth("prepassed"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("worldTerrain");
  const backdropPipeline = root
    .createRenderPipeline({
      ...base,
      fragment: ground.backdrop,
      targets: worldTargets(),
      depthStencil: battleWorldDepth("prepassed"),
      multisample: { count: FRAME_MSAA },
    })
    .$name("worldBackdrop");
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
  // Glass is drawn after everything opaque, over it by its coverage: it
  // reads depth and writes none, is in no prepass and casts no shadow. It
  // leaves the fog mask alone: a pane is not the thing seen or unseen.
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
  const ghosts = createGhostPass(root, environment, models);
  // The x-ray: a unit's fragments behind the world's depth (without the
  // units), over the transparent overlay target. Max blending, so a hidden
  // arm behind a hidden torso never doubles the silhouette's alpha. The
  // bias pulls each fragment toward the eye by about 1/128 of its distance
  // (float reverse-Z: a constant bias of 2^16 is ~2^-7 of the depth), so a
  // body touching the ground (a prone man, a track's lower run) is not
  // x-rayed where it dips under the surface; a canopy or a wall hides by
  // metres.
  // Allocate the full uniform block explicitly: SwiftShader rejects the
  // mapped four-byte buffer TypeGPU can otherwise create for this scalar.
  const xrayMinimum = registry.own(
    registry.device.createBuffer({
      label: "xray-minimum",
      size: 16,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    }),
  );
  // The configured fraction is checked once; 0 turns the coverage test off.
  const xrayFraction = validateXrayFraction(xrayMinHiddenFragmentFraction);
  const writeXrayMinimum = (value: number) =>
    registry.device.queue.writeBuffer(xrayMinimum, 0, new Float32Array([value]));
  writeXrayMinimum(xrayFraction);
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
      ...water.pipelines,
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
      ghosts.pipeline,
      modelXray,
      modelXrayCount,
      modelCards,
    ].map((pipeline) => pipeline.initAsync()),
  );
  const submitted = createSubmittedFrame();
  /** The models layer's draws take the pipelines' binding methods as they are. */
  type Bound = Parameters<ModelLayer["draw"]>[0];
  const drawModels = (
    bound: unknown,
    surface: Parameters<ModelLayer["draw"]>[1],
    fog?: Parameters<ModelLayer["draw"]>[2],
    high?: boolean,
    counter?: SubmittedDraws,
  ) => models.draw(bound as Bound, surface, fog, high, counter);
  const drawCards = (bound: unknown, fog: Parameters<ModelLayer["drawCards"]>[1]) =>
    models.drawCards(
      bound as Parameters<ModelLayer["drawCards"]>[0],
      fog,
      submitted.color.modelsIncludingProps,
    );

  const terrainHeights = createTerrainHeights(registry);
  const fog = await createFogVisibility(root, registry, fogGeometry, terrainHeights);
  const scenery = await createSceneryLayer(root, registry, environment);
  const terrain = createTerrainSource(root, registry, detail);
  const grass = await createGrassPass(
    root,
    registry,
    environment,
    terrain,
    terrainHeights,
    grassDensity,
  );
  const paint = createPaintedMarks(root, registry);
  await paint.ready();
  fog.setPaintStyle(validatePaintStyle(paintStyle));
  const identity = identityInstance(root, registry);
  const spatialMesh = () => new SpatialMeshDraws(new MeshSlot(root, registry, identity), registry);
  const world = { ground: spatialMesh(), props: spatialMesh(), water: spatialMesh() };
  const backdrop = new MeshSlot(root, registry, identity);
  const proxies = new ProxyInstances(root, registry);
  /** The map's box (`map_box`): what a scar's scissor projects. */
  let box: Float64Array | undefined;
  let frameProjection: Float32Array | null = null;
  /** The pixels a scar region can cover (`scar_scissor`), none off screen. */
  const scissor = (
    r: ScarRegion,
    width: number,
    height: number,
  ): readonly [number, number, number, number] | null => {
    if (!frameProjection) return [0, 0, width, height];
    const rect = scar_scissor(new Float64Array(r), box, frameProjection, width, height);
    return rect.length ? [rect[0], rect[1], rect[2], rect[3]] : null;
  };

  let classView = false;
  const NO_WATER = new Float32Array(0);
  // Material, grass and water consume both layers and presented ground.
  let layers: WorldLayers | null = null;
  let presented: PresentedGround | null = null;
  function setGround() {
    const ground = layers ? presented : null;
    terrain.set(ground);
    fog.setSource(ground, layers?.terrain.grid ?? null);
    grass.setWorld(layers?.terrain ?? null, layers?.grass ?? null, ground);
    world.water.set(
      ground && layers?.terrain.water ? (ground.ground_water_mesh() as Mesh) : NO_WATER,
    );
  }

  return {
    setXrayCoverageEnabled(on: boolean) {
      xrayCoverageEnabled = on;
      writeXrayMinimum(on ? xrayFraction : 0);
    },
    setWorld(next: WorldLayers) {
      world.ground.set(next.terrain.mesh);
      world.props.set(next.props);
      layers = next;
      scenery.set(next.scenery);
      setGround();
      box = map_box(next.terrain.mesh);
      const surround = next.backdropTerrain ? map_box(next.backdropTerrain) : box;
      if (surround)
        backdrop.set(backdrop_mesh(surround, environment.light.backdrop.reach_m) as Mesh);
    },
    /** The ground the scene's presentation resolved: its tables, its grass
     *  and its water. */
    setPresentedGround(next: PresentedGround | null) {
      if (next === presented) return;
      presented = next;
      setGround();
    },
    /** Upload learned ground changes and regrow grass. */
    setGround(next: GroundMarks | null): boolean {
      const changed = terrain.setGround(next);
      if (changed) grass.regrow();
      return changed;
    },
    /** The scenery's static batches (`BattleFrame.setStaticFrame`). */
    setStaticFrame(frame: Parameters<typeof scenery.setFrame>[0], stats: SceneryWalkStats) {
      return scenery.setFrame(frame, stats);
    },
    setSceneryWorld(rows: Parameters<typeof scenery.setWorld>[0]) {
      scenery.setWorld(rows);
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
    drawnScenery: () => scenery.drawn(),
    prepare(
      camera: Float32Array,
      view: Float32Array,
      sunShadow: Float32Array,
      casters: Float32Array,
      count: number,
    ) {
      frameProjection = camera.subarray(CameraAt.view_proj, CameraAt.view_proj + 16);
      environment.prepare(view, sunShadow, casters, count);
      world.ground.prepare(camera, casters, count);
      world.props.prepare(camera, casters, count);
      world.water.prepare(camera, casters, 0);
      grass.prepare(frameProjection as unknown as Mat4);
    },
    encodeShadows(encoder: TgpuCommandEncoder) {
      submitted.reset();
      scenery.beginFrame();
      environment.encodeShadows(encoder, (pass, cameraGroup, cascade) => {
        const counts = submitted.cascade(cascade);
        const bound = caster.with(pass).with(cameraGroup);
        world.ground.draw(bound, counts.terrain, cascade);
        world.props.draw(bound, counts.skirtAndDebugProps, cascade);
        proxies.draw(bound);
        const raw = root.unwrap(pass);
        const modelCasters = modelCaster.with(pass).with(cameraGroup) as unknown as Bound;
        models.drawCasters(modelCasters, "solid", cascade, counts.modelsIncludingProps);
        models.drawBuildingCasters(modelCasters, raw, "solid", cascade, counts.buildings);
        const cutoutCasters = modelCasterCutout.with(pass).with(cameraGroup) as unknown as Bound;
        models.drawCasters(cutoutCasters, "cutout", cascade, counts.modelsIncludingProps);
        models.drawBuildingCasters(cutoutCasters, raw, "cutout", cascade, counts.buildings);
        scenery.encodeShadows(pass, cameraGroup, cascade, counts.scenery);
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
      world.ground.draw(bound, submitted.depth.terrain);
      world.props.draw(bound, submitted.depth.skirtAndDebugProps);
      backdrop.draw(bound, submitted.depth.backdrop);
      scenery.encodeDepth(scene, cameraGroup, submitted.depth.scenery);
      models.drawBuildings(
        modelPrepass.with(scene).with(cameraGroup) as unknown as Bound,
        root.unwrap(scene),
        "solid",
        submitted.depth.buildings,
      );
      models.drawBuildings(
        modelPrepassCutout.with(scene).with(cameraGroup) as unknown as Bound,
        root.unwrap(scene),
        "cutout",
        submitted.depth.buildings,
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
      drawModels(
        modelPrepass.with(units).with(cameraGroup),
        "solid",
        undefined,
        false,
        submitted.depth.modelsIncludingProps,
      );
      drawModels(
        modelPrepassCutout.with(units).with(cameraGroup),
        "cutout",
        undefined,
        false,
        submitted.depth.modelsIncludingProps,
      );
      units.end();
      raw.copyTextureToTexture({ texture: targets.depth }, { texture: targets.overlayDepth }, [
        targets.width,
        targets.height,
      ]);
    },
    /** Lit world and glass, then the water, with their fog coverage alongside. */
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
        // The shared scar atlas cannot change before its last consumer submits.
        encoder.submit();
        first = drawTerrainRegions(
          root,
          targets,
          regions,
          scissor,
          (region) => terrain.prepareScars(region, true),
          (groundPass) =>
            world.ground.draw(
              terrainPipeline
                .with(groundPass)
                .with(cameraGroup)
                .with(environment.group)
                .with(fogGroups.paintedGround)
                .with(terrain.group),
              submitted.color.terrain,
            ),
        );
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
            storeOp: "store",
          },
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
          submitted.color.terrain,
        );
      const faces = opaque
        .with(pass)
        .with(cameraGroup)
        .with(environment.group)
        .with(fogGroups.faces);
      world.props.draw(faces, submitted.color.skirtAndDebugProps);
      proxies.draw(
        opaque.with(pass).with(cameraGroup).with(environment.group).with(fogGroups.units),
      );
      // The shared model_fog rule routes each appearance to its fog group.
      const modelsLit = modelOpaque.with(pass).with(cameraGroup).with(environment.group);
      const cutoutsLit = modelCutout.with(pass).with(cameraGroup).with(environment.group);
      const roomsShown = modelRooms.with(pass).with(cameraGroup).with(environment.group);
      const cardsLit = modelCards.with(pass).with(cameraGroup).with(environment.group);
      for (const fog of ["units", "faces", "ground", "paintedFaces"] as const) {
        drawModels(
          modelsLit.with(fogGroups[fog]),
          "opaque",
          fog,
          false,
          submitted.color.modelsIncludingProps,
        );
        drawModels(
          cutoutsLit.with(fogGroups[fog]),
          "cutout",
          fog,
          false,
          submitted.color.modelsIncludingProps,
        );
        drawModels(
          roomsShown.with(fogGroups[fog]),
          "room",
          fog,
          false,
          submitted.color.modelsIncludingProps,
        );
        drawCards(cardsLit.with(fogGroups[fog]), fog);
      }
      // Buildings are faces: an occluding structure takes fog whole.
      models.drawBuildings(
        modelsLit.with(fogGroups.faces) as unknown as Bound,
        root.unwrap(pass),
        "opaque",
        submitted.color.buildings,
      );
      models.drawBuildings(
        cutoutsLit.with(fogGroups.faces) as unknown as Bound,
        root.unwrap(pass),
        "cutout",
        submitted.color.buildings,
      );
      models.drawBuildings(
        roomsShown.with(fogGroups.faces) as unknown as Bound,
        root.unwrap(pass),
        "room",
        submitted.color.buildings,
      );
      scenery.encode(pass, cameraGroup, fogGroups.faces, submitted.color.scenery);
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
        submitted.color.backdrop,
      );
      // Glass last, over everything: in the order the layer packs it, not by
      // depth (two panes of one glass blend nearly the same either way round).
      if (!classView) {
        const glassLit = modelBlended.with(pass).with(cameraGroup).with(environment.group);
        for (const fog of ["units", "faces", "ground", "paintedFaces"] as const)
          drawModels(
            glassLit.with(fogGroups[fog]),
            "blended",
            fog,
            false,
            submitted.color.modelsIncludingProps,
          );
        models.drawBuildings(
          glassLit.with(fogGroups.faces) as unknown as Bound,
          root.unwrap(pass),
          "blended",
          submitted.color.buildings,
        );
      }
      pass.end();
      if (!classView)
        water.encode(
          encoder,
          targets,
          world.water,
          [cameraGroup, fogGroups.water, terrain.group],
          submitted.color.water,
        );
      return { encoder, raw };
    },
    /** The high-air models on their own plane (`ModelLayer.setHighApart`):
     *  over a transparent clear, in a depth of their own, so they hide only
     *  each other, lit as the world's models are; resolved into `hdr`, which
     *  post has made its look from by now, for post's high-plane pass. The
     *  world's multisampled colour, fog mask and depth are spent too, and
     *  serve again. Nothing of the world's frame is read. */
    encodeHigh(encoder: TgpuCommandEncoder, targets: FrameTargets, cameraGroup: CameraGroup) {
      const depthView = targets.depth.createView();
      const depth = encoder.beginRenderPass({
        label: "high-air-depth",
        colorAttachments: [],
        depthStencilAttachment: {
          view: depthView,
          depthClearValue: BATTLE_DEPTH_ATTACHMENT.clearValue,
          depthLoadOp: "clear",
          depthStoreOp: "store",
        },
      });
      drawModels(
        modelPrepass.with(depth).with(cameraGroup),
        "solid",
        undefined,
        true,
        submitted.highDepth.modelsIncludingProps,
      );
      drawModels(
        modelPrepassCutout.with(depth).with(cameraGroup),
        "cutout",
        undefined,
        true,
        submitted.highDepth.modelsIncludingProps,
      );
      depth.end();
      const pass = encoder.beginRenderPass({
        label: "high-air",
        colorAttachments: [
          {
            view: targets.hdrMsaa.createView(),
            resolveTarget: targets.hdr.createView(),
            loadOp: "clear",
            storeOp: "discard",
            clearValue: [0, 0, 0, 0],
          },
          {
            view: targets.fogMaskMsaa.createView(),
            loadOp: "clear",
            storeOp: "discard",
            clearValue: [0, 0, 0, 0],
          },
        ],
        depthStencilAttachment: { view: depthView, depthLoadOp: "load", depthStoreOp: "discard" },
      });
      const fogGroups = fog.groups();
      const modelsLit = modelOpaque.with(pass).with(cameraGroup).with(environment.group);
      const cutoutsLit = modelCutout.with(pass).with(cameraGroup).with(environment.group);
      const roomsShown = modelRooms.with(pass).with(cameraGroup).with(environment.group);
      const glassLit = modelBlended.with(pass).with(cameraGroup).with(environment.group);
      for (const fog of ["units", "faces", "ground", "paintedFaces"] as const) {
        drawModels(
          modelsLit.with(fogGroups[fog]),
          "opaque",
          fog,
          true,
          submitted.highColor.modelsIncludingProps,
        );
        drawModels(
          cutoutsLit.with(fogGroups[fog]),
          "cutout",
          fog,
          true,
          submitted.highColor.modelsIncludingProps,
        );
        drawModels(
          roomsShown.with(fogGroups[fog]),
          "room",
          fog,
          true,
          submitted.highColor.modelsIncludingProps,
        );
      }
      for (const fog of ["units", "faces", "ground", "paintedFaces"] as const)
        drawModels(
          glassLit.with(fogGroups[fog]),
          "blended",
          fog,
          true,
          submitted.highColor.modelsIncludingProps,
        );
      pass.end();
    },
    encodeGhosts: ghosts.encode,
    stats() {
      return {
        submitted: submitted.snapshot(),
        terrainVisitedNodes: world.ground.visited,
        skirtVisitedNodes: world.props.visited,
        waterVisitedNodes: world.water.visited,
        worldVertices: world.ground.vertices + world.props.vertices + world.water.vertices,
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
