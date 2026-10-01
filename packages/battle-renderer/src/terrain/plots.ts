// The patchwork's plots: a seeded binary split of the land into convex
// polygons, cut first along the roads (so fields meet them edge-on, as
// farmland does) and then by the biome's field rules. The split tree is what
// the terrain material walks per pixel: each node is a line, each leaf a plot,
// and the nearest cut on the way down is the distance to the plot's edge,
// where the verge grows. No texture holds the plots, so an edge is as sharp
// at ground level as from the strategic height.
import { vec2, type Vec2 } from "math";
import { polygon2 } from "math/shapes";
import { mulberry32, random, type RandomGenerator } from "math/random";
import type { Rgb } from "../light/sceneLight";
import type { Biome } from "./biome";
import { plotGuideEdges, type SurfaceGeometry } from "./surfaces";

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

/** What the plots are cut around, from the simulation's static export. */
export interface PlotSite extends SurfaceGeometry {
  /** Map box `[minX, minY, maxX, maxY]`. */
  map: readonly [number, number, number, number];
  /** Building centres. */
  buildings: readonly Vec2[];
}

const DEG = Math.PI / 180;
/** A cut leaves both sides at least this much area (m²), so a road that grazes
 *  a plot's corner does not carve a sliver. */
const MIN_PIECE_AREA_M2 = 100;
/** A road cuts a plot only where it runs along at least this share of the
 *  cut's chord through it; a larger plot is cut by size first. */
const ROAD_CHORD_SHARE = 0.7;
/** A split tree deeper than this is a runaway (bad rules), not a patchwork. */
export const MAX_PLOT_DEPTH = 64;

const _cut_normal = vec2.create();
const _centroid = vec2.create();

/** Keeps the part of convex `poly` where `n·p >= c` (Sutherland–Hodgman). */
function clipHalfPlane(poly: number[], n: Vec2, c: number): number[] {
  const out: number[] = [];
  const count = poly.length / 2;
  for (let i = 0; i < count; i++) {
    const ax = poly[i * 2],
      ay = poly[i * 2 + 1];
    const j = (i + 1) % count;
    const bx = poly[j * 2],
      by = poly[j * 2 + 1];
    const da = n[0] * ax + n[1] * ay - c;
    const db = n[0] * bx + n[1] * by - c;
    if (da >= 0) out.push(ax, ay);
    if (da >= 0 !== db >= 0) {
      const t = da / (da - db);
      out.push(ax + (bx - ax) * t, ay + (by - ay) * t);
    }
  }
  return out;
}

/** Extent of `poly` along unit `axis`: `[min, max]`. */
function span(poly: number[], ax: number, ay: number): [number, number] {
  let lo = Infinity,
    hi = -Infinity;
  for (let i = 0; i < poly.length; i += 2) {
    const d = poly[i] * ax + poly[i + 1] * ay;
    lo = Math.min(lo, d);
    hi = Math.max(hi, d);
  }
  return [lo, hi];
}

/** Whether a road segment cuts `poly` into two real pieces. Its line must
 *  run along the road for most of its chord through the plot, or the plot be
 *  no longer than `shortChord` that way, so a cut never runs far past a
 *  road's end or bend across open fields. */
