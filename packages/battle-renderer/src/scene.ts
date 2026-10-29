// The seam between the lab's views and the renderer: what a view hands the
// battle frame to draw, and what the frame reports back. The frame itself is
// `frame/battleFrame.ts`.
import type { GpuAllocationCounts } from "@packages/renderer-core/src/gpuAllocations";
import type { Mesh } from "./mesh";
import type { ProxyKind } from "./proxies";
import type { GpuFrameTime } from "./frame/gpuTiming";
import type { OverlayGlowStyle } from "./frame/overlayPass";
import type { FogProbes, FogVisibilityStats } from "./frame/fogVisibility";
import type { TerrainSurface } from "./terrain/terrainSurface";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { ModelInstance } from "./models/modelInstances";
import type { frameViews } from "./records.gen";
import type { WorldAppearance } from "./models/modelLayer";
import type { ModelStats } from "./models/modelLayer";
import type { BuildingStats, BuildingWalkStats } from "./models/buildingLayer";
import type { SurfaceClass } from "./models/surfaceParts";
import type { ImpostorAtlas, ImpostorSpec } from "./models/impostor";
import type { StaticBundle, TextureChannel } from "@packages/scene-assets/src/schema";
import type { PresentedGround } from "./terrain/terrainSurface";
import type { SceneryStats, SceneryWalkStats } from "./frame/sceneryLayer";
import type { GrassProbes, GrassStats } from "./frame/grassPass";
import type { SceneryCounts } from "./frame/sceneryLayer";
import type { GrassAppearances } from "./terrain/grassField";
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

/** The trees, hedgerows and forest-floor dressing's art: the appearances
 *  (`tree`, `hedgerow` and `dressing` bundles from the one loader) the
 *  scenery's rows name. Where each stands, and a felled tree's stump, are
 *  the presentation's. */
export interface WorldScenery {
  appearances: ReadonlyMap<string, StaticBundle>;
}

/** The static world in layers, so each takes its own material and FogTerm
 *  its own ground flag: the terrain is ground; the map's skirt (and the
 *  traversal view's prop boxes) are faces. The map's props draw as their
 *  appearances, fitted to their boxes by the scene's presentation (its
 *  static batches). The scenery's forest stands in the simulation's forests
 *  and draws their trunks; `null` draws none (a route without appearances). */
export interface WorldLayers {
  terrain: TerrainSurface;
  /** Geometry the visual backdrop surrounds when it differs from the terrain
   * drawn and received by shadows (for example disconnected remote patches).
   * Omitted: surround the full terrain. This never bounds terrain selection. */
  backdropTerrain?: Mesh;
  props: Mesh;
  scenery: WorldScenery | null;
  /** The grass kinds the biome grows (scenery "grass" appearances) by catalog
   *  name; `null` draws no grass (a route without appearances, or a view
   *  where the tint replaces the biome). */
  grass: GrassAppearances | null;
}

