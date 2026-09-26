// The seam between the lab's views and the renderer: what a view hands the
// battle frame to draw, and what the frame reports back. The frame itself is
// `frame/battleFrame.ts`.
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import type { ViewportCamera } from "@packages/renderer-core/src/cameraUniform";
import type { Mesh } from "./mesh";
import type { ProxyKind } from "./proxies";
import type { GpuFrameTime } from "./frame/gpuTiming";
import type { FogInput } from "./frame/fogInputs";
import type { FogStyle } from "./frame/fogStyle";
import type { FogProbes, FogVisibilityStats } from "./frame/fogVisibility";
import type { TerrainSurface } from "./terrain/terrainSurface";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { ModelInstance } from "./models/modelInstances";
import type { ModelStats } from "./models/modelLayer";
import type { ImpostorAtlas, ImpostorSpec } from "./models/impostor";
import type { StaticBundle } from "@packages/scene-assets/src/schema";
import type { SceneryPlacement } from "./scenery/placement";
import type { SceneryStats } from "./frame/sceneryLayer";
import type { GrassProbes, GrassStats } from "./frame/grassPass";
import type { GrassAppearances } from "./terrain/grassField";

/** A drawn proxy: world placement plus presentation tint. */
export interface SceneInstance {
  kind: ProxyKind;
  x: number;
  y: number;
  z: number;
  /** Heading of local +X, radians counter-clockwise from world +X. */
  yaw: number;
  color: readonly [number, number, number];
  highlight?: boolean;
}

/** Overlay geometry in world space (orders, contacts, tracers, rings).
 *  Translucent triangles draw after everything opaque without writing depth. */
export interface WorldMeshes {
  opaque: Mesh;
  translucent: Mesh;
}

/** The trees and hedgerows: where each stands, and the appearances
 *  (`tree` and `hedgerow` bundles from the one loader) it instances. */
export interface WorldScenery {
  placement: SceneryPlacement;
  appearances: ReadonlyMap<string, StaticBundle>;
  /** `biome.trees.lod_px`: the detail tiers by projected height. */
  lodPx: readonly [number, number, number];
}

/** The static world in layers, so each takes its own material and FogTerm
 *  its own ground flag: the terrain is ground; the props standing on it
 *  (buildings, walls, the map's skirt) are faces. Translucent triangles
 *  (water) draw after everything opaque without writing depth. The
 *  scenery's forest stands in the simulation's forests and draws their
 *  trunks; `null` draws none (a route without appearances). */
export interface WorldLayers {
  terrain: TerrainSurface;
  props: Mesh;
  translucent: Mesh;
  scenery: WorldScenery | null;
  /** The grass kinds the biome grows (scenery "grass" appearances) by catalog
   *  name; `null` draws no grass (a route without appearances, or a view
   *  where the tint replaces the biome). */
  grass: GrassAppearances | null;
}

export interface InstalledDepthState {
  format: GPUTextureFormat;
  clearValue: number;
  compare: GPUCompareFunction;
}

/** What the frame shows: the finished frame, or one stage of it for the
 *  lab's pass inspector. The overlay views replace post's output with a flat
 *  clear, so what remains is exactly what the overlay pass lays down. The fog
 *  mask draws the resolved fog mask: black where unseen, white elsewhere
 *  (seen, and whatever fog never covers: units, the sky, the backdrop). The
 *  ground mask draws the same mask's ground coverage: white where a pixel is
 *  mostly ground (the terrain and grass), black elsewhere. */
export type FrameView =
  | "final"
  | "world"
  | "overlays-on-black"
  | "overlays-on-white"
  | "fog-mask"
  | "ground-mask";
export const FRAME_VIEWS: readonly FrameView[] = [
  "final",
  "world",
  "overlays-on-black",
  "overlays-on-white",
  "fog-mask",
  "ground-mask",
];

