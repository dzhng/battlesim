// The terrain's GPU source: its tables uploaded from the presented ground,
// its scars, and the bind group the ground's material reads
// (`terrainLayout`).
import { tgpu, d } from "typegpu";
import type { PresentedGround } from "../terrain/terrainSurface";
import { SURFACE_FLOATS } from "../terrain/surfaceField";
import type { GpuRegistry, GpuSlot } from "./registry";
import type { WaterDetail } from "./targets";
import { createWaterNormals } from "./waterNormals";
import {
  createScarTexture,
  type ScarRegion,
  type ScarStats,
  type GroundMarks,
} from "./scarTexture";
import { SCAR_PARAMS_WORDS } from "../records.gen";

import {
  NODE_WORDS,
  PLOT_WORDS,
  PlotNode,
  PlotRecord,
  ScarParams,
  SurfaceRecord,
  TerrainParams,
  terrainLayout,
} from "./terrainLayout";

type Root = ReturnType<typeof tgpu.initFromDevice>;

const EMPTY_SCAR_REGIONS: readonly ScarRegion[] = [];
const NO_SCARS: Omit<ScarStats, "textureBytes" | "directoryBytes"> = {
  cols: 0,
  rows: 0,
  cellM: 1,
  uploads: 0,
  fullUploads: 0,
  lastBytes: 0,
  totalBytes: 0,
  pages: 0,
  directoryEntries: 1,
  poolWords: 0,
  defaultWord: 0,
  cacheBounds: [0, 0, 0, 0],
};

