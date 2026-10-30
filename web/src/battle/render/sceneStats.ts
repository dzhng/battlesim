/** Shared renderer diagnostics. Counts describe prepared populations or actual
 * issued draw work as marked; none claim browser/driver memory residency. */
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

export interface SubmittedDraws {
  triangles: number;
  instances: number;
  draws: number;
}
/** Partial world accounting excludes effects, grass, sky, paint, ghosts,
 * x-ray and display overlays. Instances repeat across surfaces and passes. */
export interface SubmittedPass {
  terrain: SubmittedDraws;
  skirtAndProps: SubmittedDraws;
  water: SubmittedDraws;
  backdrop: SubmittedDraws;
  buildings: SubmittedDraws;
  scenery: SubmittedDraws;
  modelsIncludingProps: SubmittedDraws;
}
export interface SubmittedFrame {
  depth: SubmittedPass;
  color: SubmittedPass;
  highDepth: SubmittedPass;
  highColor: SubmittedPass;
  /** All shared cascade slots; inactive slots contain no submissions. */
  cascades: SubmittedPass[];
}
export interface GpuFrameTime {
  frames: number;
  meanMs: number;
  p95Ms: number;
}
export interface AllocationCounts {
  buffers: number;
  textures: number;
  bufferBytes: number;
  textureBytes: number;
}
/** BrowserWebGpu exposes available facts; vendor/architecture may be absent. */
export interface BrowserGpuInfo {
  name: string;
  backend: "browser-webgpu";
  fallback: boolean;
  features: string[];
  limits: Record<string, number>;
}
export interface ModelStats {
  installed: string[];
  /** Installed generated primitives, classified by the browser asset owner. */
  proxyAppearances: number;
  appearances: number;
  /** Prepared non-building mesh population, before surface/pass filtering. */
  instances: number;
  triangles: number;
  draws: number;
  paletteMatrices: number;
  skinned: number;
  tiers: number[];
  bodyTiers: number[];
  cards: number;
  culledBodies: number;
  corpses: number;
  atlasLayers: number;
  textureLayers: number;
  textureBytes: number;
}
export interface BuildingStats {
  buildings: number;
  fallen: number;
  chunks: number;
  coarse: number;
  residentChunks: number;
  visitedBounds: number;
  pool: { capacity: number; used: number };
  ruins: number;
  ruinTiers: number[];
  expandedRows: number;
  refused: number;
  /** Held GPU record allocations; view and independent casters stay resident. */
  recordBytes: number;
  tiers: number[];
  triangles: number[];
  draws: number;
  casterDraws: number;
  casterTriangles: number;
}
export interface SceneryPopulationStats {
  placed: number;
  tiers: number[];
  triangles: number;
  casters: number;
}
export interface SceneryStats {
  visitedBounds: number;
  kinds: number;
  forest: SceneryPopulationStats;
  understorey: SceneryPopulationStats;
  backdrop: SceneryPopulationStats;
  dressing: SceneryPopulationStats & { cells: number; bytes: number; pending: boolean };
  felled: {
    trees: number;
    falling: boolean;
    stumps: number;
    work: { envelopeRowsTotal: number; snapshotRowsTotal: number; membershipChecks: number };
  };
  draws: number;
}
export interface GrassStats {
  enabled: boolean;
  tiles: number;
  capacity: [number, number];
  clumpBytes: number;
  kinds: string[];
}
export interface FogStats {
  enabled: boolean;
  eyes: number;
  slots: number;
  occluders: number;
  rebuilt: number;
  pending: number;
  rebuiltTotal: number;
  mapBytes: number;
  terrainMapBytes: number;
  tileListBytes: number;
  tiles: [number, number];
}
export interface ScarStats {
  cols: number;
  rows: number;
  cellM: number;
  uploads: number;
  fullUploads: number;
  lastBytes: number;
  totalBytes: number;
  pages: number;
  directoryEntries: number;
  poolWords: number;
  defaultWord: number;
  cacheBounds: [number, number, number, number];
  textureBytes: number;
  directoryBytes: number;
}
export interface SceneryCounts {
  forest: number[];
  understorey: number[];
  backdrop: number[];
  dressing: number[];
  felled: number[];
  stumps: number[];
}
/** Last prepared camera populations. These count objects once, while
 * SubmittedFrame counts each issued surface/pass separately. */
export interface DrawnInstances {
  buildings: number[];
  scenery: SceneryCounts;
  props: { appearance: string; levels: number[] }[];
}
export interface SceneStats {
  width: number;
  height: number;
  frames: number;
  clock: number;
  instances: number;
  view: FrameView;
  models: ModelStats;
  buildings: BuildingStats;
  submitted: SubmittedFrame;
  terrainVisitedNodes: number;
  skirtAndPropsVisitedNodes: number;
  waterVisitedNodes: number;
  worldVertices: number;
  depth: { format: "depth32float"; clearValue: 0; compare: "greater" };
  gpu: GpuFrameTime | null;
  memory: AllocationCounts;
  shadow: { receiverRange: [number, number]; cascades: number };
  fog: FogStats;
  fogEdge: {
    edgeSoftnessPx: number;
    rimWidthPx: number;
    reachPx: number;
    maskView: "none" | "fog" | "ground" | "classes";
  };
  overlay: { glowRadiusPx: number; glowStrength: number };
  scenery: SceneryStats;
  grass: GrassStats;
  effects: { instances: number; capacity: number; lights: number };
  scars: ScarStats;
}
