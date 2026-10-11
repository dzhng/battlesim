// A map's buildings as word tables for a second client (`frame/wordTables.ts`):
// every building, and every module instance its template places in each state
// a side can know it in, expanded by the owner the layer draws from
// (`buildingPlacements.ts`), with the tier rule's numbers. A second client
// places nothing itself: it draws these rows, swaps a building's state from
// what its side knows, and picks tiers by the same rule.
//
// Exported only when a plated run asks (`BattleFrame.uploadedTables`): a whole
// map's rows in three states are far more than a frame ever expands.
import { d } from "typegpu";
import { TIER_COUNT } from "@packages/scene-assets/src/schema";
import { TEMPLATE_STATES, templateArt } from "@packages/scene-assets/src/templateLibrary";
import { fillRecords, type WordTable } from "../frame/wordTables";
import { CASTER_COARSER, drawnModules, stateRows, type BuildingScene } from "./buildingPlacements";
import { FRAME_FLOATS, type BuildingStyle } from "./buildingReferences";

/** A placed building: its frame (x, y, z, yaw), its intact bounds in the
 *  world, its owner prop, the chunk it draws coarse in (an index of the
 *  map's chunks, -1 for a building with no coarse row), and its rows in each
 *  state (`TEMPLATE_STATES` order; -1 where its template has none). */
export const BuildingRecord = d.struct({
  frame: d.vec4f,
  min: d.vec3f,
  owner: d.u32,
  max: d.vec3f,
  chunk: d.i32,
  rows: d.vec3i,
});

/** A module instance: placed (scaled, turned by `yaw` about +Z, then moved
 *  to `position`), tinted (linear, times its building's value), its kit module
 *  (an index of `BuildingTables.modules`), its tier mask (bit t: drawn at
 *  tier t), its building and that building's chunk. */
export const BuildingModule = d.struct({
  position: d.vec3f,
  yaw: d.f32,
  scale: d.vec3f,
  kitModule: d.u32,
  tint: d.vec3f,
  tiers: d.u32,
  building: d.u32,
  chunk: d.i32,
});

export type BuildingTableName = "buildings" | (typeof TEMPLATE_STATES)[number];

/** The tier rule's numbers (`presentation.buildings`): a chunk `chunkM`
 *  square takes tier t while a metre at its nearest point covers more than
 *  `lodPxPerM[t]` pixels (else 3), and a resident chunk casts
 *  `casterCoarser` tiers coarser. The pool holds `poolRecords` rows, and a
 *  view change expands at most `expandRows`. */
export interface BuildingTiers {
  lodPxPerM: number[];
  chunkM: number;
  poolRecords: number;
  expandRows: number;
  casterCoarser: number;
}

export interface BuildingTables {
  tables: Record<BuildingTableName, WordTable>;
  /** The library modules the map's templates place, in library order. */
  modules: { kit: string; module: string }[];
  tiers: BuildingTiers;
}

const exported = new WeakMap<BuildingScene, { modules: number[]; index: Map<number, number> }>();

/** The library modules `scene`'s templates place in any state, and each
 *  one's index among them. */
function sceneModules(scene: BuildingScene) {
  let found = exported.get(scene);
  if (!found) {
    const { library } = scene.art;
    const used = new Set<number>();
    for (const id of scene.placed.templates)
      for (const range of Object.values(templateArt(library, id).states))
        for (let r = range.first; r < range.first + range.count; r++)
          used.add(library.rows.module[r]);
    const modules = [...used].sort((a, b) => a - b);
    found = { modules, index: new Map(modules.map((m, i) => [m, i])) };
    exported.set(scene, found);
  }
  return found;
}

/** The tables of `scene`'s buildings, every one in every state its
 *  template has (none without a scene), under `style`'s tiers. */
export function buildingTables(scene: BuildingScene | null, style: BuildingStyle): BuildingTables {
  const count = scene?.placed.template.length ?? 0;
  const { modules, index } = scene ? sceneModules(scene) : { modules: [], index: new Map() };
  const rows = Array.from({ length: count }, () => [0, 0, 0]);
  const states = TEMPLATE_STATES.map((state, s) => {
    const records: number[] = [];
    const meta: number[] = [];
    for (let b = 0; b < count; b++) {
      const before = meta.length;
      const has = stateRows(scene!, b, state, (row, record) => {
        // The model record (`writeRow`): x, y, z, yaw; tint at 8; scale at 12.
        records.push(
          ...record.subarray(0, 4),
          ...record.subarray(12, 15),
          ...record.subarray(8, 11),
        );
        meta.push(
          index.get(scene!.art.library.rows.module[row])!,
          scene!.art.library.rows.tiers[row],
          b,
        );
      });
      rows[b][s] = has ? (meta.length - before) / 3 : -1;
    }
    return fillRecords(BuildingModule, meta.length / 3, (put, i) => {
      const [o, b] = [i * 10, meta[i * 3 + 2]];
      put("position", records[o], records[o + 1], records[o + 2]);
      put("yaw", records[o + 3]);
      put("scale", records[o + 4], records[o + 5], records[o + 6]);
      put("tint", records[o + 7], records[o + 8], records[o + 9]);
      put("kitModule", meta[i * 3]);
      put("tiers", meta[i * 3 + 1]);
      put("building", b);
      put("chunk", scene!.chunkOf[b]);
    });
  });
  const buildings = fillRecords(BuildingRecord, count, (put, b) => {
    const { frames, owners } = scene!.placed;
    const bounds = scene!.bounds.subarray(b * 6, b * 6 + 6);
    put("frame", ...frames.subarray(b * FRAME_FLOATS, (b + 1) * FRAME_FLOATS));
    put("min", ...bounds.subarray(0, 3));
    put("max", ...bounds.subarray(3, 6));
    put("owner", owners[b]);
    put("chunk", scene!.chunkOf[b]);
    put("rows", ...rows[b]);
  });
  const { library } = scene?.art ?? { library: null };
  return {
    tables: {
      buildings,
      ...(Object.fromEntries(TEMPLATE_STATES.map((state, s) => [state, states[s]])) as Record<
        (typeof TEMPLATE_STATES)[number],
        WordTable
      >),
    },
    modules: modules.map((m) => {
      const { kit, module } = library!.modules[m];
      return { kit: library!.kits[kit].appearance, module };
    }),
    tiers: {
      lodPxPerM: [...style.lod_px_per_m],
      chunkM: style.chunk_m,
      poolRecords: style.pool_records,
      expandRows: style.expand_rows,
      casterCoarser: CASTER_COARSER,
    },
  };
}

/** The module instances the last view of `scene` drew, per exported module
 *  (`buildingTables`' `modules`) and tier: `module · TIER_COUNT + tier`. */
export function buildingCounts(scene: BuildingScene | null): number[] {
  if (!scene) return [];
  const { modules, index } = sceneModules(scene);
  const counts = Array.from({ length: modules.length * TIER_COUNT }, () => 0);
  drawnModules(scene, (module, tier, n) => {
    counts[index.get(module)! * TIER_COUNT + tier] += n;
  });
  return counts;
}
