// The seam between the lab's views and the renderer: what a view hands the
// battle frame to draw, and what the frame reports back. The frame itself is
// `frame/battleFrame.ts`.
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import type { ViewportCamera } from "@packages/renderer-core/src/cameraUniform";
import type { Mesh } from "./mesh";
import type { ProxyKind } from "./proxies";
import type { GpuFrameTime } from "./frame/gpuTiming";

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

/** Geometry in world space. Translucent triangles draw after everything
 *  opaque without writing depth. */
export interface WorldMeshes {
  opaque: Mesh;
  translucent: Mesh;
}

/** One bit per ground cell, row-major, set where the side can see. */
export interface FogField {
  cellM: number;
  nx: number;
  ny: number;
  bits: Uint32Array;
}

export interface InstalledDepthState {
  format: GPUTextureFormat;
  clearValue: number;
  compare: GPUCompareFunction;
}

/** What the frame shows: the finished frame, or one stage of it for the
 *  lab's pass inspector. The overlay views replace post's output with a flat
 *  clear, so what remains is exactly what the overlay pass lays down. */
export type FrameView = "final" | "world" | "overlays-on-black" | "overlays-on-white";
export const FRAME_VIEWS: readonly FrameView[] = [
  "final",
  "world",
  "overlays-on-black",
  "overlays-on-white",
];

export interface FrameStats {
  width: number;
  height: number;
  frames: number;
  instances: number;
  worldVertices: number;
  structureVertices: number;
  depth: InstalledDepthState;
  view: FrameView;
  /** The frame's GPU time from `timestamp-query`, when the device has it. */
  gpu: GpuFrameTime | null;
  /** The device's live allocations, textures sized. */
  memory: GpuAllocationCounts;
  /** Where the sun's cascades fit this frame. */
  shadow: {
    receiverRange: [number, number];
    cascades: { extent: number; texel: number }[];
  };
}

export interface BattleFrame {
  /** Draw one frame into `target` at `camera`'s viewport. While targets for a
   *  new size are still being built the frame draws nothing and asks for a
   *  redraw once they exist. */
  render(target: GPUTextureView, camera: ViewportCamera): void;
  /** The static world: terrain and props. */
  setWorld(world: WorldMeshes): void;
  /** Knowledge-drawn world geometry: the buildings the side knows stand and
   *  the ruins and wrecks it remembers. Lit, graded and shadow-casting like the
   *  world, but read over fog. */
  setStructures(structures: Mesh): void;
  /** Display-space marks (orders, contacts, tracers, rings), drawn after post
   *  over the world's depth so their colours are exactly their own. */
  setOverlay(overlay: WorldMeshes): void;
  setInstances(instances: readonly SceneInstance[]): void;
  /** The observing side's ground visibility; `null` shows everything clear. */
  setFog(fog: FogField | null): void;
  /** The pass inspector's view. */
  setView(view: FrameView): void;
  /** Resolves once no target rebuild is pending; true if one was. */
  settled(): Promise<boolean>;
  stats(): FrameStats;
  /** Frees every allocation. Calls after dispose are ignored, since the
   *  viewport may still draw while it awaits a rebuilt frame. */
  dispose(): void;
}