function roadCuts(
  poly: number[],
  ax: number,
  ay: number,
  bx: number,
  by: number,
  shortChord: number,
): boolean {
  const len = Math.hypot(bx - ax, by - ay);
  if (len === 0) return false;
  const n = vec2.set(_cut_normal, -(by - ay) / len, (bx - ax) / len);
  const c = n[0] * ax + n[1] * ay;
  // Clip the road's line, a + t (b − a), to the polygon: its chord [t0, t1].
  let t0 = -Infinity,
    t1 = Infinity;
  const count = poly.length / 2;
  const area = polygon2.signedArea(poly, count);
  for (let i = 0; i < count && t0 < t1; i++) {
    const px = poly[i * 2],
      py = poly[i * 2 + 1];
    const j = (i + 1) % count;
    // Inward normal of the edge.
    const ex = poly[j * 2] - px,
      ey = poly[j * 2 + 1] - py;
    const [nx, ny] = area >= 0 ? [-ey, ex] : [ey, -ex];
    const fa = nx * (ax - px) + ny * (ay - py);
    const slope = nx * (bx - ax) + ny * (by - ay);
    if (slope === 0) {
      if (fa < 0) return false;
    } else if (slope > 0) t0 = Math.max(t0, -fa / slope);
    else t1 = Math.min(t1, -fa / slope);
  }
  if (!(t1 > t0)) return false;
  const along = Math.min(t1, 1) - Math.max(t0, 0);
  if (along * len < 1) return false;
  if (along < ROAD_CHORD_SHARE * (t1 - t0) && (t1 - t0) * len > shortChord) return false;
  const front = clipHalfPlane(poly, n, c);
  const back = clipHalfPlane(poly, vec2.negate(_cut_normal, n), -c);
  return (
    front.length >= 6 &&
    back.length >= 6 &&
    polygon2.area(front, front.length / 2) >= MIN_PIECE_AREA_M2 &&
    polygon2.area(back, back.length / 2) >= MIN_PIECE_AREA_M2
  );
}

/** Split the map and its surroundings into the biome's plots. Deterministic
 *  for a biome seed and a site. */
