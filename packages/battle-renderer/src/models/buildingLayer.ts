// Buildings on the GPU: what the Rust building walk (`crates/static-world`,
// through `BuildingWalk`) chooses each view, drawn. A part of the models layer
// (`modelLayer.ts`), which owns the kits' meshes, the material and the
// pipelines: a building's module is drawn exactly as any static model is, from
// a record of the same layout.
//
// Rust resolves every building against the template art library and walks
// the map's chunks for each view: a chunk in view draws its coarse rows, or
// its rows at the tier its nearest point calls for while the residency pool
// holds them, and a building the side has seen destroyed draws its ruin's or
// gutted shell's rows. It hands back the records drawn into the view and into
// the sun's cascades, a list per module and tier; this layer uploads them when
// they change, into one buffer each, and draws each list as a range.
//
// The meshes of a kit's modules at a tier share a vertex and an index buffer,
// so consecutive draws usually change only the range: those go straight to
// the raw pass, and the pipeline's whole state is set again only when a
// buffer changes.
import type { InstalledTemplateArt } from "@packages/scene-assets/src/loader";
import type { Bounds } from "@packages/scene-assets/src/schema";
import { TIER_COUNT } from "@packages/scene-assets/src/schema";
import {
  TEMPLATE_STATES,
  templateKits,
  templateRows,
  type TemplateArtLibrary,
} from "@packages/scene-assets/src/templateLibrary";
import type { DetailView } from "../frame/detailView";
import type { GpuRegistry } from "../frame/registry";
import type { SunShadow } from "../frame/staticChunks";
import type { BuildingStyle, PlacedBuildings, SideBuildings } from "./buildingReferences";
import { INSTANCE_WORDS as MODEL_RECORD_FLOATS } from "../records.gen";
import {
  SURFACE_CLASSES,
  partRange,
  surfaceFlags,
  type SurfaceFlags,
  type SurfacePart,
  type SurfaceParts,
} from "./surfaceParts";

const RECORD_BYTES = MODEL_RECORD_FLOATS * 4;
const VERTEX_USAGE = 0x20 | 0x08; // VERTEX | COPY_DST
/** The walk's two passes: drawn into the view, cast into the sun's cascades. */
const VIEW = 0;
const CAST = 1;

/** The template art a scene draws from. */
export interface BuildingArt {
  library: TemplateArtLibrary;
  /** The library's file, which Rust resolves buildings from. */
  bytes: Uint8Array;
  /** Per library module: its mesh's bounds in its own frame, or null for a
   *  module of a kit that is not installed (a map installs the kits its
   *  templates' rows place: `templateKits`). */
  bounds: readonly (Bounds | null)[];
}

/** Whether every kit `placed` needs is installed in `art`. The buildings and
 *  the kits that draw them arrive apart; a scene is built once both have. */
export function artCovers(placed: PlacedBuildings, art: BuildingArt): boolean {
  const { kits, modules } = art.library;
  const needed = templateKits(art.library, placed.templates);
  return modules.every((m, i) => art.bounds[i] !== null || !needed.has(kits[m.kit].appearance));
}

/** A map's buildings in Rust (the page's presentation module): resolved
 *  once, then walked for each view. */
export interface BuildingWalk {
  /** Per module a count names (`module · 4 + tier`), its library module. */
  modules(): Uint32Array;
  /** What the side knows destroyed (states: 1 ruin, 2 gutted); returns those
   *  whose template has no rows for their state, and then changes nothing. */
  set_fallen(buildings: Uint32Array, states: Uint32Array): Uint32Array;
  /** Walks a view (`[eye; 3, pixels per metre, shadow fall; 2, reach,
   *  sides; 16]`); true when what is drawn or cast changed. */
  select(view: Float64Array): boolean;
  /** The last walk's records for a pass (0 view, 1 cast), and its draws:
   *  library module, tier, first record, count. */
  records(pass: number): Float32Array;
  draws(pass: number): Uint32Array;
  /** Instances drawn per module and tier. */
  counts(): Uint32Array;
  /** Chunks, coarse rows, resident chunks, resident rows, rows expanded,
   *  chunks refused room, fallen rows, fallen rows drawn per tier. */
  stats(): Uint32Array;
  free(): void;
}

