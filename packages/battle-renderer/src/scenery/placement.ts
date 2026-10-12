// SceneryPlacement: where every tree, hedgerow shrub and piece of
// forest-floor dressing stands, as the ground placed it in Rust
// (`crates/presentation/src/static_world/ground/scenery.rs`, which says how:
// a tree on each of the simulation's trunks, hedge under its tree lines,
// hedgerows and copses past the map, the dressing laid a cell at a time).
// The instanced unit is an appearance (`assets/catalog.json`), sized only by
// its unscaled height and crown radius.
import type { Ground } from "@wasm/presentation/presentation_wasm.js";

/** Floats per placed tree or piece of dressing. `kind` indexes
 *  `SceneryPlacement.kinds`; the scales apply to the appearance's own size;
 *  `r, g, b` multiply its albedo. */
export const TREE_FIELD = {
  x: 0,
  y: 1,
  z: 2,
  yaw: 3,
  scaleXY: 4,
  scaleZ: 5,
  kind: 6,
  r: 7,
  g: 8,
  b: 9,
} as const;
export const TREE_FLOATS = 10;

/** An appearance's unscaled size: its top, and its farthest reach from the
 *  trunk's axis. */
export interface KindSize {
  height: number;
  radius: number;
}

export interface SceneryPlacement {
  /** Appearance names, indexed by each tree's `kind`. */
  kinds: readonly string[];
  forest: Float32Array;
  /** Per `forest` tree: the simulation's trunk it stands on (a prop id), so
   *  a published fall finds the tree it felled. */
  forestIds: Uint32Array;
  understorey: Float32Array;
  backdrop: Float32Array;
  dressing: DressingField;
}

/** One number for cell (i, j) of a grid over the map. */
export const cellKey = (i: number, j: number) => (j + 0x8000) * 0x10000 + (i + 0x8000);

/** The forest floors' dressing, laid a cell of ground at a time and only
 *  where it is asked for: a map's forests hold too many pieces to lay whole. */
export interface DressingField {
  cellM: number;
  /** The most pieces a cell holds. */
  capacity: number;
  /** The appearances it lays, as indices into the placement's `kinds`. */
  kinds: readonly number[];
  /** The tallest any piece stands and the farthest any reaches from its
   *  foot, metres. */
  topM: number;
  reachM: number;
  /** The ground's lowest and highest points, metres. */
  ground: readonly [number, number];
  /** The cells a forest reaches, `i, j` pairs: cell (i, j) is the ground
   *  from `(i, j) * cellM` to `(i + 1, j + 1) * cellM`. */
  cells: Int32Array;
  /** The pieces of cell (i, j), `TREE_FLOATS` each: the same every time. */
  place(i: number, j: number): Float32Array;
}

/** The kinds `ground`'s scenery places, in kind order. */
export const sceneryKinds = (ground: Ground): string[] => JSON.parse(ground.scenery_kinds());

/** `ground`'s scenery, its kinds (`sceneryKinds`) sized `sizes`. */
export function placeScenery(ground: Ground, sizes: readonly KindSize[]): SceneryPlacement {
  ground.place_scenery(Float64Array.from(sizes.flatMap((s) => [s.height, s.radius])));
  const [cellM, capacity, topM, reachM, low, high, ...kinds] = ground.dressing();
  return {
    kinds: sceneryKinds(ground),
    forest: ground.trees(0),
    forestIds: ground.forest_ids(),
    understorey: ground.trees(1),
    backdrop: ground.trees(2),
    dressing: {
      cellM,
      capacity,
      kinds,
      topM,
      reachM,
      ground: [low, high],
      cells: ground.dressing_cells(),
      place: (i, j) => ground.dress(i, j),
    },
  };
}
