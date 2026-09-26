// The battle frame: one owner for the passes, their order and every GPU
// allocation they make. Each frame runs
//
//   shadows (4 cascades) → depth prepass → fog (moved eyes' horizon maps,
//   then the per-tile eye lists from that depth) → sky + HDR world (4× MSAA,
//   rgba16float, FogTerm before post) → post (bloom, grade, AgX, into the
//   canvas) → overlays (display space, against the world's depth) → composite
//
// bracketed by two timestamp markers for the frame's GPU time. Rewritten here
// from reading ~/dev/game battle-renderer/src/world/frame.ts (reuse manifest,
// technique); that frame had no structures layer, overlay stage, registry or
// timing.
import { tgpu } from "typegpu";
import { liveCamera, type ViewportCamera } from "@packages/renderer-core/src/cameraUniform";
import { GPU_DEPTH_CLEAR, GPU_DEPTH_FORMAT } from "@packages/renderer-core/src/depthContract";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { BattleFrame, FrameView, SceneInstance, WorldMeshes } from "../scene";
import { Camera, typegpuCameraLayout } from "../world/camera";
import { createTypegpuPost } from "../world/post";
import { frameCamera } from "../frameCamera";
import { battleWorldDepth } from "../worldDepth";
import type { LightPresentation } from "../light/sceneLight";
import type { FogGeometryPresentation } from "./fogInputs";
import { createEnvironmentFrame } from "./environmentFrame";
import { GpuRegistry } from "./registry";
import { allocateFrameTargets, SizedTargets } from "./targets";
import { createWorldPass } from "./worldPass";
import { createOverlayPass } from "./overlayPass";
import { createFrameTimer } from "./gpuTiming";

/** Post's five-level bloom needs at least this many pixels a side. */
const MIN_TARGET_PX = 64;

export interface BattleFrameOptions {
  /** `presentation.light`: sun, sky, haze, grade, bloom and cascades. */
  light: LightPresentation;
  /** `presentation.fog_geometry`: the sight lights' resolution and budgets. */
  fogGeometry: FogGeometryPresentation;
  world: WorldMeshes;
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
    const world = await createWorldPass(root, registry, environment, options.fogGeometry);
    const overlay = await createOverlayPass(root, registry, displayFormat);
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
      return { ...t, post, fog, overlaySource: overlay.sourceFor(t) };
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
            },
            width,
            height,
          );
          camera.write(state.bytes.buffer);
          world.prepare(camera3d, state.view, state.rays);

          const encoder = root["~unstable"].createCommandEncoder({ label: "battle-frame" });
          const raw = root.unwrap(encoder);
          timer?.begin(raw);
          world.encodeShadows(encoder);
          world.encodeDepth(encoder, t, cameraGroup);
          world.encodeFog(raw, t.fog, state.bytes, width, height);
          world.encode(encoder, raw, t, cameraGroup);
          const worldOnly = view === "world" || view === "fog-mask";
          if (view === "final" || worldOnly) {
            // The fog mask skips bloom and grade: seen stays white, unseen black.
            const graded = view !== "fog-mask";
            t.post.encode(raw, output, graded, graded);
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
        setOverlay(next) {
          if (!disposed) overlay.set(next);
        },
        setInstances(next) {
          if (!disposed) world.setInstances(next);
        },
        setFog(next) {
          if (!disposed) world.setFog(next);
        },
        setView(next) {
          view = next;
          world.setFogMask(next === "fog-mask");
          timer?.reset();
        },
        settled: () => targets.settled(),
        fogProbes: world.fog,
        stats() {
          const t = targets.current;
          const passes = world.stats();
          return {
            width: t?.width ?? 0,
            height: t?.height ?? 0,
            frames,
            instances: passes.instances,
            worldVertices: passes.worldVertices,
            structureVertices: passes.structureVertices,
            depth: {
              format: GPU_DEPTH_FORMAT,
              clearValue: GPU_DEPTH_CLEAR,
              compare: battleWorldDepth("read-write").depthCompare!,
            },
            view,
            gpu: timer?.stats() ?? null,
            memory: registry.stats(),
            shadow: passes.shadow,
            fog: passes.fog,
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
