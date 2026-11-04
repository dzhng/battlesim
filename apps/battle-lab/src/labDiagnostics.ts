import type { BrowserScene } from "@web/battle/render/scene";
import type {
  AllocationCounts,
  BrowserGpuInfo,
  SceneStats,
  FrameView,
} from "@web/battle/render/sceneStats";
import type { SurfaceClass } from "@packages/battle-renderer/src/models/surfaceParts";
import type { PickBox } from "@packages/battle-renderer/src/picking";
import type { Camera3DParams, WorldRay } from "@packages/renderer-core/src/camera3d";
import type { CameraPose } from "@packages/renderer-core/src/cameraController";
import type { ClearanceState } from "@packages/renderer-core/src/cameraClearance";
import type { PropStats } from "@web/battle/sim/presentation";

/** What holds a drawn pose off the pose asked for, and how it got there. */
export type ClearanceReading = Pick<ClearanceState, "hold" | "cut" | "blocked">;

/** Diagnostic hooks the scene harness reads; lab-only, never on a player route. */
export interface LabHandle {
  ready: boolean;
  error: string | null;
  adapter?: BrowserGpuInfo;
  /** The frame's statistics, and what the scene's props draw. */
  stats?: () => SceneStats & { props: PropStats };
  /** Start a fresh GPU measurement window without moving or rebuilding the scene. */
  resetGpuTiming?: () => void;
  allocations?: () => AllocationCounts;
  /** The camera drawn. */
  camera?: () => Camera3DParams;
  /** The harness's raw framing: drawn exactly as given, through neither the
   *  rig's limits nor camera clearance, so a check can frame its subject from
   *  anywhere. No playable or benchmark path uses it; the next input or
   *  scripted placement returns the camera to the rig. */
  setCamera?: (camera: Camera3DParams) => void;
  /** A scripted framing, as the rig places it and the next frame clears it
   *  (the benchmark tour's path): `camera()` is then the pose drawn for it. */
  placeCamera?: (pose: CameraPose) => void;
  /** Where camera clearance stands: the pose asked for, what holds the drawn
   *  pose off it, and whether the last step was a cut or found nothing clear. */
  clearance?: () => ClearanceReading & { asked: Camera3DParams; settled: boolean };
  /** Fly a scratch camera through `poses`, `dt` seconds apart, by the rig
   *  and the obstacles the viewport uses, drawing nothing: each pose drawn,
   *  and what the resolver cost (wall time and box tests, over `reps` flights). */
  flyClearance?: (
    poses: readonly CameraPose[],
    dt: number,
    reps?: number,
  ) => {
    frames: (ClearanceReading & { camera: Camera3DParams })[];
    msPerFrame: number;
    boxTestsPerFrame: number;
  };
  reset?: () => void;
  /** Rebuild the scene from scratch (reset/dispose cycle) and resolve when drawn. */
  rebuild?: () => Promise<void>;
  pickAt?: (cssX: number, cssY: number) => number;
  rayAt?: (cssX: number, cssY: number) => WorldRay;
  route?: Record<string, unknown>;
  /** What a click can pick: every drawn unit and soldier, as boxes. */
  instances?: () => readonly PickBox[];
  /** World point → CSS pixel in the page, or null when behind the eye. */
  projectToCss?: (x: number, y: number, z: number) => [number, number] | null;
  frame?: () => Promise<void>;
  /** The pass inspector's view of the frame; resolves once it is drawn. */
  setFrameView?: (view: FrameView) => Promise<void>;
  /** The sight lights' lab probes. */
  fog?: () => BrowserScene["fogProbes"];
  /** Draw without fog while on (a paired cost measure); the route's fog
   *  returns when it goes off. */
  suppressFog?: (on: boolean) => Promise<void>;
  /** The grass field's lab probes. */
  grass?: () => BrowserScene["grassProbes"];
  /** Draw without grass while on (a paired cost measure). */
  suppressGrass?: (on: boolean) => Promise<void>;
  /** Draw the plots without their own texture while on: their plain rows
   *  alone (paired frames, and a paired cost measure). */
  suppressFieldTexture?: (on: boolean) => Promise<void>;
  /** Draw no models or corpses while on (a paired cost measure). */
  suppressModels?: (on: boolean) => Promise<void>;
  /** Draw no trees and none of their shadows while on (they are scenery, not
   *  models): the ground under a wood, and a paired cost measure. */
  suppressTrees?: (on: boolean) => Promise<void>;
  /** Draw no forest-floor dressing while on (a paired cost measure). */
  suppressDressing?: (on: boolean) => Promise<void>;
  /** Draw no shrubs under tree lines while on (a paired cost measure). */
  suppressUnderstorey?: (on: boolean) => Promise<void>;
  /** Draw no template-art buildings and none of their shadows while on (a
   *  paired cost measure). */
  suppressBuildings?: (on: boolean) => Promise<void>;
  /** Draw one kind of model surface (a cutout, say), its depth and its
   *  shadow, or none of it while on (paired frames and cost). */
  suppressSurface?: (surface: SurfaceClass, on: boolean) => Promise<void>;
  /** Draw the roads plain while on, with no surface detail or shoulder (a
   *  paired cost measure). */
  suppressRoadWear?: (on: boolean) => Promise<void>;
  /** Draw no combat effects while on (a paired cost measure). */
  suppressEffects?: (on: boolean) => Promise<void>;
  /** Draw the effects but light nothing by them while on (paired frames, cost). */
  suppressCastLights?: (on: boolean) => Promise<void>;
  /** The ground marks' halo at `strength` (0 draws none: a paired cost
   *  measure), or the fixture's with null. */
  setOverlayGlowStrength?: (strength: number | null) => Promise<void>;
  /** Draw the painted ground marks or not (paired frames isolate them). */
  suppressPaint?: (on: boolean) => Promise<void>;
  /** Disable the per-model x-ray coverage gate for paired verification/cost. */
  suppressXrayCoverage?: (on: boolean) => Promise<void>;
  /** Draw the ground unmarked while on (paired frames and cost). */
  suppressScars?: (on: boolean) => Promise<void>;
}

declare global {
  interface Window {
    __lab?: LabHandle;
  }
}
