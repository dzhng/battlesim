// The patchwork's plots, as the ground's seeded split cut them
// (`crates/static-world/src/ground/plots.rs`, which says how). The split tree
// is what the terrain material walks per pixel: each node is a line, each
// leaf a plot, and the nearest cut on the way down is the distance to the
// plot's edge, where the verge grows.
import type { Vec2 } from "math";
import type { Ground } from "@wasm/presentation/presentation_wasm.js";
import type { Rgb } from "../light/sceneLight";

/** A leaf of the split: one field, meadow or ploughed plot. */
export interface Plot {
  /** Index into `biome.plots`. */
  kind: number;
  /** sRGB, jittered from the kind's palette. */
  colour: Rgb;
  /** Unit vector across the plot's rows (the rows run along its length). */
  across: Vec2;
  /** Convex outline, counter-clockwise, flat `[x0, y0, x1, y1, …]`. */
  outline: number[];
}

/** Floats per node: the cut's unit normal and offset, then the children. */
export const NODE_FLOATS = 5;

export interface PlotTree {
  /** The split region: the map box widened by `field_rules.extent_m`. */
  region: readonly [number, number, number, number];
  /** Per node: `nx, ny, c, front, back`. A point `p` is in front where
   *  `nx·px + ny·py >= c`. A child `>= 0` is a node index; `-1 - k` is plot k. */
  nodes: Float64Array;
  plots: Plot[];
  /** The longest path from the root to a plot, in nodes. */
  depth: number;
}

/** A split tree deeper than this is a runaway (bad rules), not a patchwork. */
export const MAX_PLOT_DEPTH = 64;
/** Numbers per plot in the ground's table (`Ground.plots`). */
const PLOT_NUMBERS = 8;

/** The plots `ground` split. */
export function plotTree(ground: Ground): PlotTree {
  const [x0, y0, x1, y1, depth] = ground.plot_region();
  const table = ground.plots();
  const outlines = ground.plot_outlines();
  const plots: Plot[] = [];
  for (let o = 0; o < table.length; o += PLOT_NUMBERS) {
    const p = table.subarray(o, o + PLOT_NUMBERS);
    plots.push({
      kind: p[0],
      colour: [p[1], p[2], p[3]],
      across: [p[4], p[5]],
      outline: Array.from(outlines.subarray(p[6], p[7])),
    });
  }
  return { region: [x0, y0, x1, y1], nodes: ground.plot_nodes(), plots, depth };
}

/** The plot under (x, y) and the distance to its nearest edge, walking the
 *  split tree exactly as the terrain material does. `null` outside the region. */
export function plotAt(
  tree: PlotTree,
  x: number,
  y: number,
): { plot: number; edge: number } | null {
  const [x0, y0, x1, y1] = tree.region;
  if (x < x0 || x > x1 || y < y0 || y > y1) return null;
  let node = 0;
  let edge = Math.min(x - x0, x1 - x, y - y0, y1 - y);
  for (let step = 0; step < MAX_PLOT_DEPTH; step++) {
    const o = node * NODE_FLOATS;
    const s = tree.nodes[o] * x + tree.nodes[o + 1] * y - tree.nodes[o + 2];
    edge = Math.min(edge, Math.abs(s));
    const child = s >= 0 ? tree.nodes[o + 3] : tree.nodes[o + 4];
    if (child < 0) return { plot: -1 - child, edge };
    node = child;
  }
  throw new Error("plot tree deeper than MAX_PLOT_DEPTH");
}
