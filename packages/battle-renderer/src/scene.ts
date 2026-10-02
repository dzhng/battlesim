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
import type { OverlayGlowStyle } from "./frame/overlayPass";
import type { FogProbes, FogVisibilityStats } from "./frame/fogVisibility";
import type { TerrainSurface } from "./terrain/terrainSurface";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { CorpseInstance, ModelInstance } from "./models/modelInstances";
import type { ModelStats } from "./models/modelLayer";
import type { ImpostorAtlas, ImpostorSpec } from "./models/impostor";
import type { StaticBundle, TextureChannel } from "@packages/scene-assets/src/schema";
import type { SceneryPlacement } from "./scenery/placement";
import type { PlacedInstances } from "./scenery/lod";
import type { SceneryStats } from "./frame/sceneryLayer";
import type { GrassProbes, GrassStats } from "./frame/grassPass";
import type { GrassAppearances } from "./terrain/grassField";
import type { EffectBatch } from "./effects/effectFrame";
import type { GroundMarks, ScarStats } from "./frame/scarTexture";

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

/** Marks in world space. `opaque` and `translucent` are display-space
 *  overlay (contacts, and the labs' tracers, garrison and guidance marks):
 *  drawn after post, never fogged or lit; translucent triangles draw after
 *  everything opaque without writing depth. `painted` and `paintedMarching`
 *  are painted on the ground in the lit world (`frame/paintedMarks.ts`: the
 *  orders and selection, a truck's reach, the zone, the border): shadowed,
 *  fogged, under the effects, glowing by their own emissive; the marching marks' normals carry (phase in cycles, cycles a
 *  second, amplitude) instead of a direction (`orderOverlay.ts`
 *  `travelChevrons`). */