/** Resolves a map's buildings against template art into a walk; refuses
 *  what has no art by name. */
export type CreateBuildingWalk = (
  placed: PlacedBuildings,
  art: BuildingArt,
  style: BuildingStyle,
) => BuildingWalk;

/** A module's mesh at one tier: ranges of its kit's buffers for that tier,
 *  one per surface class (`surfaceParts.ts`), `count` indices in all. */
export interface ModuleMesh {
  vertices: GPUBuffer;
  indices: GPUBuffer;
  parts: SurfaceParts;
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
  /** The record buffers' bytes, view and cast. */
  recordBytes: number;
  /** The fallen buildings' rows (a ruin's or a gutted shell's). */
  ruins: number;
  /** Per tier: module instances and triangles drawn into the view, and those
   *  of the instances that are fallen buildings' rows. */
  tiers: number[];
  triangles: number[];
  ruinTiers: number[];
  /** Draw calls into the view, and into each of the sun's cascades with the
   *  triangles those draw. */
  draws: number;
  casterDraws: number;
  casterTriangles: number;
  /** The last view change: rows expanded into the pool, near chunks drawn
   *  coarse for want of room, the time the walk took and the bytes uploaded. */
  expandedRows: number;
  refused: number;
  selectMs: number;
  uploadBytes: number;
  /** Since the buildings were set: rows expanded and bytes uploaded, and how
   *  long resolving them took. */
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
  /** Whether any of its meshes has a surface of each class. */
  surfaces: SurfaceFlags;
}
const createDrawList = (): DrawList => ({
  meshes: [],
  ranges: new Int32Array(3 * 64),
  count: 0,
  surfaces: surfaceFlags(),
});
const clearDrawList = (list: DrawList) => {
  list.count = 0;
  for (const c of SURFACE_CLASSES) list.surfaces[c] = false;
};

