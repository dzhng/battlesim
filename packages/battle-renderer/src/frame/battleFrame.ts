// The battle frame: one owner for the passes, their order and every GPU
// allocation they make. Each frame runs
//
//   shadows (4 cascades) → depth prepass (the world, the own units' x-ray
//   into the overlay target, the units; copied for the overlays) → fog
//   (moved eyes' horizon maps,
//   then the per-tile eye lists from that depth) → sky + HDR world (4× MSAA,
//   rgba16float, with FogTerm's mask beside it, glass last; samples kept) →
//   water (over those samples, resolved into the world and its mask) →
//   combat effects (into the resolved world and its mask) → fog mask pass
//   (distances,
//   then the unseen look, before post) → post (bloom, grade, AgX, into the
//   canvas) → fog rim (display space) → overlays (display space, against the
//   world's depth) → composite → the high-air plane (its models alone, lit
//   as the world's, then post's look premultiplied, into the high target),
//   when the caller gives one
//
// bracketed by two timestamp markers for the frame's GPU time. Rewritten here
// from reading ~/dev/game battle-renderer/src/world/frame.ts;
// that frame had no structures layer, overlay stage, registry or
// timing.
import { tgpu } from "typegpu";
import { GPU_DEPTH_CLEAR, GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import type { BattleFrame, FrameView, SceneInstance, WorldLayers } from "../scene";
import { typegpuCameraLayout } from "../world/camera";
import { createTypegpuPost } from "../world/post";
import { battleWorldDepth } from "../worldDepth";
import type { LightPresentation, ResolvedLight } from "../light/sceneLight";
import type { FogShaders } from "./fogVisibility";
import { createEnvironmentFrame } from "./environmentFrame";
import { GpuRegistry } from "./registry";
import { allocateFrameTargets, SizedTargets, type WaterDetail } from "./targets";
import { createWorldPass } from "./worldPass";
import { createWaveField } from "./waveField";
import { createFogMaskPass, type FogMaskViewKind } from "./fogMaskPass";
import { createOverlayPass, type OverlayGlowStyle } from "./overlayPass";
import type { PaintStyle } from "./paintedMarks";
import { createFrameTimer } from "./gpuTiming";
import { createEffectPass } from "../effects/effectPass";
import { createModelLayer, type CardAtlas } from "../models/modelLayer";
import type { GlassStyle } from "../models/surfaceFragments";
import { createImpostorBaker } from "../models/impostor";
import { CARD_SPEC } from "../models/impostorCards";
import { Camera, CAMERA_WORDS, CameraAt, FrameHeader, VIEW_WORDS } from "../records.gen";
import { EMPTY_MESH } from "../mesh";

const EMPTY = new Float32Array(0);
/** Post's five-level bloom needs at least this many pixels a side. */
export const MIN_TARGET_PX = 64;

export interface BattleFrameOptions {
  /** Optional resolved graphics quality; omitted preserves authored High.
   *  The units', buildings' and scenery's detail is the presentation's
   *  (`setDetailScale`, `setSceneryQuality`): Rust tiers them. */
  quality?: {
    grassDensity: number;
    shadowMapSize: 512 | 1024 | 2048;
    shadowReach: number;
    waterDetail: WaterDetail;
  };
  /** `presentation.light`: sun, sky, haze, grade, bloom and cascades. */
  light: ResolvedLight;
  /** The sight lights' pipeline text constants (`presentation.fog_geometry`);
   *  the rest of the geometry is the presentation's. */
  fogGeometry: FogShaders;
  /** The unseen look to start with: `presentation.fog`'s selected style as
   *  the presentation crate resolves it (a `FogLook` record). */
  fogLook: Float32Array;
  /** The halo the display-space overlays carry. */
  overlayGlow: OverlayGlowStyle;
  /** How the painted ground marks look. */
  paint: PaintStyle;
  /** Minimum fraction of hidden rasterized surface fragments before a model receives its x-ray cue. */
  xrayMinHiddenFragmentFraction: number;
  /** `presentation.models` as JSON: the tiers Rust gives a lab's own models. */
  models: string;
  /** `presentation.glass`: how a pane of glass takes light. */
  glass: GlassStyle;
  world: WorldLayers;
  instances: readonly SceneInstance[];
  /** The viewport's size in device pixels: targets are built for it up front. */
  width: number;
  height: number;
  /** Called when the frame should be drawn again with nothing new handed
   *  to it: targets for a new size finished building, or buildings still
   *  wait their turn to be expanded. */
  requestRedraw?: () => void;
}

/** The frame views the fog mask pass draws in place of the look. */
const MASK_OF_VIEW: Partial<Record<FrameView, FogMaskViewKind>> = {
  "fog-mask": "fog",
  "ground-mask": "ground",
  "ground-classes": "classes",
};

/** The last static generation any frame took: each frame's is new. */
let generations = 0;

/** A graphics quality's cascades: its shadow map size and reach over the
 *  authored light's. The battle frame draws them, and the presentation
 *  fits them (`ScenePresentation.setShadow`). */
export function cascadeLight(
  light: LightPresentation,
  quality?: BattleFrameOptions["quality"],
): LightPresentation {
  if (!quality) return light;
  return {
    ...light,
    cascades: {
      ...light.cascades,
      map_size: quality.shadowMapSize,
      max_far_m: quality.shadowReach,
    },
  };
}

/** Caller owns the device and canvas; the frame owns every allocation it makes. */
export async function createBattleFrame(
  device: GPUDevice,
  displayFormat: GPUTextureFormat,
  options: BattleFrameOptions,
): Promise<BattleFrame> {
  const registry = new GpuRegistry(device);
  try {
    const root = tgpu.initFromDevice({ device });
    registry.adopt(() => root.destroy());
    const camera = registry.own(root.createBuffer(Camera).$usage("uniform"));
    const cameraGroup = root.createBindGroup(typegpuCameraLayout, { cam: camera });
    const quality = options.quality;
    const water = quality?.waterDetail ?? "full";
    const waveField = createWaveField(root, registry);
    await waveField.pipeline.initAsync();
    /** The wave field's steps the last frame handed over, not yet run. */
    let waves: Float32Array | null = null;
    let wavesTaken: Float32Array | undefined;
    const effectiveLight = {
      ...options.light,
      settings: cascadeLight(options.light.settings, quality),
    };
    const environment = await createEnvironmentFrame(device, registry, effectiveLight);
    const models = await createModelLayer(root, registry, options.models);
    registry.adopt(() => models.dispose());
    const world = await createWorldPass(
      root,
      registry,
      environment,
      options.fogGeometry,
      models,
      options.paint,
      options.xrayMinHiddenFragmentFraction,
      options.glass,
      quality?.grassDensity ?? 1,
      water,
    );
    const fogMask = await createFogMaskPass(root, registry, displayFormat, options.fogLook);
    const impostors = createImpostorBaker(root, registry, models, environment);
    const overlay = await createOverlayPass(root, registry, displayFormat, options.overlayGlow);
    const effects = await createEffectPass(device, registry, environment.raw);
    const cameraBuffer = root.unwrap(camera);
    const timer = createFrameTimer(device, registry);
    const targets = new SizedTargets(registry, async (scope, width, height) => {
      const t = allocateFrameTargets(scope, width, height, water);
      const post = await createTypegpuPost(
        device,
        t.hdr.createView(),
        width,
        height,
        displayFormat,
        effectiveLight,
      );
      scope.adopt(post.dispose);
      const fog = world.fogTiles(
        scope,
        t.depth,
        t.paint,
        t.sceneColour,
        t.fogMaskUnder,
        waveField.view,
      );
      return {
        ...t,
        post,
        fog,
        fogEdge: fogMask.groupsFor(t),
        overlaySource: overlay.sourceFor(t),
        effectGroup: effects.groupFor(t, cameraBuffer),
      };
    });
    const size = (px: number) => Math.max(MIN_TARGET_PX, Math.floor(px));
    await targets.ensure(size(options.width), size(options.height));
    world.setWorld(options.world);
    world.setInstances(options.instances);
    return frameOf();

    function frameOf(): BattleFrame {
      let view: FrameView = "final";
      let frames = 0;
      let disposed = false;
      /** The presentation frame's camera, as `setCamera` copied it: its
       *  view, its uniform and the sun's cascades fitted to it. None before
       *  the first. */
      const viewRecord = new Float32Array(VIEW_WORDS);
      const cameraRecord = new Float32Array(CAMERA_WORDS);
      let sunShadow = new Float32Array(0);
      let casters = new Float32Array(0);
      let casterCount = 0;
      let framed = false;
      let appearanceUpdates = Promise.resolve();
      /** The cast lights, as the last presentation frame chose them. */
      let lights = new Float32Array(0);
      let lightCount = 0;
      let lightWrap = 0;
      let lightsShown = true;
      /** What the static batches held were sent for: new now, and with each
       *  new presentation world (which drops them). */
      let staticGeneration = ++generations;
      const rebuild = (width: number, height: number) =>
        targets.ensure(width, height).then(
          () => options.requestRedraw?.(),
          (error: unknown) => {
            if (!disposed) console.error("Battle frame targets failed", error);
          },
        );

      return {
        get staticGeneration() {
          return staticGeneration;
        },
        render(output: GPUTextureView, high: GPUTextureView | null = null) {
          if (disposed || !framed) return;
          const width = cameraRecord[CameraAt.width];
          const height = cameraRecord[CameraAt.height];
          const t = targets.current;
          if (!t || t.width !== width || t.height !== height) {
            void rebuild(width, height);
            return;
          }
          camera.write(cameraRecord.buffer);
          // The final look draws the high-air models on their own plane when
          // given one; every other view keeps them in the world.
          const apart = high !== null && view === "final";
          models.setHighApart(apart);
          environment.setCastLights(lights, lightsShown ? lightCount : 0, lightWrap);
          world.prepare(cameraRecord, viewRecord, sunShadow, casters, casterCount);
          models.prepare(viewRecord, viewRecord.join());

          let encoder = root["~unstable"].createCommandEncoder({ label: "battle-frame" });
          let raw = root.unwrap(encoder);
          timer?.begin(raw);
          world.encodeShadows(encoder);
          world.encodeDepth(encoder, raw, t, cameraGroup);
          world.encodeFog(raw, t.fog, cameraRecord, width, height);
          // The water keeps its depth colour alone on Low: no wakes.
          if (waves && water === "full") waveField.encode(encoder, waves);
          waves = null;
          ({ encoder, raw } = world.encode(encoder, raw, t, cameraGroup));
          const classes = view === "ground-classes";
          if (!classes) effects.encode(raw, t, t.effectGroup);
          fogMask.encode(raw, t, t.fogEdge);
          if (view === "final" || view === "world") world.encodeGhosts(encoder, t, cameraGroup);
          const maskView = view === "fog-mask" || view === "ground-mask";
          const worldOnly = view === "world" || maskView || classes;
          if (view === "final" || worldOnly) {
            // The masks skip bloom and grade: white stays white, black black.
            // The classes skip the tone map too: their bytes are data.
            t.post.encode(raw, output, classes ? "raw" : maskView ? "ungraded" : "look");
            fogMask.encodeRim(raw, t.fogEdge, output);
          } else {
            const v = view === "overlays-on-white" ? 1 : 0;
            raw
              .beginRenderPass({
                label: "inspector-backdrop",
                colorAttachments: [
                  { view: output, loadOp: "clear", storeOp: "store", clearValue: [v, v, v, 1] },
                ],
              })
              .end();
          }
          // Always composited: the x-ray is in the overlay target even when
          // no overlay mesh is.
          if (view === "paint") overlay.encodePaint(raw, t.overlaySource, output);
          else if (!worldOnly) overlay.encode(raw, t, t.overlaySource, cameraGroup, output);
          if (high && apart && models.hasHighMeshes) {
            world.encodeHigh(encoder, t, cameraGroup);
            t.post.encodeHighPlane(raw, high);
          } else if (high)
            raw
              .beginRenderPass({
                label: "high-air-clear",
                colorAttachments: [
                  { view: high, loadOp: "clear", storeOp: "store", clearValue: [0, 0, 0, 0] },
                ],
              })
              .end();
          timer?.end(raw);
          encoder.submit();
          timer?.collect();
          frames++;
        },
        setWorld(next) {
          if (!disposed) world.setWorld(next);
        },
        setStaticFrame(frame, stats) {
          if (disposed) return false;
          const buildings = models.setStaticFrame(frame, stats.buildings);
          return world.setStaticFrame(frame, stats.scenery) || buildings;
        },
        setPresentedGround(ground) {
          if (disposed) return;
          world.setPresentedGround(ground);
        },
        setGround(next) {
          return !disposed && world.setGround(next);
        },
        setCamera(frame) {
          if (disposed) return;
          // Copied: the views last only until the next call into the module.
          viewRecord.set(frame.view.f32);
          cameraRecord.set(frame.camera.f32);
          // Each frame's wave steps run once, however often it is handed over.
          if (frame.waves && frame.waves !== wavesTaken) waves = wavesTaken = frame.waves;
          if (frame.sun_shadow.count) {
            sunShadow = Float32Array.from(frame.sun_shadow.f32);
            casters = Float32Array.from(frame.casters.f32);
            casterCount = frame.casters.count;
          }
          framed = true;
        },
        setOverlay(next) {
          if (!disposed) {
            overlay.set(next);
            world.setPainted(next.painted ?? EMPTY_MESH, next.paintedMarching ?? EMPTY_MESH);
          }
        },
        setPointerMarks(marks) {
          if (!disposed) world.setPointerPaint(marks);
        },
        setFrame(frame, shown = { models: true, effects: true }) {
          if (disposed) return;
          models.setFrame(shown.models ? frame : null);
          const fx = shown.effects ? frame : null;
          effects.set(fx?.effects.f32 ?? EMPTY, fx?.effects.count ?? 0);
          // Copied: the views last only until the next call into the module.
          lightCount = fx?.lights.count ?? 0;
          if (lights.length < lightCount * 8) lights = new Float32Array(lightCount * 8);
          if (fx) lights.set(fx.lights.f32);
          lightWrap = frame
            ? new Float32Array(frame.header.buffer, frame.header.byteOffset)[FrameHeader.light_wrap]
            : 0;
        },
        setPresentationWorld(rows) {
          if (disposed) return;
          staticGeneration = ++generations;
          models.setWorld(rows);
          world.setSceneryWorld(rows);
        },
        setInstances(next) {
          if (!disposed) world.setInstances(next);
        },
        setAppearances(next) {
          // Installation and card baking share model buffers: finish the whole
          // update before another appearance can replace them.
          const done = appearanceUpdates.then(async () => {
            if (disposed) return;
            await models.setAppearances(next);
            // The battle carries every body's far-pose and corpse impostors,
            // baked here by the frame's own model path.
            const atlases: CardAtlas[] = [];
            const started = performance.now();
            for (const card of models.cardPoses()) {
              if (disposed) return;
              atlases.push({
                pose: card.pose,
                atlas: await impostors.bake(card.appearance, card.pose, card.bounds, CARD_SPEC),
              });
            }
            if (!disposed) models.setCards(atlases, performance.now() - started);
          });
          appearanceUpdates = done.catch(() => {});
          return done;
        },
        setModels(next) {
          if (!disposed) models.setModels(next);
        },
        setTextureChannels(channels) {
          if (!disposed) models.setTextureChannels(channels);
        },
        async bakeImpostor(appearance, spec) {
          const far = models.farPose(appearance);
          if (!far) throw new Error(`no installed appearance ${appearance}`);
          return impostors.bake(appearance, far.pose, far.bounds, spec);
        },
        setFogLook(next) {
          if (!disposed) fogMask.setLook(next);
        },
        setCastLightsShown(on) {
          lightsShown = on;
        },
        setXrayCoverageEnabled(on) {
          if (!disposed) world.setXrayCoverageEnabled(on);
        },
        setPaintShown(on) {
          if (!disposed) world.setPaintShown(on);
        },
        setFieldTextureShown(on) {
          if (!disposed) world.setFieldTextureShown(on);
        },
        setTreesShown(on) {
          if (!disposed) world.setTreesShown(on);
        },
        setBuildingsShown(on) {
          if (!disposed) models.setBuildingsShown(on);
        },
        setSurfaceShown(surface, on) {
          if (!disposed) models.setSurfaceShown(surface, on);
        },
        setRoadWearShown(on) {
          if (!disposed) world.setRoadWearShown(on);
        },
        setOverlayGlow(next) {
          if (!disposed) overlay.setGlow(next);
        },
        setView(next) {
          view = next;
          fogMask.setMaskView(MASK_OF_VIEW[next] ?? "none");
          world.setClassView(next === "ground-classes");
          timer?.reset();
        },
        settled: () => targets.settled(),
        resetGpuTiming() {
          timer?.reset();
        },
        fogProbes: world.fog,
        grassProbes: world.grassProbes,
        drawnInstances: () => ({
          buildings: models.drawnBuildings(),
          scenery: world.drawnScenery(),
          props: models.drawnProps(),
        }),
        stats() {
          const t = targets.current;
          const passes = world.stats();
          return {
            width: t?.width ?? 0,
            height: t?.height ?? 0,
            frames,
            instances: passes.instances,
            models: models.stats(),
            buildings: models.buildingStats(),
            submitted: passes.submitted,
            terrainVisitedNodes: passes.terrainVisitedNodes,
            skirtVisitedNodes: passes.skirtVisitedNodes,
            waterVisitedNodes: passes.waterVisitedNodes,
            worldVertices: passes.worldVertices,
            depth: {
              format: GPU_DEPTH_FORMAT,
              clearValue: GPU_DEPTH_CLEAR,
              compare: battleWorldDepth("read-write").depthCompare!,
            },
            view,
            clock: cameraRecord[CameraAt.time],
            gpu: timer?.stats() ?? null,
            memory: registry.stats(),
            shadow: passes.shadow,
            fog: passes.fog,
            fogEdge: fogMask.stats(),
            overlay: overlay.stats(),
            scenery: passes.scenery,
            grass: passes.grass,
            effects: {
              ...effects.stats(),
              lights: environment.stats().castLights,
            },
            scars: passes.scars,
          };
        },
        dispose() {
          if (disposed) return;
          disposed = true;
          registry.release();
        },
      };
    }
  } catch (error) {
    registry.release();
    throw error;
  }
}
