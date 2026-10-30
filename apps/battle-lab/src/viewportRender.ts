/** The viewport's render callbacks and owned GPU handle. */
import type { AllocationCounts, DrawnInstances, SceneStats } from "@web/battle/render/sceneStats";
import type { BrowserGpu } from "@wasm/renderer_browser";
import type { BrowserScene } from "@web/battle/render/scene";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import type { GrassClumpRow } from "@packages/battle-renderer/src/grass";
import type { EffectCounts, ModelCounts } from "./presentedFrame";

/** What a drawn frame drew, counted, for a second client to count against:
 *  the building modules, scenery and props, the models it posed and the
 *  combat effects. */
export interface FrameCounts {
  drawn: DrawnInstances;
  /** The soldiers, vehicles and corpses the frame posed. */
  models: ModelCounts;
  /** The combat effects the frame drew. */
  effects: EffectCounts;
}

/** A drawn frame read back for a second client to draw against: the canvas
 *  as the player saw it, the grass clumps the frame grew, its counts, and
 *  the same frame drawn again offscreen lit without its grass. The canvas
 *  and the next frame are untouched. */
export interface FramePlate extends FrameCounts {
  png: Promise<Blob>;
  clumps: Promise<readonly Pick<GrassClumpRow, "root">[]>;
  plain: Promise<Blob>;
}

/** A script's hold on the viewport's camera. */
export interface ViewportPilot {
  /** The pilot reads drawn frames back (`frame`'s `plate`). Only then may
   *  the canvas be copied from, so no other frame pays for it. */
  plates?: boolean;
  /** Show the in-game frame-rate counter, which a piloted view otherwise
   *  hides (the graphics test shows it; the menu's reel does not). */
  frameRate?: boolean;
  /** Renderer diagnostics and a fresh GPU measurement boundary. */
  attach?(frame: {
    adapter: string;
    stats: () => SceneStats & {
      receiverFit: ReturnType<ScenePresentation["shadowStats"]> | null;
    };
    resetGpuTiming: () => void;
  }): void;
  /** The framing for the frame at `now`; the rig places it
   *  (`CameraController.place`) and draws it clear of obstacles (`resolve`).
   *  Null leaves the camera to the player for that frame. */
  pose(now: number): CameraPose | null;
  /** Measurements and optional readback from the task that drew this frame. */
  frame?(f: {
    now: number;
    cpuMs: number;
    receiverFit: ReturnType<ScenePresentation["shadowStats"]> | null;
    camera: Camera3DParams;
    simTime: number;
    canvas: readonly [number, number];
    counts: () => FrameCounts;
    plate?: () => FramePlate;
  }): void;
}

/** What the viewport's device reports once it is up. */
export interface ViewportGpu {
  allocations: () => AllocationCounts;
  /** Independent workbench outputs share the page's admitted GPU. */
  gpu: BrowserGpu;
  /** The live scene (it is replaced by `rebuild`). */
  frame: () => BrowserScene;
}
