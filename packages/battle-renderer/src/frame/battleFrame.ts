// The battle frame: one owner for the passes, their order and every GPU
// allocation they make. Each frame runs
//
//   shadows (4 cascades) → depth prepass (the world, the own units' x-ray
//   into the overlay target, the units; copied for the overlays) → fog
//   (moved eyes' horizon maps,
//   then the per-tile eye lists from that depth) → sky + HDR world (4× MSAA,
//   rgba16float, with FogTerm's mask beside it) → combat effects (into the
//   resolved world and its mask) → fog mask pass (distances,
//   then the unseen look, before post) → post (bloom, grade, AgX, into the
//   canvas) → fog rim (display space) → overlays (display space, against the
//   world's depth) → composite
//
// bracketed by two timestamp markers for the frame's GPU time. Rewritten here
// from reading ~/dev/game battle-renderer/src/world/frame.ts;
// that frame had no structures layer, overlay stage, registry or
// timing.
import { tgpu } from "typegpu";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/cameraUniform";
import { GPU_DEPTH_CLEAR, GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import { metresPerPxAt, type Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { BattleFrame, FrameView, SceneInstance, WorldLayers } from "../scene";
import { Camera, typegpuCameraLayout } from "../world/camera";
import { createTypegpuPost } from "../world/post";
import { frameCamera } from "../frameCamera";
import { battleWorldDepth } from "../worldDepth";
import type { LightPresentation } from "../light/sceneLight";
import type { FogGeometryPresentation } from "./fogInputs";
import type { FogStyle } from "./fogStyle";
import { createEnvironmentFrame } from "./environmentFrame";
import { GpuRegistry } from "./registry";
import { allocateFrameTargets, SizedTargets } from "./targets";
import { createWorldPass } from "./worldPass";
import { createFogMaskPass, type FogMaskViewKind } from "./fogMaskPass";
import { createOverlayPass, type OverlayGlowStyle } from "./overlayPass";
import type { PaintStyle } from "./paintedMarks";
import { createFrameTimer } from "./gpuTiming";
import { createEffectPass } from "../effects/effectPass";
import { createModelLayer, type CardAtlas } from "../models/modelLayer";
import { createImpostorBaker } from "../models/impostor";
import { CARD_SPEC } from "../models/impostorCards";
import type { ModelDetailPresentation } from "../models/modelDetail";
import type { BuildingStyle } from "../models/buildingReferences";
import { sunShadow } from "./staticChunks";
import { createDetailView, detailKey, setDetailView } from "./detailView";
import { createCastLightList, type CastLightList } from "../light/castLights";
import { EMPTY_MESH } from "../mesh";

const NO_LIGHTS = createCastLightList(0);

/** Post's five-level bloom needs at least this many pixels a side. */
const MIN_TARGET_PX = 64;

export interface BattleFrameOptions {
  /** `presentation.light`: sun, sky, haze, grade, bloom and cascades. */
  light: LightPresentation;
  /** `presentation.fog_geometry`: the sight lights' resolution and budgets. */
  fogGeometry: FogGeometryPresentation;
  /** The unseen look to start with (`presentation.fog`'s selected style). */
  fogStyle: FogStyle;
  /** The halo the display-space overlays carry. */
  overlayGlow: OverlayGlowStyle;
  /** How the painted ground marks look. */
  paint: PaintStyle;
  /** Minimum fraction of hidden rasterized surface fragments before a model receives its x-ray cue. */
  xrayMinHiddenFragmentFraction: number;
  /** `presentation.models`: the models' detail tiers and impostor size. */
  models: ModelDetailPresentation;
  /** `presentation.buildings`: the buildings' tiers, chunks and pool. */
  buildings: BuildingStyle;
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

/** Device pixels per metre at the camera's target. */
function pixelsPerMetre(camera: Camera3DParams, height: number): number {
  return 1 / metresPerPxAt(camera.distance, camera.fovY, height);
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
    const environment = await createEnvironmentFrame(device, registry, options.light);
    const models = await createModelLayer(root, registry, options.models, options.buildings);
    registry.adopt(() => models.dispose());
    const world = await createWorldPass(
      root,
      registry,
      environment,
      options.fogGeometry,
      models,
      options.paint,
      options.xrayMinHiddenFragmentFraction,
    );
    const fogMask = await createFogMaskPass(root, registry, displayFormat, options.fogStyle);
    const impostors = createImpostorBaker(root, registry, models, environment);
    const overlay = await createOverlayPass(root, registry, displayFormat, options.overlayGlow);
    const effects = await createEffectPass(device, registry, environment.raw);
    const cameraBuffer = root.unwrap(camera);
    const timer = createFrameTimer(device, registry);
    const targets = new SizedTargets(registry, async (scope, width, height) => {
      const t = allocateFrameTargets(scope, width, height);
      const post = await createTypegpuPost(
        device,
        t.hdr.createView(),
        width,
        height,
        displayFormat,
        environment.post,
      );
      scope.adopt(post.dispose);
      const fog = world.fogTiles(scope, width, height, t.depth, t.paint);
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
      let clock = 0;
      let disposed = false;
      let appearanceUpdates = Promise.resolve();
      /** The effects' lights, as the last `setEffects` batch cast them. */
      let lights: CastLightList = NO_LIGHTS;
      let lightsShown = true;
      const detailView = createDetailView();
      const shadow = { fall: [0, 0] as [number, number], reach: 0 };
      const sun = {
        azimuth: options.light.sun_azimuth,
        elevation: options.light.sun_elevation,
        maxFarM: options.light.cascades.max_far_m,
      };
      const rebuild = (width: number, height: number) =>
        targets.ensure(width, height).then(
          () => options.requestRedraw?.(),
          (error: unknown) => {
            if (!disposed) console.error("Battle frame targets failed", error);
          },
        );

      return {
        render(output: GPUTextureView, viewport: ViewportCamera) {
          if (disposed) return;
          const width = size(viewport.width);
          const height = size(viewport.height);
          const t = targets.current;
          if (!t || t.width !== width || t.height !== height) {
            void rebuild(width, height);
            return;
          }
          const camera3d = liveCamera(viewport);
          const state = frameCamera(
            {
              camera3d,
              width,
              height,
              x: camera3d.target[0],
              y: camera3d.target[1],
              zoom: pixelsPerMetre(camera3d, height),
              sunAzimuth: options.light.sun_azimuth,
              sunElevation: options.light.sun_elevation,
              time: clock,
            },
            width,
            height,
          );
          camera.write(state.bytes.buffer);
          const detail = setDetailView(detailView, camera3d, height);
          environment.setCastLights(lightsShown ? lights : NO_LIGHTS, detail.sides, camera3d);
          world.prepare(camera3d, state.view, state.viewProj, state.rays, height);
          models.prepare(detail, detailKey(camera3d, height), sunShadow(shadow, sun, camera3d));
          // Buildings expand into their pool a budget a frame: draw until done.
          if (models.buildingsPending) options.requestRedraw?.();

          let encoder = root["~unstable"].createCommandEncoder({ label: "battle-frame" });
          let raw = root.unwrap(encoder);
          timer?.begin(raw);
          models.encodePose(raw);
          world.encodeShadows(encoder);
          world.encodeDepth(encoder, raw, t, cameraGroup);
          world.encodeFog(raw, t.fog, state.bytes, width, height);
          ({ encoder, raw } = world.encode(encoder, raw, t, cameraGroup));
          const classes = view === "ground-classes";
          if (!classes) effects.encode(raw, t, t.effectGroup);
          fogMask.encode(raw, t, t.fogEdge);
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
          timer?.end(raw);
          encoder.submit();
          timer?.collect();
          frames++;
        },
        setWorld(next) {
          if (!disposed) world.setWorld(next);
        },
        setStructures(next) {
          if (!disposed) world.setStructures(next);
        },
        setBuildings(next) {
          if (!disposed) models.setBuildings(next);
        },
        setGround(next) {
          return !disposed && world.setGround(next);
        },
        setClock(seconds) {
          clock = seconds;
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
        setEffects(batch) {
          if (disposed) return;
          effects.set(batch);
          lights = batch.lights;
        },
        setInstances(next) {
          if (!disposed) world.setInstances(next);
        },
        setFog(next) {
          if (!disposed) world.setFog(next);
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
                which: card.which,
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
        setCorpses(next) {
          if (!disposed) models.setCorpses(next);
        },
        setTextureChannels(channels) {
          if (!disposed) models.setTextureChannels(channels);
        },
        readPalette: () => models.readPalette(),
        timePoseKernel: (reps, bodies) => models.timeKernel(reps, bodies),
        async bakeImpostor(appearance, spec) {
          const far = models.farPose(appearance);
          if (!far) throw new Error(`no installed appearance ${appearance}`);
          return impostors.bake(appearance, far.pose, far.bounds, spec);
        },
        setFogStyle(next) {
          if (!disposed) fogMask.setStyle(next);
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
        fogProbes: world.fog,
        grassProbes: world.grassProbes,
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
            worldVertices: passes.worldVertices,
            structures: passes.structures,
            depth: {
              format: GPU_DEPTH_FORMAT,
              clearValue: GPU_DEPTH_CLEAR,
              compare: battleWorldDepth("read-write").depthCompare!,
            },
            view,
            clock,
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
              lightsOffered: lights.count,
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