export interface FrameStats {
  width: number;
  height: number;
  frames: number;
  instances: number;
  /** The models layer: appearances installed, models drawn, triangles, draws. */
  models: ModelStats;
  worldVertices: number;
  structureVertices: number;
  depth: InstalledDepthState;
  view: FrameView;
  /** Presentation seconds: the wind's clock (`setClock`). */
  clock: number;
  /** The frame's GPU time from `timestamp-query`, when the device has it. */
  gpu: GpuFrameTime | null;
  /** The device's live allocations, textures sized. */
  memory: GpuAllocationCounts;
  /** Where the sun's cascades fit this frame. */
  shadow: {
    receiverRange: [number, number];
    cascades: { extent: number; texel: number }[];
  };
  /** The sight lights: eyes, rebuilds and their buffers. */
  fog: FogVisibilityStats;
  /** The fog mask pass: the edge it draws and how far it searched for it
   *  (0 when the style has no edge, and the distance passes were skipped). */
  fogEdge: {
    edgeSoftnessPx: number;
    rimWidthPx: number;
    reachPx: number;
    /** Which mask compose draws in place of the look ("none" for the look). */
    maskView: "none" | "fog" | "ground";
  };
  /** Trees and hedgerows placed and drawn per detail tier. */
  scenery: SceneryStats;
  /** The grass field: its window and buffers. */
  grass: GrassStats;
}

export interface BattleFrame {
  /** Draw one frame into `target` at `camera`'s viewport. While targets for a
   *  new size are still being built the frame draws nothing and asks for a
   *  redraw once they exist. */
  render(target: GPUTextureView, camera: ViewportCamera): void;
  /** The static world: the terrain and the props on it. */
  setWorld(world: WorldLayers): void;
  /** Knowledge-drawn world geometry: the buildings the side knows stand and
   *  the ruins and wrecks it remembers. Lit, graded and shadow-casting like the
   *  world, but read over fog. */
  setStructures(structures: Mesh): void;
  /** Display-space marks (orders, contacts, tracers, rings), drawn after post
   *  over the world's depth so their colours are exactly their own. */
  setOverlay(overlay: WorldMeshes): void;
  setInstances(instances: readonly SceneInstance[]): void;
  /** Presentation seconds: the wind's clock. Hold it and the frame holds
   *  (paused battles, deterministic captures). */
  setClock(seconds: number): void;
  /** What the observing side sees from (its eyes at the published tick and
   *  the occluders it knows), over the static world; `null` shows everything
   *  clear. */
  setFog(fog: FogInput | null): void;
  /** Install a catalog generation from `AppearanceLibrary` (the one loader);
   *  resolves once its meshes, clips and pose kernel are on the GPU. */
  setAppearances(installed: InstalledAppearances | null): Promise<void>;
  /** The posed models to draw (from the pose driver), replacing the last list. */
  setModels(models: readonly ModelInstance[]): void;
  /** Debug readback: the palette matrices of the last drawn frame, and where
   *  each drawn model's palette starts. */
  readPalette(): Promise<Float32Array>;
  paletteBases(): number[];
  /** Bake an installed appearance's far-pose impostor atlas with this frame's
   *  model path (`models/impostor.ts`). */
  bakeImpostor(appearance: string, spec?: ImpostorSpec): Promise<ImpostorAtlas>;
  /** How unseen looks: identity on seen pixels, live from the next frame. */
  setFogStyle(style: FogStyle): void;
  /** The pass inspector's view. */
  setView(view: FrameView): void;
  /** Lab probes of the sight lights (debug readbacks, never in a frame). */
  readonly fogProbes: FogProbes;
  /** Lab probes of the grass field (debug readbacks, never in a frame). */
  readonly grassProbes: GrassProbes;
  /** Resolves once no target rebuild is pending; true if one was. */
  settled(): Promise<boolean>;
  stats(): FrameStats;
  /** Frees every allocation. Calls after dispose are ignored, since the
   *  viewport may still draw while it awaits a rebuilt frame. */
  dispose(): void;
}
