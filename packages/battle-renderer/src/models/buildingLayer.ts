// Buildings on the GPU: the static batches of a presentation frame
// (`crates/presentation`), drawn. A part of the models layer (`modelLayer.ts`),
// which owns the kits' meshes, the material and the pipelines: a building's
// module is drawn exactly as any static model is, from a record of the same
// layout.
//
// Rust resolves every building against the template art library and walks
// the map's chunks for each view; each static batch it hands over is one kit
// module (its `World.appearances` row, marked `placed`: the kit, and the
// module as the state) at one tier, drawn into the view or cast into the
// sun's cascades. A frame whose walk changed anything writes every static
// batch, and this layer uploads their records into one buffer and draws each
// as a range; any other frame names them unchanged, and the buffer is drawn
// as it stands. The frame's other batches are the models layer's.
//
// The meshes of a kit's modules at a tier share a vertex and an index buffer,
// so consecutive draws usually change only the range: those go straight to
// the raw pass, and the pipeline's whole state is set again only when a
// buffer changes.
import { TIER_COUNT } from "@packages/scene-assets/src/schema";
import type { GpuRegistry } from "../frame/registry";
import { Batch, BATCH_WORDS, INSTANCE_WORDS, NONE, type frameViews } from "../records.gen";
import type { WorldAppearance } from "./modelLayer";
import {
  SURFACE_CLASSES,
  partRange,
  surfaceFlags,
  type SurfaceFlags,
  type SurfacePart,
  type SurfaceParts,
} from "./surfaceParts";

const VERTEX_USAGE = 0x20 | 0x08; // VERTEX | COPY_DST
/** A static batch's passes (`Batch.pass`): drawn into the view, cast into
 *  the sun's cascades. */
const VIEW = 0;
const CAST = 1;

/** A module's mesh at one tier: ranges of its kit's buffers for that tier,
 *  one per surface class (`surfaceParts.ts`), `count` indices in all. */
export interface ModuleMesh {
  vertices: GPUBuffer;
  indices: GPUBuffer;
  parts: SurfaceParts;
  count: number;
}

/** How the installed kits answer for a module: the mesh of kit appearance
 *  `kit`'s module `module` at `tier`, or null. */
export type ModuleSource = (kit: string, module: string, tier: number) => ModuleMesh | null;

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

/** What the presentation says of its buildings and their walk
 *  (`Presentation::building_stats`). */
export interface BuildingWalkStats {
  /** Buildings drawn from template art, and those of them the side knows
   *  destroyed (collapsed or gutted). */
  buildings: number;
  fallen: number;
  /** Chunks they are bucketed in, their coarse rows, and the chunks resident
   *  in the pool. */
  chunks: number;
  coarse: number;
  residentChunks: number;
  /** The pool: the rows it can hold, and holds now. */
  pool: { capacity: number; used: number };
  /** The fallen buildings' rows (a ruin's or a gutted shell's), and those
   *  drawn into the view per tier. */
  ruins: number;
  ruinTiers: number[];
  /** The last walk: rows expanded into the pool, near chunks drawn coarse
   *  for want of room, and the time the frame took in Rust. */
  expandedRows: number;
  refused: number;
  selectMs: number;
  /** Since the buildings were loaded: rows expanded, and how long loading took. */
  totalExpandedRows: number;
  buildMs: number;
}

export interface BuildingStats extends BuildingWalkStats {
  /** The record buffer's bytes. */
  recordBytes: number;
  /** Per tier: module instances and triangles drawn into the view. */
  tiers: number[];
  triangles: number[];
  /** Draw calls into the view, and into each of the sun's cascades with the
   *  triangles those draw. */
  draws: number;
  casterDraws: number;
  casterTriangles: number;
  /** The bytes the last written frame uploaded, and all of them since the
   *  buildings were loaded. */
  uploadBytes: number;
  totalUploadBytes: number;
}

export const emptyBuildingWalkStats = (): BuildingWalkStats => ({
  buildings: 0,
  fallen: 0,
  chunks: 0,
  coarse: 0,
  residentChunks: 0,
  pool: { capacity: 0, used: 0 },
  ruins: 0,
  ruinTiers: Array.from({ length: TIER_COUNT }, () => 0),
  expandedRows: 0,
  refused: 0,
  selectMs: 0,
  totalExpandedRows: 0,
  buildMs: 0,
});

