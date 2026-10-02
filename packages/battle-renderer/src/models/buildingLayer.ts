// Buildings on the GPU: the buffers behind `buildingPlacements.ts` and the
// draws it chooses. A part of the models layer (`modelLayer.ts`), which owns
// the kits' meshes, the material and the pipelines: a building's module is
// drawn exactly as any static model is, from a record of the same layout.
//
// Three record buffers, each written where it changes and nowhere else: the
// coarse population's (once a map, and a fallen building's records when
// knowledge changes), the pool's (a chunk's rows when it enters), and the
// fallen population's (rebuilt with knowledge). A view change chooses the
// draws on the CPU, a walk over chunks, never over modules; a still camera
// does nothing.
//
// A draw is a range of one buffer as one mesh range. The meshes of a kit's
// modules at a tier share a vertex and an index buffer, so consecutive draws
// usually change only the range: those go straight to the raw pass, and the
// pipeline's whole state is set again only when a buffer changes.
import type { InstalledTemplateArt } from "@packages/scene-assets/src/loader";
import type { Bounds } from "@packages/scene-assets/src/schema";
import { TIER_COUNT } from "@packages/scene-assets/src/schema";
import type { DetailView } from "../frame/detailView";
import type { GpuRegistry } from "../frame/registry";
import type { SunShadow } from "../frame/staticChunks";
import {
  buildingCasters,
  buildingDraws,
  COARSE,
  createBuildingScene,
  POOL,
  selectBuildings,
  setFallenBuildings,
  type BuildingArt,
  type BuildingScene,
} from "./buildingPlacements";
import type { BuildingStyle, SideBuildings } from "./buildingReferences";
import { MODEL_RECORD_FLOATS } from "./modelInstances";
import { flushPool } from "./placementPool";

const RECORD_BYTES = MODEL_RECORD_FLOATS * 4;
const VERTEX_USAGE = 0x20 | 0x08; // VERTEX | COPY_DST

/** A module's mesh at one tier: a range of its kit's buffers for that tier. */
export interface ModuleMesh {
  vertices: GPUBuffer;
  indices: GPUBuffer;
  first: number;
  count: number;
}

/** How the installed kits answer for the library's modules. */
export interface ModuleSource {
  /** The mesh of module `state` of kit appearance `kit` at `tier`, or null. */
  mesh(kit: string, state: number, tier: number): ModuleMesh | null;
  /** That module's bounds in its own frame, or null when the kit is not installed. */
  bounds(kit: string, state: number): Bounds | null;
}

/** A pipeline bound to a mesh and a record buffer, ready to draw ranges. */
export type BindBuildingDraw = (
  vertices: GPUBuffer,
  records: GPUBuffer,
  indices: GPUBuffer,
) => {
  drawIndexed(
    count: number,
    instances: number,
    first: number,
    baseVertex: number,
    firstInstance: number,
  ): void;
};

export interface BuildingStats {
  /** Buildings drawn from template art, and those of them the side knows fell. */
  buildings: number;
  fallen: number;
  /** Chunks they are bucketed in, and those resident in the pool. */
  chunks: number;
  residentChunks: number;
  /** The pool: the records it can hold, holds now, and its buffer's bytes. */
  pool: { capacity: number; used: number; bytes: number };
  /** Records in the coarse population (the whole map's), its buffer's
   *  bytes, and records in the fallen one. */
  coarse: number;
  coarseBytes: number;
  ruins: number;
  /** Per tier: module instances and triangles drawn into the view. */
  tiers: number[];
  triangles: number[];
  /** Draw calls into the view, and into each of the sun's cascades with the
   *  triangles those draw. */
  draws: number;
  casterDraws: number;
  casterTriangles: number;
  /** The last view change: rows expanded into the pool, the time choosing
   *  and expanding took, the bytes it uploaded, near chunks drawn coarse for
   *  want of room, and whether chunks still wait their turn. */
  expandedRows: number;
  selectMs: number;
  uploadBytes: number;
  refused: number;
  pending: boolean;
  /** Since the buildings were set: rows expanded, bytes uploaded, and how
   *  long building the references' scene took. */
  totalExpandedRows: number;
  totalUploadBytes: number;
  buildMs: number;
}

/** A list of draws, grown once. */
interface DrawList {
  meshes: ModuleMesh[];
  /** Per draw: source buffer, first record, records. */
  ranges: Int32Array;
  count: number;
}
const createDrawList = (): DrawList => ({ meshes: [], ranges: new Int32Array(3 * 64), count: 0 });