/** The building modules, scenery and props a frame drew. */
export interface DrawnInstances {
  /** Per module the buildings' walk draws (`StaticWorld.modules`) and
   *  tier: `module · 4 + tier`. */
  buildings: number[];
  scenery: SceneryCounts;
  /** The props (the map's and those the side knows: wrecks, shoved
   *  bodies, rubble), by appearance in name order: per tier 0–3. */
  props: { appearance: string; levels: number[] }[];
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
  /** Buildings drawn from template art: what a view draws of them, the
   *  pool's residency and what the last view change cost. */
  buildings: BuildingStats;
  /** Actual partial-world submissions, split by pass and cascade. */
  submitted: import("./frame/submitted").SubmittedFrame;
  /** Terrain bounds tested across the camera and active shadow cascades. */
  terrainVisitedNodes: number;
  /** Boundary/debug bounds tested across camera and active shadow cascades. */
  skirtVisitedNodes: number;
  /** Water bounds tested for the camera; water casts no shadows. */
  waterVisitedNodes: number;
  /** Installed world mesh vertices, not submitted geometry. */
  worldVertices: number;
  depth: InstalledDepthState;
  view: FrameView;
  /** Presentation seconds: the wind's clock, the camera's (`setCamera`). */
  clock: number;
  /** The frame's GPU time from `timestamp-query`, when the device has it. */
  gpu: GpuFrameTime | null;
  /** The device's live allocations, textures sized. */
  memory: GpuAllocationCounts;
  /** Where the sun's cascades fit this frame. */
  shadow: {
    receiverRange: [number, number];
    cascades: number;
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
  effects: { instances: number; capacity: number; lights: number };
  /** The scar texture: the grid it holds and what its uploads wrote. */
  scars: ScarStats;
}

export interface BattleFrame {
  /** Draw one frame into `target` for the camera `setCamera` took, at its
   *  viewport. Before any camera, and while targets for a new size are still
   *  being built, the frame draws nothing (and asks for a redraw once they
   *  exist). With `high` (a target of `target`'s size and format, laid
   *  over it by the page, between the ground and low-air cards and the
   *  high-air ones), the high-air models (`HIGH_PASS`) draw there alone,
   *  over transparency, premultiplied, in the final view; it is cleared
   *  when there are none. Without it they draw with the world. */
  render(target: GPUTextureView, high?: GPUTextureView | null): void;
  /** The presentation frame's camera (`crates/presentation`'s `Frame`): its
   *  view, the camera uniform at the frame's clock (the wind's), and the
   *  sun's cascades fitted to it, uploaded as they are. Read at once: the
   *  views last only until the next call into the module. A frame without
   *  cascades keeps the last. With it, the wave field's steps and splats
   *  since the frame before (`Presentation.waves`), run once each. */
  setCamera(frame: ReturnType<typeof frameViews> & { waves?: Float32Array }): void;
  /** The static world: the terrain and the props on it. */
  setWorld(world: WorldLayers): void;
  /** The static batches of the scene's presentation frame
   *  (`crates/presentation`): the map's buildings, each a kit module at a
   *  tier, its scenery (trees, shrubs, dressing, felled trees and their
   *  stumps), and its props and what the side knows of them (rubble,
   *  wrecks, shoved bodies), each an appearance at a tier, drawn into the
   *  view or cast into the sun's cascades, as Rust walked them for this
   *  frame's camera and what the side knows. Lit,
   *  shadow-casting and fogged like the world.
   *  These are the frame's batches of rows marked `placed`
   *  (`setPresentationWorld`); `stats` is what the presentation says of its
   *  walk. Call every frame before drawing, with the frame `setFrame` took
   *  (taken again after any call into the module): a frame naming its static
   *  batches unchanged keeps what was uploaded. Returns whether what is
   *  drawn changed. Null draws none. Frame the presentation for
   *  `staticGeneration`, so it writes every batch when this frame holds none. */
  setStaticFrame(
    frame: ReturnType<typeof frameViews> | null,
    stats: { buildings: BuildingWalkStats; scenery: SceneryWalkStats },
  ): boolean;
  /** The presentation's `generation` for this frame's static batches: new
   *  with the frame and with each `setPresentationWorld`, both of which leave
   *  it holding none. */
  readonly staticGeneration: number;
  /** The ground the scene's presentation resolved (`TerrainSurface.input`):
   *  the terrain material's tables, the grass's and the water's mesh.
   *  `null` draws no ground. */
  setPresentedGround(ground: PresentedGround | null): void;
  /** The observing side's learned ground (the presentation's `LearnedGround`), drawn
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
  /** This frame's presentation (`crates/presentation`'s `Frame`): the
   *  units, corpses and ghosts as batches with their palettes (the static
   *  world's batches are `setStaticFrame`'s), the combat
   *  effects drawn into the lit world before fog and post, and the lights
   *  they cast (every world material and lit smoke). Read at once: the
   *  views last only until the next call into the module. Null draws none. */
  setFrame(
    frame: ReturnType<typeof frameViews> | null,
    shown?: { models: boolean; effects: boolean },
  ): void;
  /** The drawables the presentation's batches name, by `World.appearances` row. */
  setPresentationWorld(world: readonly WorldAppearance[]): void;
  /** Install a catalog generation from `AppearanceLibrary` (the one loader);
   *  resolves once its meshes and impostor atlases (every
   *  body's far pose and corpse) are on the GPU. */
  setAppearances(installed: InstalledAppearances | null): Promise<void>;
  /** A lab's models (posed with `posedPalette`), replacing the last list. */
  setModels(models: readonly ModelInstance[]): void;
  /** Switch material texture channels on or off for every model (the
   *  workbench's per-channel toggles); off draws the material's factors. */
  setTextureChannels(channels: Partial<Record<TextureChannel, boolean>>): void;
  /** Bake an installed appearance's far-pose impostor atlas with this frame's
   *  model path (`models/impostor.ts`). */
  bakeImpostor(appearance: string, spec?: ImpostorSpec): Promise<ImpostorAtlas>;
  /** How unseen looks (a `FogLook` record, `resolveFogLook`): identity on
   *  seen pixels, live from the next frame. */
  setFogLook(look: Float32Array): void;
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
   *  their shadows or not. */
  setTreesShown(on: boolean): void;
  /** Lab diagnostics: draw the buildings and their shadows or not (a paired
   *  cost measure). */
  setBuildingsShown(on: boolean): void;
  /** Lab diagnostics: draw one kind of model surface (`surfaceParts.ts`: a
   *  cutout, say) with its depth and shadow, or none of it: paired frames
   *  and cost. */
  setSurfaceShown(surface: SurfaceClass, on: boolean): void;
  /** Lab diagnostics: draw the roads worn (surface detail, shoulders, the
   *  grass thinned across them) or plain, each kind's flat colour to its
   *  edge: paired frames and cost. */
  setRoadWearShown(on: boolean): void;
  /** Lab probes of the sight lights (debug readbacks, never in a frame). */
  readonly fogProbes: FogProbes;
  /** Lab probes of the grass field (debug readbacks, never in a frame). */
  readonly grassProbes: GrassProbes;
  /** What the last drawn frame drew of the buildings and the scenery, per
   *  kind and tier, for a second client to count against. */
  drawnInstances(): DrawnInstances;
  /** Resolves once no target rebuild is pending; true if one was. */
  settled(): Promise<boolean>;
  stats(): FrameStats;
  /** Clear GPU timing samples and exclude frames begun before this call.
   *  Does not wait for readbacks or reset the lifetime frame count. */
  resetGpuTiming(): void;
  /** Frees every allocation. Calls after dispose are ignored, since the
   *  viewport may still draw while it awaits a rebuilt frame. */
  dispose(): void;
}