export function generatePlots(site: PlotSite, biome: Biome): PlotTree {
  const rules = biome.field_rules;
  const state = mulberry32.create(biome.seed);
  const rng: RandomGenerator = () => mulberry32.sample(state);
  const [mx0, my0, mx1, my1] = site.map;
  const region = [
    mx0 - rules.extent_m,
    my0 - rules.extent_m,
    mx1 + rules.extent_m,
    my1 + rules.extent_m,
  ] as const;
  const nodes: number[] = [];
  const plots: Plot[] = [];
  const totalWeight = biome.plots.reduce((s, p) => s + p.weight, 0);
  const settlement = biome.plots.findIndex((p) => p.name === rules.settlement_kind);
  const settlementSq = rules.settlement_m ** 2;
  const roadEdges = plotGuideEdges(site);
  const roadCount = roadEdges.length / 4;
  let depth = 0;

  const pickKind = (centre: Vec2) => {
    if (site.buildings.some((b) => vec2.squaredDistance(b, centre) <= settlementSq))
      return settlement;
    let roll = rng() * totalWeight;
    for (let k = 0; k < biome.plots.length; k++) {
      roll -= biome.plots[k].weight;
      if (roll < 0) return k;
    }
    return biome.plots.length - 1;
  };

  const leaf = (poly: number[], heading: number): number => {
    const centre = polygon2.centroid(_centroid, poly, poly.length / 2);
    const kind = pickKind(centre);
    const palette = biome.palettes[biome.plots[kind].palette];
    const base = palette[Math.floor(rng() * palette.length)];
    const value = 1 + (rng() * 2 - 1) * rules.colour_jitter;
    const colour = base.map((ch) =>
      Math.min(1, Math.max(0, ch * value * (1 + (rng() * 2 - 1) * rules.colour_jitter * 0.4))),
    ) as unknown as Rgb;
    // Rows run along the plot's longer side.
    const [ux, uy] = [Math.cos(heading), Math.sin(heading)];
    const [u0, u1] = span(poly, ux, uy);
    const [v0, v1] = span(poly, -uy, ux);
    const across: Vec2 = u1 - u0 >= v1 - v0 ? [-uy, ux] : [ux, uy];
    plots.push({ kind, colour, across, outline: poly });
    return -1 - (plots.length - 1);
  };

  /** Adds a node cutting `poly` by `n·p = c` and returns its index. */
  const cut = (
    poly: number[],
    nx: number,
    ny: number,
    c: number,
    heading: number,
    level: number,
  ): number => {
    const at = nodes.length / NODE_FLOATS;
    nodes.push(nx, ny, c, 0, 0);
    const n = vec2.set(_cut_normal, nx, ny);
    const front = clipHalfPlane(poly, n, c);
    const back = clipHalfPlane(poly, vec2.set(_cut_normal, -nx, -ny), -c);
    nodes[at * NODE_FLOATS + 3] = split(front, heading, level + 1);
    nodes[at * NODE_FLOATS + 4] = split(back, heading, level + 1);
    return at;
  };

  const split = (poly: number[], heading: number, level: number): number => {
    depth = Math.max(depth, level);
    if (level >= MAX_PLOT_DEPTH) throw new Error("field_rules: the plot split runs away");
    // Roads first: the first road segment crossing this plot cuts it.
    for (let r = 0; r < roadCount; r++) {
      const o = r * 4;
      const [ax, ay, bx, by] = [roadEdges[o], roadEdges[o + 1], roadEdges[o + 2], roadEdges[o + 3]];
      if (!roadCuts(poly, ax, ay, bx, by, rules.size_m[1])) continue;
      const len = Math.hypot(bx - ax, by - ay);
      const nx = -(by - ay) / len,
        ny = (bx - ax) / len;
      return cut(poly, nx, ny, nx * ax + ny * ay, heading, level);
    }
    const [ux, uy] = [Math.cos(heading), Math.sin(heading)];
    const [u0, u1] = span(poly, ux, uy);
    const [v0, v1] = span(poly, -uy, ux);
    const [lu, lv] = [u1 - u0, v1 - v0];
    const [long, short] = lu >= lv ? [lu, lv] : [lv, lu];
    const area = polygon2.area(poly, poly.length / 2);
    const side = random.float(rng, rules.size_m[0], rules.size_m[1]);
    // A large tract turns its heading; its plots keep it.
    const turned =
      long > rules.tract_m
        ? heading + random.float(rng, -1, 1) * rules.orientation_jitter_deg * DEG
        : heading;
    if (area <= side * side && long <= rules.max_aspect * short) return leaf(poly, heading);
    if (short < 2 * rules.min_width_m && long < 2 * rules.min_width_m) return leaf(poly, heading);
    // Across the length by default; along it into strips, by chance, while
    // the strips stay wide enough and not too long.
    const strips =
      rng() < rules.strip_chance &&
      short >= 2 * rules.min_width_m &&
      long <= rules.max_aspect * (short / 2);
    const acrossU = lu >= lv !== strips;
    if (!acrossU && lv < 2 * rules.min_width_m) return leaf(poly, heading);
    if (acrossU && lu < 2 * rules.min_width_m) return leaf(poly, heading);
    const lean = random.float(rng, -1, 1) * rules.cut_jitter_deg * DEG;
    const theta = turned + (acrossU ? 0 : Math.PI / 2) + lean;
    const [nx, ny] = [Math.cos(theta), Math.sin(theta)];
    const [s0, s1] = span(poly, nx, ny);
    const extent = s1 - s0;
    // Keep both pieces at least the minimum width.
    const lo = Math.max(rules.cut_range[0], rules.min_width_m / extent);
    const hi = Math.min(rules.cut_range[1], 1 - rules.min_width_m / extent);
    if (lo > hi) return leaf(poly, heading);
    return cut(poly, nx, ny, s0 + random.float(rng, lo, hi) * extent, turned, level);
  };

  const root = [
    region[0],
    region[1],
    region[2],
    region[1],
    region[2],
    region[3],
    region[0],
    region[3],
  ];
  const top = split(root, rules.orientation_deg * DEG, 1);
  if (top < 0) {
    // A single plot: one node whose both sides are it.
    nodes.push(1, 0, -Infinity, top, top);
  }
  return { region, nodes: Float64Array.from(nodes), plots, depth };
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
