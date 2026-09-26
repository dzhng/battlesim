// The battle frame: one owner for the passes, their order and every GPU
// allocation they make. Each frame runs
//
//   shadows (4 cascades) → depth prepass → fog (moved eyes' horizon maps,
//   then the per-tile eye lists from that depth) → sky + HDR world (4× MSAA,
//   rgba16float, with FogTerm's mask beside it) → combat effects (into the
//   resolved world and its mask) → fog mask pass (distances,
//   then the unseen look, before post) → post (bloom, grade, AgX, into the
//   canvas) → fog rim (display space) → overlays (display space, against the
//   world's depth) → composite
//
// bracketed by two timestamp markers for the frame's GPU time. Rewritten here
// from reading ~/dev/game battle-renderer/src/world/frame.ts (reuse manifest,
// technique); that frame had no structures layer, overlay stage, registry or
// timing.
import { tgpu } from "typegpu";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/cameraUniform";
import { GPU_DEPTH_CLEAR, GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
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
import { createFogMaskPass } from "./fogMaskPass";
import { createOverlayPass } from "./overlayPass";
import { createFrameTimer } from "./gpuTiming";
import { createEffectPass } from "../effects/effectPass";
import { createModelLayer, type CardAtlas } from "../models/modelLayer";
import { createImpostorBaker } from "../models/impostor";
import { CARD_SPEC } from "../models/impostorCards";
import type { ModelDetailPresentation } from "../models/modelDetail";
import { createDetailView, detailKey, setDetailView } from "./detailView";

/** Post's five-level bloom needs at least this many pixels a side. */
const MIN_TARGET_PX = 64;

export interface BattleFrameOptions {
  /** `presentation.light`: sun, sky, haze, grade, bloom and cascades. */
  light: LightPresentation;
  /** `presentation.fog_geometry`: the sight lights' resolution and budgets. */
  fogGeometry: FogGeometryPresentation;
  /** The unseen look to start with (`presentation.fog`'s selected style). */
  fogStyle: FogStyle;
  /** `presentation.models`: the models' detail tiers and impostor size. */
  models: ModelDetailPresentation;
  world: WorldLayers;
  instances: readonly SceneInstance[];
  /** The viewport's size in device pixels: targets are built for it up front. */
  width: number;
  height: number;
  /** Called when targets for a new size finish building, so the viewport draws. */
  requestRedraw?: () => void;
}

/** Device pixels per metre at the camera's target. */
function pixelsPerMetre(camera: Camera3DParams, height: number): number {
  return height / (2 * camera.distance * Math.tan(camera.fovY / 2));
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
    const models = await createModelLayer(root, registry, options.models);
    registry.adopt(() => models.dispose());
    const world = await createWorldPass(root, registry, environment, options.fogGeometry, models);
    const fogMask = await createFogMaskPass(root, registry, displayFormat, options.fogStyle);
    const impostors = createImpostorBaker(root, registry, models, environment);
    const overlay = await createOverlayPass(root, registry, displayFormat);
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
      const fog = world.fogTiles(scope, width, height, t.depth);
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
      const detailView = createDetailView();
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
          world.prepare(camera3d, state.view, state.viewProj, state.rays, height);
          models.prepare(setDetailView(detailView, camera3d, height), detailKey(camera3d, height));

          const encoder = root["~unstable"].createCommandEncoder({ label: "battle-frame" });
          const raw = root.unwrap(encoder);
          timer?.begin(raw);
          models.encodePose(raw);
          world.encodeShadows(encoder);
          world.encodeDepth(encoder, t, cameraGroup);
          world.encodeFog(raw, t.fog, state.bytes, width, height);
          world.encode(encoder, raw, t, cameraGroup);
          effects.encode(raw, t, t.effectGroup);
          fogMask.encode(raw, t, t.fogEdge);
          const maskView = view === "fog-mask" || view === "ground-mask";
          const worldOnly = view === "world" || maskView;
          if (view === "final" || worldOnly) {
            // The masks skip bloom and grade: white stays white, black black.
            const graded = !maskView;
            t.post.encode(raw, output, graded, graded);
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
          if (!worldOnly && !overlay.empty) {
            overlay.encode(raw, t, t.overlaySource, cameraGroup, output);
          }
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
        setGround(next) {
          return !disposed && world.setGround(next);
        },
        setClock(seconds) {
          clock = seconds;
        },
        setOverlay(next) {
          if (!disposed) overlay.set(next);
        },
        setEffects(batch) {
          if (!disposed) effects.set(batch);
        },
        setInstances(next) {
          if (!disposed) world.setInstances(next);
        },
        setFog(next) {
          if (!disposed) world.setFog(next);
        },
        async setAppearances(next) {
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
        },
        setModels(next) {
          if (!disposed) models.setModels(next);
        },
        setCorpses(next) {
          if (!disposed) models.setCorpses(next);
        },
        readPalette: () => models.readPalette(),
        timePoseKernel: (reps, bodies) => models.timeKernel(reps, bodies),
        async bakeImpostor(appearance, spec) {
          const far = models.farPose(appearance);
          if (!far) throw new Error(`no installed appearance ${appearance}`);
          return impostors.bake(appearance, far.pose, far.bounds, spec);
        },
        paletteBases: () => models.paletteBases(),
        setFogStyle(next) {
          if (!disposed) fogMask.setStyle(next);
        },
        setView(next) {
          view = next;
          fogMask.setMaskView(
            next === "fog-mask" ? "fog" : next === "ground-mask" ? "ground" : "none",
          );
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
            scenery: passes.scenery,
            grass: passes.grass,
            effects: effects.stats(),
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