/** A list of draws, grown once. */
interface DrawList {
  meshes: ModuleMesh[];
  /** Per draw: first record, records. */
  ranges: Int32Array;
  count: number;
  /** Whether any of its meshes has a surface of each class. */
  surfaces: SurfaceFlags;
}
const createDrawList = (): DrawList => ({
  meshes: [],
  ranges: new Int32Array(2 * 64),
  count: 0,
  surfaces: surfaceFlags(),
});

export function createBuildingLayer(device: GPUDevice, registry: GpuRegistry) {
  let source: ModuleSource | null = null;
  /** The drawables the batches name, by row. */
  let rows: readonly WorldAppearance[] = [];
  /** The static batches last written: `Batch` words each. */
  let batches = new Uint32Array(0);
  let batchCount = 0;
  let scope: GpuRegistry | null = null;
  let buffer: GPUBuffer | null = null;
  let shown = true;
  const lists = [createDrawList(), createDrawList()];
  const tiers = Array.from({ length: TIER_COUNT }, () => 0);
  const triangles = Array.from({ length: TIER_COUNT }, () => 0);
  let casterTriangles = 0;
  let uploadBytes = 0;
  let totalUploadBytes = 0;
  let walk = emptyBuildingWalkStats();

  /** The batches' draws, against the meshes installed now. */
  function relist() {
    for (const list of lists) {
      list.count = 0;
      for (const c of SURFACE_CLASSES) list.surfaces[c] = false;
    }
    tiers.fill(0);
    triangles.fill(0);
    casterTriangles = 0;
    for (let b = 0; b < batchCount; b++) {
      const o = b * BATCH_WORDS;
      const row = rows[batches[o + Batch.appearance]];
      const tier = batches[o + Batch.tier];
      const mesh = row && source?.(row.name, row.state, tier);
      if (!mesh) continue;
      const count = batches[o + Batch.count];
      const view = batches[o + Batch.pass] === VIEW;
      push(lists[view ? VIEW : CAST], mesh, batches[o + Batch.first], count);
      if (view) {
        tiers[tier] += count;
        triangles[tier] += (count * mesh.count) / 3;
      } else casterTriangles += (count * mesh.count) / 3;
    }
  }

  function push(list: DrawList, mesh: ModuleMesh, first: number, count: number) {
    if (list.ranges.length < (list.count + 1) * 2) {
      const grown = new Int32Array(list.ranges.length * 2);
      grown.set(list.ranges);
      list.ranges = grown;
    }
    list.meshes[list.count] = mesh;
    list.ranges.set([first, count], list.count * 2);
    list.count++;
    for (const c of SURFACE_CLASSES) if (mesh.parts[c].count) list.surfaces[c] = true;
  }

  function drawList(
    list: DrawList,
    bind: BindBuildingDraw,
    raw: GPURenderPassEncoder,
    surface: SurfacePart,
    hidden: Readonly<SurfaceFlags>,
  ) {
    const has = list.surfaces;
    if (!buffer || (surface === "solid" ? !has.opaque && !has.room : !has[surface])) return;
    let vertices: GPUBuffer | null = null;
    for (let i = 0; i < list.count; i++) {
      const mesh = list.meshes[i];
      const { first: firstIndex, count: indices } = partRange(mesh.parts, surface, hidden);
      if (!indices) continue;
      const [first, count] = [list.ranges[i * 2], list.ranges[i * 2 + 1]];
      // The pipeline, its groups and buffers through the typed path once per
      // mesh buffer; the ranges after it straight to the pass.
      if (mesh.vertices !== vertices) {
        vertices = mesh.vertices;
        bind(mesh.vertices, buffer, mesh.indices).drawIndexed(indices, count, firstIndex, 0, first);
      } else raw.drawIndexed(indices, count, firstIndex, 0, first);
    }
  }

  return {
    /** How the installed kits answer for the modules (a new generation's
     *  buffers): the batches are drawn from these meshes now. */
    setSource(next: ModuleSource | null) {
      source = next;
      relist();
    },
    /** The drawables a new load's batches name: what the last load's
     *  buildings held is returned. */
    setWorld(world: readonly WorldAppearance[]) {
      if (world === rows) return;
      scope?.release();
      [scope, buffer, rows, totalUploadBytes, batchCount] = [null, null, world, 0, 0];
      relist();
    },
    /** The static batches of a presentation frame (null: none), and what it
     *  says of its walk. Returns whether what is drawn changed. Read at
     *  once: the views last only until the next call into the module. */
    setFrame(frame: ReturnType<typeof frameViews> | null, stats: BuildingWalkStats): boolean {
      walk = stats;
      const all = frame?.batches.count ?? 0;
      const words = frame?.batches.u32;
      // The static batches, and whether this frame wrote them.
      let count = 0;
      let written = false;
      let end = 0;
      for (let b = 0; b < all; b++) {
        const o = b * BATCH_WORDS;
        if (!rows[words![o + Batch.appearance]]?.placed) continue;
        count++;
        const first = words![o + Batch.first];
        if (first !== NONE) {
          written = true;
          end = Math.max(end, first + words![o + Batch.count]);
        }
      }
      if (count ? !written : !batchCount) return false;
      batchCount = count;
      if (batches.length < count * BATCH_WORDS) batches = new Uint32Array(count * BATCH_WORDS * 2);
      for (let b = 0, at = 0; b < all; b++) {
        const o = b * BATCH_WORDS;
        if (!rows[words![o + Batch.appearance]]?.placed) continue;
        batches.set(words!.subarray(o, o + BATCH_WORDS), at);
        at += BATCH_WORDS;
      }
      // The static records lead the frame's: those up to the last are uploaded.
      const records = frame?.instances.f32.subarray(0, end * INSTANCE_WORDS);
      uploadBytes = records?.byteLength ?? 0;
      totalUploadBytes += uploadBytes;
      if (records?.length) {
        // A power of two that holds them: the same view holds the same
        // buffer, however it was reached.
        const size = 2 ** Math.ceil(Math.log2(records.byteLength));
        if (!buffer || buffer.size !== size) {
          scope?.release();
          scope = registry.scope();
          buffer = scope.own(
            device.createBuffer({ label: "building-records", size, usage: VERTEX_USAGE }),
          );
        }
        device.queue.writeBuffer(buffer, 0, records as Float32Array<ArrayBuffer>);
      }
      relist();
      return true;
    },
    /** Lab diagnostics: draw the buildings and their shadows, or neither. */
    setShown(on: boolean) {
      shown = on;
    },
    /** One kind of surface of every building into the view (the depth prepass
     *  and the colour pass), less the classes `hidden` names. */
    draw(
      bind: BindBuildingDraw,
      raw: GPURenderPassEncoder,
      surface: SurfacePart,
      hidden: Readonly<SurfaceFlags>,
    ) {
      if (shown) drawList(lists[VIEW], bind, raw, surface, hidden);
    },
    /** One kind of surface of every casting building into one of the sun's cascades. */
    drawCasters(
      bind: BindBuildingDraw,
      raw: GPURenderPassEncoder,
      surface: SurfacePart,
      hidden: Readonly<SurfaceFlags>,
    ) {
      if (shown) drawList(lists[CAST], bind, raw, surface, hidden);
    },
    /** The module instances the last view drew, per row (a walked module)
     *  and tier (`row · 4 + tier`); none while they are hidden. */
    drawn(): number[] {
      const counts = Array.from({ length: rows.length * TIER_COUNT }, () => 0);
      if (!shown) return counts;
      for (let b = 0; b < batchCount; b++) {
        const o = b * BATCH_WORDS;
        if (batches[o + Batch.pass] !== VIEW) continue;
        counts[batches[o + Batch.appearance] * TIER_COUNT + batches[o + Batch.tier]] +=
          batches[o + Batch.count];
      }
      return counts;
    },
    stats(): BuildingStats {
      return {
        ...walk,
        pool: { ...walk.pool },
        ruinTiers: [...walk.ruinTiers],
        recordBytes: buffer?.size ?? 0,
        tiers: [...tiers],
        triangles: [...triangles],
        draws: lists[VIEW].count,
        casterDraws: lists[CAST].count,
        casterTriangles,
        uploadBytes,
        totalUploadBytes,
      };
    },
    dispose() {
      scope?.release();
    },
  };
}