/** The terrain's GPU tables, rebuilt whole when the world is set. */
export function createTerrainSource(root: Root, registry: GpuRegistry, water: WaterDetail) {
  const params = registry.own(root.createBuffer(TerrainParams).$usage("uniform"));
  // Each table holds at least one record: a storage binding cannot be empty.
  const nodeBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(PlotNode, Math.max(1, n))).$usage("storage");
  const plotBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(PlotRecord, Math.max(1, n))).$usage("storage");
  const surfaceBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(SurfaceRecord, Math.max(1, n))).$usage("storage");
  const indexBuffer = (n: number) =>
    root.createBuffer(d.arrayOf(d.u32, Math.max(1, n))).$usage("storage");
  const nodes: GpuSlot<ReturnType<typeof nodeBuffer>> = registry.slot();
  const plots: GpuSlot<ReturnType<typeof plotBuffer>> = registry.slot();
  const surfaces: GpuSlot<ReturnType<typeof surfaceBuffer>> = registry.slot();
  const surfaceIndex: GpuSlot<ReturnType<typeof indexBuffer>> = registry.slot();
  const scarParams = registry.own(root.createBuffer(ScarParams).$usage("uniform"));
  // The scar params as one record: the presentation's words for the grid and
  // cache, then the biome's look (`ground::tables`).
  const scarWords = new Uint32Array(d.sizeOf(ScarParams) / 4);
  new Float32Array(scarWords.buffer).fill(1, SCAR_PARAMS_WORDS, SCAR_PARAMS_WORDS + 4);
  /** The learned ground the scars were last uploaded from. */
  let marks: GroundMarks | null = null;
  const groupOf = () =>
    root.createBindGroup(terrainLayout, {
      params,
      nodes: nodes.current!,
      plots: plots.current!,
      surfaces: surfaces.current!,
      surfaceIndex: surfaceIndex.current!,
      scarParams,
      scars: scars.texture.createView({ dimension: "2d-array" }),
      scarPages: scars.directory.createView(),
      waterNormals: waterNormals.view,
      waterTiles: waterNormals.sampler,
    });
  const scars = createScarTexture(registry);
  const waterNormals = createWaterNormals(root, registry);
  /** `TerrainParams` as `set` last packed it, its last `vec4u` the frame's view. */
  let look: Uint32Array | null = null;
  let classView = false;
  let roadWear = true;
  /** The ground `set` last drew, and whether its plots carry their texture. */
  let ground: PresentedGround | null = null;
  let textured = true;
  const writeParams = () => {
    if (!look) return;
    look.set([classView ? 1 : 0, roadWear ? 0 : 1, water === "depth" ? 1 : 0, 0], look.length - 4);
    params.write(look.buffer as ArrayBuffer);
  };

  const source = {
    /** Draw the ground's classes in place of its lit colour, or not. */
    setClassView(on: boolean) {
      if (on === classView) return;
      classView = on;
      writeParams();
    },
    /** Draw the roads worn (their patches and grain, their shoulders), or
     *  plain: each kind's flat colour to its edge. True when it changed. */
    setRoadWear(on: boolean): boolean {
      if (on === roadWear) return false;
      roadWear = on;
      writeParams();
      return true;
    },
    /** Draw the plots' own texture (grain, broken rows, wheelings), or their
     *  plain rows alone: a paired cost measure, and a check that the texture
     *  leaves a plot's mean colour be. True when it changed, so what grows on
     *  the ground must regrow. */
    setFieldTexture(on: boolean): boolean {
      if (on === textured) return false;
      textured = on;
      if (ground) plots.current!.write(ground.ground_plots(textured).buffer as ArrayBuffer);
      return true;
    },
    group: null as unknown as ReturnType<typeof groupOf>,
    /** Follow the side's learned ground (null: none); true when the scars
     *  changed, so what grows on them must regrow. */
    setGround(next: GroundMarks | null): boolean {
      if (next === marks) return next?.changed() ?? false;
      marks = next;
      next?.resend();
      next?.changed();
      // No ground draws no scars: the grid is off.
      if (!next) scarParams.write(scarWords.fill(0, 0, SCAR_PARAMS_WORDS).buffer as ArrayBuffer);
      return true;
    },
    scarStats: (): ScarStats => ({
      ...(marks?.stats() ?? NO_SCARS),
      ...scars.bytes(),
    }),
    /** Upload what the presentation says the scars over `region` (none:
     *  the view) need, clipped to it when `clipped`. */
    prepareScars(region?: ScarRegion, clipped = false) {
      if (!marks) return;
      const upload = marks.prepare(region, clipped);
      if (scars.upload(upload)) source.group = groupOf();
      scarWords.set(upload.params);
      scarParams.write(scarWords.buffer as ArrayBuffer);
    },
    scarRegions(): readonly ScarRegion[] {
      return marks?.regions() ?? EMPTY_SCAR_REGIONS;
    },
    /** The ground the scene's presentation resolved; none uploads empty
     *  tables until it has. */
    set(next: PresentedGround | null) {
      ground = next;
      const tables = next?.ground_terrain();
      const upload = (words: Uint32Array | Float32Array) => words.buffer as ArrayBuffer;
      const none = new Uint32Array(PLOT_WORDS);
      const [nodeWords, plotWords] = [
        tables?.nodes() ?? none,
        next?.ground_plots(textured) ?? none,
      ];
      const [records, index] = [
        tables?.surfaces() ?? new Float32Array(SURFACE_FLOATS),
        tables?.surface_index() ?? none,
      ];
      nodes.set(nodeBuffer(nodeWords.length / NODE_WORDS)).write(upload(nodeWords));
      plots.set(plotBuffer(plotWords.length / PLOT_WORDS)).write(upload(plotWords));
      surfaces.set(surfaceBuffer(records.length / SURFACE_FLOATS)).write(upload(records));
      surfaceIndex.set(indexBuffer(index.length)).write(upload(index));
      look = new Uint32Array(d.sizeOf(TerrainParams) / 4);
      if (tables) {
        const words = tables.params();
        // All but its `view`; a short one would draw zeros.
        if (words.length !== look.length - 4) throw new Error("terrain params: bad length");
        look.set(words);
        scarWords.set(tables.scars(), SCAR_PARAMS_WORDS);
        tables.free();
      }
      writeParams();
      scarParams.write(scarWords.buffer as ArrayBuffer);
      source.group = groupOf();
    },
  };
  return source;
}
export type TerrainSource = ReturnType<typeof createTerrainSource>;