export interface WorldMeshes {
  opaque: Mesh;
  translucent: Mesh;
  painted?: Mesh;
  paintedMarching?: Mesh;
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
 *  its own ground flag: the terrain is ground; the map's skirt (and the
 *  traversal view's prop boxes) are faces. The map's props draw as their
 *  appearances, fitted to their boxes (`structures`, faces too), except
 *  those a route draws from knowledge (`BattleFrame.setStructures`).
 *  The scenery's forest stands in the simulation's forests
 *  and draws their trunks; `null` draws none (a route without appearances). */
export interface WorldLayers {
  terrain: TerrainSurface;
  props: Mesh;
  structures: readonly ModelInstance[];
  /** The water surfaces (the terrain material's water), drawn after
   *  everything opaque without writing depth. */
  water: Mesh;
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
 *  mostly ground (the terrain and grass), black elsewhere. The ground classes
 *  draw what the terrain material says the ground under each pixel is
 *  (`terrain/groundClasses.ts`), byte for byte: black where the pixel is not
 *  wholly bare terrain; grass and water are left out so the ground shows. */
export type FrameView =
  | "final"
  | "world"
  | "overlays-on-black"
  | "overlays-on-white"
  | "fog-mask"
  | "ground-mask"
  | "ground-classes"
  | "paint";
export const FRAME_VIEWS: readonly FrameView[] = [
  "final",
  "world",
  "overlays-on-black",
  "overlays-on-white",
  "fog-mask",
  "ground-mask",
  "ground-classes",
  "paint",
];

export interface FrameStats {
  width: number;
  height: number;
  frames: number;
  instances: number;
  /** The models layer: appearances installed, models drawn, triangles, draws. */
  models: ModelStats;
  worldVertices: number;
  /** Props drawn from what the side knows (standing buildings, ruins, wrecks). */
  structures: number;
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
    maskView: "none" | "fog" | "ground" | "classes";
  };
  /** The overlays' own halo (0 strength: none drawn). */
  overlay: { glowRadiusPx: number; glowStrength: number };
  /** Trees and hedgerows placed and drawn per detail tier. */
  scenery: SceneryStats;
  /** The grass field: its window and buffers. */
  grass: GrassStats;
  /** Combat effects: instances drawn and the instance buffer's capacity;
   *  the cast lights the world took and the effects offered. */
  effects: { instances: number; capacity: number; lights: number; lightsOffered: number };
  /** The scar texture: the grid it holds and what its uploads wrote. */
  scars: ScarStats;
}

export interface BattleFrame {
  /** Draw one frame into `target` at `camera`'s viewport. While targets for a
   *  new size are still being built the frame draws nothing and asks for a
   *  redraw once they exist. */
  render(target: GPUTextureView, camera: ViewportCamera): void;
  /** The static world: the terrain and the props on it. */
  setWorld(world: WorldLayers): void;
  /** Knowledge-drawn props, as fitted appearances (`structureModels`): the
   *  buildings the side knows stand and the ruins and wrecks it remembers.
   *  Lit, graded, shadow-casting and fogged like the world. */
  setStructures(structures: readonly ModelInstance[]): void;
  /** Knowledge-drawn massing (`massingInstances`): the boxes of the artless
   *  buildings the side knows stand and the remains of those it has seen
   *  fall, replacing the last list. Lit, shadow-casting and fogged like the
   *  world. Call when the list changes, not every frame: it is chunked.
   *  `null` draws none. */
  setMassing(massing: PlacedInstances | null): void;
  /** The observing side's learned ground (the client's `GroundView`), drawn
   *  as scars on the terrain and grass. Called every animation frame: a new
   *  view is uploaded whole, the same view only where it changed since the
   *  last call. Returns whether anything drawn changed. `null` draws none. */
  setGround(ground: GroundMarks | null): boolean;
  /** Display-space marks (orders, contacts, tracers, rings), drawn after post
   *  over the world's depth so their colours are exactly their own. */
  setOverlay(overlay: WorldMeshes): void;
  /** Ground paint that follows the pointer (the range ruler), drawn with the
   *  overlay's painted marks but set apart, so a pointer move uploads only
   *  it. An empty mesh draws none. */
  setPointerMarks(painted: Mesh): void;
  setInstances(instances: readonly SceneInstance[]): void;
  /** This frame's combat effects (`EffectFrame.build`), drawn into the lit
   *  world before fog and post, and the lights they cast on it (every world
   *  material and lit smoke). Cheap enough to call every frame. */
  setEffects(batch: EffectBatch): void;
  /** Presentation seconds: the wind's clock. Hold it and the frame holds
   *  (paused battles, deterministic captures). */
  setClock(seconds: number): void;
  /** What the observing side sees from (its eyes at the published tick and
   *  the occluders it knows), over the static world; `null` shows everything
   *  clear. */
  setFog(fog: FogInput | null): void;
  /** Install a catalog generation from `AppearanceLibrary` (the one loader);
   *  resolves once its meshes, clips, pose kernel and impostor atlases (every
   *  body's far pose and corpse) are on the GPU. */
  setAppearances(installed: InstalledAppearances | null): Promise<void>;
  /** The posed models to draw (from the pose driver), replacing the last list.
   *  Cheap enough to call every frame. */
  setModels(models: readonly ModelInstance[]): void;
  /** The corpses, drawn static (never skinned), replacing the last list. Call
   *  when the list changes, not every frame: it is chunked for culling. */
  setCorpses(corpses: readonly CorpseInstance[]): void;
  /** Switch material texture channels on or off for every model (the
   *  workbench's per-channel toggles); off draws the material's factors. */
  setTextureChannels(channels: Partial<Record<TextureChannel, boolean>>): void;
  /** Debug readback: the palette matrices of the frame's models, posed
   *  afresh in the readback's own submission (so an impostor bake between
   *  frames can't stand in for them), and where each mesh-drawn model's
   *  palette starts, from the same packing. */
  readPalette(): Promise<{ palette: Float32Array; bases: number[] }>;
  /** Lab probe: GPU time of one pose-kernel dispatch over this frame's posed
   *  bodies, or `bodies` copies of them (the mean of `reps` in one
   *  timestamped pass), or null. */
  timePoseKernel(reps: number, bodies?: number): Promise<{ bodies: number; ms: number } | null>;
  /** Bake an installed appearance's far-pose impostor atlas with this frame's
   *  model path (`models/impostor.ts`). */
  bakeImpostor(appearance: string, spec?: ImpostorSpec): Promise<ImpostorAtlas>;
  /** How unseen looks: identity on seen pixels, live from the next frame. */
  setFogStyle(style: FogStyle): void;
  /** The overlays' own halo, live from the next frame (strength 0: none). */
  setOverlayGlow(glow: OverlayGlowStyle): void;
  /** Lab diagnostics: draw the painted ground marks or not (paired frames). */
  setPaintShown(on: boolean): void;
  /** Lab diagnostics: draw the plots' own texture (grain, broken rows,
   *  wheelings) or their plain rows alone (paired frames and cost). */
  setFieldTextureShown(on: boolean): void;
  /** Diagnostic paired cost/falsification switch; ordinary frames keep coverage enabled. */
  setXrayCoverageEnabled(on: boolean): void;
  /** Lab diagnostics: light the world by the effects' cast lights or not
   *  (paired frames and cost); the effects themselves still draw. */
  setCastLightsShown(on: boolean): void;
  /** The pass inspector's view. */
  setView(view: FrameView): void;
  /** Lab diagnostics: draw the trees (the forest's and the backdrop's) and
   *  their shadows or not; massing still draws. */
  setTreesShown(on: boolean): void;
  /** Lab diagnostics: draw the roads worn (surface detail, shoulders, the
   *  grass thinned across them) or plain, each kind's flat colour to its
   *  edge: paired frames and cost. */
  setRoadWearShown(on: boolean): void;
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