export function createBuildingLayer(
  device: GPUDevice,
  registry: GpuRegistry,
  style: BuildingStyle,
) {
  let art: BuildingArt | null = null;
  /** Per library module and tier. */
  let meshes: (ModuleMesh | null)[][] = [];
  let buildings: SideBuildings | null = null;
  let scene: BuildingScene | null = null;
  let scope: GpuRegistry | null = null;
  let coarseRecords: GPUBuffer | null = null;
  let poolRecords: GPUBuffer | null = null;
  /** The fallen population's records, in a scope of their own: replaced
   *  whenever knowledge changes, gone when nothing has fallen. */
  let ruinScope: GpuRegistry | null = null;
  let ruinRecords: GPUBuffer | null = null;
  let ruinsVersion = -1;
  let viewKey = "";
  let dirty = true;
  let shown = true;
  const view = createDrawList();
  const cast = createDrawList();
  const stats: BuildingStats = {
    buildings: 0,
    fallen: 0,
    chunks: 0,
    residentChunks: 0,
    pool: { capacity: 0, used: 0, bytes: 0 },
    coarse: 0,
    coarseBytes: 0,
    ruins: 0,
    tiers: Array.from({ length: TIER_COUNT }, () => 0),
    triangles: Array.from({ length: TIER_COUNT }, () => 0),
    draws: 0,
    casterDraws: 0,
    casterTriangles: 0,
    expandedRows: 0,
    selectMs: 0,
    uploadBytes: 0,
    refused: 0,
    pending: false,
    totalExpandedRows: 0,
    totalUploadBytes: 0,
    buildMs: 0,
  };

  const recordBuffer = (label: string, records: number) =>
    device.createBuffer({ label, size: Math.max(1, records) * RECORD_BYTES, usage: VERTEX_USAGE });
  const write = (
    buffer: GPUBuffer,
    at: number,
    data: Float32Array,
    first: number,
    count: number,
  ) => {
    device.queue.writeBuffer(
      buffer,
      at * RECORD_BYTES,
      data as Float32Array<ArrayBuffer>,
      first * MODEL_RECORD_FLOATS,
      count * MODEL_RECORD_FLOATS,
    );
    return count * RECORD_BYTES;
  };

  /** Build the scene for the buildings and the art in hand, or none. */
  function rebuild() {
    scope?.release();
    scope = null;
    scene = null;
    coarseRecords = poolRecords = ruinRecords = null;
    ruinScope = null;
    ruinsVersion = -1;
    dirty = true;
    Object.assign(stats, { totalExpandedRows: 0, totalUploadBytes: 0, buildMs: 0 });
    if (!art || !buildings || buildings.placed.template.length === 0) return;
    const started = performance.now();
    scene = createBuildingScene(buildings.placed, art, style);
    setFallenBuildings(scene, buildings.fallen);
    scene.coarseDirty.length = 0;
    scope = registry.scope();
    coarseRecords = scope.own(recordBuffer("building-coarse", scene.coarse.count));
    stats.totalUploadBytes += write(coarseRecords, 0, scene.coarse.records, 0, scene.coarse.count);
    poolRecords = scope.own(recordBuffer("building-pool", scene.pool.capacity));
    stats.buildMs = performance.now() - started;
  }

  function push(list: DrawList, mesh: ModuleMesh, source: number, first: number, count: number) {
    if (list.ranges.length < (list.count + 1) * 3) {
      const grown = new Int32Array(list.ranges.length * 2);
      grown.set(list.ranges);
      list.ranges = grown;
    }
    list.meshes[list.count] = mesh;
    list.ranges.set([source, first, count], list.count * 3);
    list.count++;
  }

  /** The record buffer a draw's source names. */
  const bufferOf = (source: number) =>
    source === COARSE ? coarseRecords! : source === POOL ? poolRecords! : ruinRecords!;

  function drawList(list: DrawList, bind: BindBuildingDraw, raw: GPURenderPassEncoder) {
    let vertices: GPUBuffer | null = null;
    let source = -1;
    for (let i = 0; i < list.count; i++) {
      const mesh = list.meshes[i];
      const [from, first, count] = [
        list.ranges[i * 3],
        list.ranges[i * 3 + 1],
        list.ranges[i * 3 + 2],
      ];
      // The pipeline, its groups and buffers through the typed path once per
      // mesh buffer and record buffer; the ranges after it straight to the pass.
      if (mesh.vertices !== vertices || from !== source) {
        vertices = mesh.vertices;
        source = from;
        bind(mesh.vertices, bufferOf(from), mesh.indices).drawIndexed(
          mesh.count,
          count,
          mesh.first,
          0,
          first,
        );
      } else raw.drawIndexed(mesh.count, count, mesh.first, 0, first);
    }
  }

  return {
    /** The template art a generation installed and how its kits answer for
     *  its modules (`undefined` or a missing kit: no building is drawn). */
    setArt(installed: InstalledTemplateArt | undefined, source: ModuleSource) {
      const library = installed?.library;
      const bounds = installed?.modules.map((m) => source.bounds(m.kit, m.state));
      const whole = !!installed && bounds!.every((b) => b !== null);
      meshes = whole
        ? installed.modules.map((m) =>
            Array.from({ length: TIER_COUNT }, (_, tier) => source.mesh(m.kit, m.state, tier)),
          )
        : [];
      const same = whole ? art?.library === library : art === null;
      if (!same) {
        art = whole ? { library: library!, bounds: bounds as Bounds[] } : null;
        rebuild();
      }
      // The meshes are a new generation's buffers either way.
      dirty = true;
    },
    /** The buildings a side draws from template art (`null`: none). A new
     *  `placed` builds the map's references; a new `fallen` alone changes
     *  only the buildings whose knowledge changed. */
    set(next: SideBuildings | null) {
      const before = buildings;
      buildings = next;
      if (next?.placed !== before?.placed) rebuild();
      else if (scene && next && next.fallen !== before?.fallen) {
        setFallenBuildings(scene, next.fallen);
        dirty = true;
      }
    },
    /** Lab diagnostics: draw the buildings and their shadows, or neither. */
    setShown(on: boolean) {
      shown = on;
    },
    /** Whether chunks wait to be expanded: the frame should draw again. */
    get pending(): boolean {
      return scene?.pending ?? false;
    },
    /** Choose this frame's draws for the camera (`detail`), when anything changed. */
    prepare(detail: DetailView, key: string, shadow: SunShadow) {
      if (!scene || (!dirty && key === viewKey && !scene.pending)) return;
      viewKey = key;
      dirty = false;
      const started = performance.now();
      selectBuildings(scene, detail, shadow);
      stats.selectMs = performance.now() - started;

      let uploaded = 0;
      const changed = scene.coarseDirty;
      for (let i = 0; i < changed.length; i += 2)
        uploaded += write(
          coarseRecords!,
          changed[i],
          scene.coarse.records,
          changed[i],
          changed[i + 1],
        );
      changed.length = 0;
      if (ruinsVersion !== scene.ruinsVersion) {
        ruinsVersion = scene.ruinsVersion;
        ruinScope?.release();
        ruinScope = ruinRecords = null;
        if (scene.ruins) {
          ruinScope = scope!.scope();
          ruinRecords = ruinScope.own(recordBuffer("building-ruins", scene.ruins.count));
          uploaded += write(ruinRecords, 0, scene.ruins.records, 0, scene.ruins.count);
        }
      }
      const { pool } = scene;
      flushPool(pool, (first, count) => {
        uploaded += write(poolRecords!, first, pool.records, first, count);
      });

      view.count = cast.count = 0;
      stats.tiers.fill(0);
      stats.triangles.fill(0);
      stats.casterTriangles = 0;
      buildingDraws(scene, (source, module, tier, first, count) => {
        const mesh = meshes[module]?.[tier];
        if (!mesh) return;
        push(view, mesh, source, first, count);
        stats.tiers[tier] += count;
        stats.triangles[tier] += (count * mesh.count) / 3;
      });
      buildingCasters(scene, (source, module, tier, first, count) => {
        const mesh = meshes[module]?.[tier];
        if (!mesh) return;
        push(cast, mesh, source, first, count);
        stats.casterTriangles += (count * mesh.count) / 3;
      });
      stats.expandedRows = scene.expandedRows;
      stats.uploadBytes = uploaded;
      stats.totalExpandedRows += scene.expandedRows;
      stats.totalUploadBytes += uploaded;
    },
    /** Every building into the view (the depth prepass and the colour pass). */
    draw(bind: BindBuildingDraw, raw: GPURenderPassEncoder) {
      if (scene && shown) drawList(view, bind, raw);
    },
    /** Every casting building into one of the sun's cascades. */
    drawCasters(bind: BindBuildingDraw, raw: GPURenderPassEncoder) {
      if (scene && shown) drawList(cast, bind, raw);
    },
    stats(): BuildingStats {
      return {
        ...stats,
        buildings: scene?.placed.template.length ?? 0,
        fallen: scene ? scene.fallen.reduce((n, f) => n + f, 0) : 0,
        chunks: scene?.coarse.chunks.chunks.length ?? 0,
        residentChunks: scene?.resident.length ?? 0,
        pool: {
          capacity: scene?.pool.capacity ?? 0,
          used: scene?.pool.used ?? 0,
          bytes: poolRecords?.size ?? 0,
        },
        coarse: scene?.coarse.count ?? 0,
        coarseBytes: coarseRecords?.size ?? 0,
        ruins: scene?.ruins?.count ?? 0,
        tiers: [...stats.tiers],
        triangles: [...stats.triangles],
        draws: scene ? view.count : 0,
        casterDraws: scene ? cast.count : 0,
        refused: scene?.refused ?? 0,
        pending: scene?.pending ?? false,
      };
    },
    dispose() {
      scope?.release();
    },
  };
}
export type BuildingLayer = ReturnType<typeof createBuildingLayer>;