export function createBuildingLayer(
  device: GPUDevice,
  registry: GpuRegistry,
  style: BuildingStyle,
  createWalk: CreateBuildingWalk,
) {
  let art: BuildingArt | null = null;
  /** Per library module and tier. */
  let meshes: (ModuleMesh | null)[][] = [];
  let buildings: SideBuildings | null = null;
  let walk: BuildingWalk | null = null;
  let scope: GpuRegistry | null = null;
  /** Per pass: its record buffer, grown to fit. */
  const buffers: (GPUBuffer | null)[] = [null, null];
  let viewKey = "";
  let dirty = true;
  /** The draw lists name meshes the art no longer has: list them again. */
  let relist = true;
  let shown = true;
  const lists = [createDrawList(), createDrawList()];
  const viewWords = new Float64Array(23);
  const stats: BuildingStats = {
    buildings: 0,
    fallen: 0,
    chunks: 0,
    coarse: 0,
    residentChunks: 0,
    pool: { capacity: style.pool_records, used: 0 },
    recordBytes: 0,
    ruins: 0,
    tiers: Array.from({ length: TIER_COUNT }, () => 0),
    triangles: Array.from({ length: TIER_COUNT }, () => 0),
    ruinTiers: Array.from({ length: TIER_COUNT }, () => 0),
    draws: 0,
    casterDraws: 0,
    casterTriangles: 0,
    expandedRows: 0,
    refused: 0,
    selectMs: 0,
    uploadBytes: 0,
    totalExpandedRows: 0,
    totalUploadBytes: 0,
    buildMs: 0,
  };

  /** Resolve the buildings for the art in hand, or none. */
  function rebuild() {
    walk?.free();
    walk = null;
    scope?.release();
    scope = null;
    buffers.fill(null);
    dirty = true;
    if (!art || !buildings || buildings.placed.template.length === 0) return;
    if (!artCovers(buildings.placed, art)) return;
    const started = performance.now();
    Object.assign(stats, { totalExpandedRows: 0, totalUploadBytes: 0 });
    walk = createWalk(buildings.placed, art, style);
    setFallen(buildings);
    scope = registry.scope();
    stats.buildMs = performance.now() - started;
  }

  /** What `side` knows destroyed, into the walk. A template without rows
   *  for a building's known state is refused by name. */
  function setFallen(side: SideBuildings) {
    const { placed, fallen } = side;
    const refused = walk!.set_fallen(
      Uint32Array.from(fallen, (f) => f.building),
      Uint32Array.from(fallen, (f) => TEMPLATE_STATES.indexOf(f.state)),
    );
    for (const b of refused)
      templateRows(
        art!.library,
        placed.templates[placed.template[b]],
        fallen.find((f) => f.building === b)!.state,
      );
    stats.fallen = fallen.length;
  }

  /** Pass `pass`'s records, uploaded, and its draws listed. */
  function upload(pass: number): number {
    const records = walk!.records(pass);
    const bytes = Math.max(1, records.length / MODEL_RECORD_FLOATS) * RECORD_BYTES;
    if (!buffers[pass] || buffers[pass]!.size < bytes)
      buffers[pass] = scope!.own(
        device.createBuffer({
          label: `building-${pass ? "cast" : "view"}`,
          size: bytes * 2,
          usage: VERTEX_USAGE,
        }),
      );
    device.queue.writeBuffer(buffers[pass]!, 0, records as Float32Array<ArrayBuffer>);
    const list = lists[pass];
    clearDrawList(list);
    const draws = walk!.draws(pass);
    for (let d = 0; d < draws.length; d += 4) {
      const mesh = meshes[draws[d]]?.[draws[d + 1]];
      if (mesh) push(list, mesh, draws[d + 2], draws[d + 3], draws[d + 1]);
    }
    return records.byteLength;
  }

  function push(list: DrawList, mesh: ModuleMesh, first: number, count: number, tier: number) {
    if (list.ranges.length < (list.count + 1) * 3) {
      const grown = new Int32Array(list.ranges.length * 2);
      grown.set(list.ranges);
      list.ranges = grown;
    }
    list.meshes[list.count] = mesh;
    list.ranges.set([list === lists[CAST] ? CAST : VIEW, first, count], list.count * 3);
    list.count++;
    for (const c of SURFACE_CLASSES) if (mesh.parts[c].count) list.surfaces[c] = true;
    if (list === lists[VIEW]) {
      stats.tiers[tier] += count;
      stats.triangles[tier] += (count * mesh.count) / 3;
    } else stats.casterTriangles += (count * mesh.count) / 3;
  }

  function drawList(
    list: DrawList,
    bind: BindBuildingDraw,
    raw: GPURenderPassEncoder,
    surface: SurfacePart,
    hidden: Readonly<SurfaceFlags>,
  ) {
    const has = list.surfaces;
    if (surface === "solid" ? !has.opaque && !has.room : !has[surface]) return;
    let vertices: GPUBuffer | null = null;
    let source = -1;
    for (let i = 0; i < list.count; i++) {
      const mesh = list.meshes[i];
      const { first: firstIndex, count: indices } = partRange(mesh.parts, surface, hidden);
      if (!indices) continue;
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
        bind(mesh.vertices, buffers[from]!, mesh.indices).drawIndexed(
          indices,
          count,
          firstIndex,
          0,
          first,
        );
      } else raw.drawIndexed(indices, count, firstIndex, 0, first);
    }
  }

  return {
    /** The template art a generation installed and how its kits answer for
     *  its modules (`undefined`: no building is drawn). The kits installed
     *  are only those the map's buildings need (`templateKits`): a module of
     *  any other is bound to no state and has no mesh here. */
    setArt(installed: InstalledTemplateArt | undefined, source: ModuleSource) {
      const modules = installed?.modules ?? [];
      const bounds = modules.map((m) => (m.state === null ? null : source.bounds(m.kit, m.state)));
      meshes = modules.map(({ kit, state }) =>
        Array.from({ length: TIER_COUNT }, (_, tier) =>
          state === null ? null : source.mesh(kit, state, tier),
        ),
      );
      const same =
        art?.library === installed?.library &&
        bounds.every((b, i) => (b === null) === (art!.bounds[i] === null));
      if (!same) {
        art = installed ? { library: installed.library, bytes: installed.bytes, bounds } : null;
        rebuild();
      }
      // The meshes are a new generation's buffers either way.
      dirty = relist = true;
    },
    /** The buildings a side draws from template art (`null`: none). A new
     *  `placed` resolves the map's buildings again; a new `fallen` alone
     *  tells the walk what the side knows. */
    set(next: SideBuildings | null) {
      const before = buildings;
      buildings = next;
      if (next?.placed !== before?.placed) rebuild();
      else if (walk && next && next.fallen !== before?.fallen) {
        setFallen(next);
        dirty = true;
      }
    },
    /** Lab diagnostics: draw the buildings and their shadows, or neither. */
    setShown(on: boolean) {
      shown = on;
    },
    /** Choose this frame's draws for the camera (`detail`), when anything changed. */
    prepare(detail: DetailView, key: string, shadow: SunShadow | null) {
      if (!walk || (!dirty && key === viewKey)) return;
      viewKey = key;
      dirty = false;
      const started = performance.now();
      viewWords.set([...detail.eye, detail.pixelsPerMetre]);
      viewWords.set(shadow ? [...shadow.fall, shadow.reach] : [0, 0, -1], 4);
      detail.sides
        .slice(0, 4)
        .forEach((p, i) => viewWords.set([...p.normal, p.constant], 7 + i * 4));
      const changed = walk.select(viewWords);
      stats.selectMs = performance.now() - started;
      stats.uploadBytes = 0;
      if (!changed && !relist) return;
      relist = false;
      stats.tiers.fill(0);
      stats.triangles.fill(0);
      stats.casterTriangles = 0;
      stats.uploadBytes = upload(VIEW) + upload(CAST);
      const [chunks, coarse, resident, used, expanded, refused, ruins, ...ruinTiers] = walk.stats();
      Object.assign(stats, {
        chunks,
        coarse,
        residentChunks: resident,
        expandedRows: expanded,
        refused,
        ruins,
        ruinTiers,
      });
      stats.totalExpandedRows += expanded;
      stats.totalUploadBytes += stats.uploadBytes;
      stats.pool.used = used;
      stats.recordBytes = (buffers[VIEW]?.size ?? 0) + (buffers[CAST]?.size ?? 0);
    },
    /** One kind of surface of every building into the view (the depth prepass
     *  and the colour pass), less the classes `hidden` names. */
    draw(
      bind: BindBuildingDraw,
      raw: GPURenderPassEncoder,
      surface: SurfacePart,
      hidden: Readonly<SurfaceFlags>,
    ) {
      if (walk && shown) drawList(lists[VIEW], bind, raw, surface, hidden);
    },
    /** One kind of surface of every casting building into one of the sun's cascades. */
    drawCasters(
      bind: BindBuildingDraw,
      raw: GPURenderPassEncoder,
      surface: SurfacePart,
      hidden: Readonly<SurfaceFlags>,
    ) {
      if (walk && shown) drawList(lists[CAST], bind, raw, surface, hidden);
    },
    /** The module instances the last view drew, per walked module and tier
     *  (`module · 4 + tier`); none while they are hidden. */
    drawn(): number[] {
      const counts = walk ? [...walk.counts()] : [];
      return shown ? counts : counts.fill(0);
    },
    stats(): BuildingStats {
      return {
        ...stats,
        buildings: walk ? (buildings?.placed.template.length ?? 0) : 0,
        pool: { ...stats.pool },
        tiers: [...stats.tiers],
        triangles: [...stats.triangles],
        ruinTiers: [...stats.ruinTiers],
        draws: walk ? lists[VIEW].count : 0,
        casterDraws: walk ? lists[CAST].count : 0,
      };
    },
    dispose() {
      walk?.free();
      scope?.release();
    },
  };
}
