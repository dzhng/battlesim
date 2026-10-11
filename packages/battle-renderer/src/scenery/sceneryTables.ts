// A map's scenery as word tables for a second client (`frame/wordTables.ts`):
// every forest tree, understorey shrub, backdrop tree and piece of forest-floor
// dressing as the scenery layer instances it (`lod.ts` `treeInstances`), from
// the one placement (`placement.ts`), with the tier rule's numbers. A second
// client places nothing itself: it draws these instances, takes what its side
// knows felled or cleared out of them, and picks tiers by the same rule.
//
// Exported only when a plated run asks (`BattleFrame.uploadedTables`): the
// dressing is laid whole here, where the layer lays only the cells in view.
import { d } from "typegpu";
import { fillRecords, type WordTable } from "../frame/wordTables";
import { DRESSING_GONE } from "./dressing";
import { INSTANCE_FLOATS, treeInstances, type PlacedInstances } from "./lod";
import { TREE_FLOATS, type KindSize, type SceneryPlacement } from "./placement";
import type { FelledRules } from "../terrain/biome";

/** An instance as the layer draws it: `pose` (x, y, z, yaw), `shape`
 *  (scale x, y, z, the heart's height above the foot), `tint` (r, g, b and
 *  the leaf-noise seed), its appearance (an index of `SceneryTables.kinds`),
 *  the simulation's body it stands on (`NO_BODY` for none), and its drawn
 *  height and farthest reach from its origin, metres. */
export const SceneryInstance = d.struct({
  pose: d.vec4f,
  shape: d.vec4f,
  tint: d.vec4f,
  kind: d.u32,
  id: d.u32,
  height: d.f32,
  reach: d.f32,
});

/** A cell of dressing: (i, j) is the ground from `(i, j) · cellM` to
 *  `(i + 1, j + 1) · cellM`, and its pieces are `count` instances of the
 *  dressing table from `first`. */
export const DressingCell = d.struct({ cell: d.vec2i, first: d.u32, count: d.u32 });

/** An instance's `id` when no simulation body stands under it. */
export const NO_BODY = 0xffffffff;

export type SceneryTableName = "forest" | "understorey" | "backdrop" | "dressing" | "dressingCells";

/** The scenery layer's own tier numbers, as it draws. */
export interface SceneryRules {
  /** `biome.trees.lod_px`. */
  lodPx: readonly number[];
  /** The chunks trees and shrubs are bucketed in, metres. */
  chunkM: number;
  casterCoarser: number;
  dressing: {
    /** `biome.forest_floor.dressing`'s tiers and fade. */
    lodPx: readonly number[];
    fadePx: number;
    /** Cells the pool holds, and cells laid a view. */
    slots: number;
    cellsPerView: number;
  };
  /** How a felled tree falls and lies (`biome.trees.felled`). */
  felled: FelledRules;
}

/** The tier rule's numbers: an instance of height h draws at tier t while
 *  `h · pixelsPerMetre / distance` exceeds `lodPx[t]` (else 3), a chunk
 *  `chunkM` square drawing whole at tier 3 when its tallest instance would
 *  from its nearest point; casters draw `casterCoarser` tiers coarser. A
 *  cell of dressing takes the tier of `topM` at its nearest point, is drawn
 *  while that is nearer than where a piece `topM` tall shrinks to `gone` of
 *  `fadePx`, and the layer holds `slots` cells, laying `cellsPerView` a view. */
export interface SceneryTiers {
  lodPx: number[];
  chunkM: number;
  casterCoarser: number;
  dressing: {
    lodPx: number[];
    fadePx: number;
    gone: number;
    cellM: number;
    topM: number;
    reachM: number;
    ground: number[];
    slots: number;
    cellsPerView: number;
  };
}

export interface SceneryTables {
  tables: Record<SceneryTableName, WordTable>;
  /** Appearance names, by an instance's `kind`. */
  kinds: string[];
  tiers: SceneryTiers;
  /** How a tree the side saw fall falls and lies (`felled.ts`). */
  felled: FelledRules;
}

/** The instances a view drew, per population: `kind · 4 + tier`; the
 *  stumps of felled trees per kind alone (a stump has one mesh). */
export interface SceneryCounts {
  forest: number[];
  understorey: number[];
  backdrop: number[];
  dressing: number[];
  felled: number[];
  stumps: number[];
}

function instanceTable(placed: PlacedInstances, ids: Uint32Array | null): WordTable {
  const { records, kinds, heights, reaches } = placed;
  return fillRecords(SceneryInstance, kinds.length, (put, i) => {
    const o = i * INSTANCE_FLOATS;
    put("pose", ...records.subarray(o, o + 4));
    put("shape", ...records.subarray(o + 4, o + 8));
    put("tint", ...records.subarray(o + 8, o + 12));
    put("kind", kinds[i]);
    put("id", ids ? ids[i] : NO_BODY);
    put("height", heights[i]);
    put("reach", reaches[i]);
  });
}

/** The tables of `placement`, its appearances `sizes` big, drawn by `rules`. */
export function sceneryTables(
  placement: SceneryPlacement,
  sizes: readonly KindSize[],
  rules: SceneryRules,
): SceneryTables {
  const field = placement.dressing;
  const laid: number[] = [];
  const cells: number[] = [];
  for (let o = 0; o < field.cells.length; o += 2) {
    const pieces = field.place(field.cells[o], field.cells[o + 1]);
    cells.push(
      field.cells[o],
      field.cells[o + 1],
      laid.length / TREE_FLOATS,
      pieces.length / TREE_FLOATS,
    );
    laid.push(...pieces);
  }
  return {
    tables: {
      forest: instanceTable(treeInstances(placement.forest, sizes), placement.forestIds),
      understorey: instanceTable(treeInstances(placement.understorey, sizes), null),
      backdrop: instanceTable(treeInstances(placement.backdrop, sizes), null),
      dressing: instanceTable(treeInstances(Float32Array.from(laid), sizes), null),
      dressingCells: fillRecords(DressingCell, cells.length / 4, (put, c) => {
        put("cell", cells[c * 4], cells[c * 4 + 1]);
        put("first", cells[c * 4 + 2]);
        put("count", cells[c * 4 + 3]);
      }),
    },
    kinds: [...placement.kinds],
    felled: rules.felled,
    tiers: {
      lodPx: [...rules.lodPx],
      chunkM: rules.chunkM,
      casterCoarser: rules.casterCoarser,
      dressing: {
        lodPx: [...rules.dressing.lodPx],
        fadePx: rules.dressing.fadePx,
        gone: DRESSING_GONE,
        cellM: field.cellM,
        topM: field.topM,
        reachM: field.reachM,
        ground: [...field.ground],
        slots: rules.dressing.slots,
        cellsPerView: rules.dressing.cellsPerView,
      },
    },
  };
}
