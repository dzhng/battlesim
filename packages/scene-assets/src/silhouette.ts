// A unit type's silhouette, rendered from its own model at asset time (the
// user's call: the unit icon is the type's model, never a drawing). The side
// view, front to the right: a hull's model at rest, or a squad's first
// soldiers in their far pose, side by side. The model's triangles are
// rasterised orthographically onto the x–z plane at a fine grid, box-filtered
// to coverage, and traced at half coverage (marching squares) into one filled
// SVG path. Pure CPU and integer-ordered, so the same bundles always give the
// same bytes: `asset icons` writes it and `asset check` and a vitest compare.

import { vec3, type Mat4 } from "math";
import { articulatedWorlds, poseWorlds, skinPositions } from "./pose.ts";
import { decodeBundle } from "./codec.ts";
import { lfsPointerOid, lfsPullCommand } from "./glb.ts";
import {
  bundlePath,
  type ArticulatedBundle,
  type RuntimeCatalog,
  type SkeletonClips,
  type SkinnedBundle,
} from "./schema.ts";
import type { UnitCatalog } from "./units.ts";

/** Triangles in model space (+X forward, +Z up). */
export interface Solid {
  positions: Float32Array;
  indices: ArrayLike<number>;
}

/** What `unitSolids` needs of an installed appearance. */
export type AppearanceLookup = (
  name: string,
) => { bundle: ArticulatedBundle | SkinnedBundle; skeleton: SkeletonClips | null } | null;

/** An appearance lookup over a baked runtime (`assets/runtime/`): `read`
 *  gives a file's bytes by its path under the runtime directory. An LFS
 *  pointer is refused with the pull command. */
export function runtimeLookup(
  runtime: RuntimeCatalog,
  read: (path: string) => Uint8Array,
): AppearanceLookup {
  const decode = (hash: string) => {
    const path = bundlePath(hash);
    const bytes = read(path);
    if (lfsPointerOid(bytes) !== null)
      throw new Error(
        `${path} is a Git LFS pointer; run: ${lfsPullCommand(`assets/runtime/${path}`)}`,
      );
    return decodeBundle(bytes);
  };
  return (name) => {
    const entry = runtime.appearances[name];
    if (!entry || (entry.kind !== "articulated" && entry.kind !== "skinned")) return null;
    const bundle = decode(entry.bundle) as ArticulatedBundle | SkinnedBundle;
    const skeleton =
      bundle.kind === "skinned" && runtime.skeletons[bundle.skeleton]
        ? (decode(runtime.skeletons[bundle.skeleton]) as SkeletonClips)
        : null;
    return { bundle, skeleton };
  };
}

/** Soldiers a squad's silhouette shows, and their spacing along the view. */
const SQUAD_FIGURES = 3;
const SQUAD_SPACING_M = 0.8;
/** Cells along the silhouette's longer side, and fine samples per cell side. */
const CELLS = 160;
const SUPERSAMPLE = 4;
/** Contour simplification tolerance, in cells. */
const SIMPLIFY = 0.3;

/** The posed triangles a type's silhouette is drawn from; null when its
 *  model is not installed. */
export function unitSolids(
  units: UnitCatalog,
  id: string,
  lookup: AppearanceLookup,
): Solid[] | null {
  const type = units.type(id);
  if (units.hull(id)) {
    const found = lookup(type.appearance ?? "");
    if (found?.bundle.kind !== "articulated") return null;
    const worlds = articulatedWorlds(found.bundle.nodes);
    return found.bundle.nodes.map((node, i) =>
      placed(node.tiers[0].positions, worlds[i], 0, node.tiers[0].indices),
    );
  }
  const solids: Solid[] = [];
  const slots = units.slots(id).slice(0, SQUAD_FIGURES);
  for (const [k, kind] of slots.entries()) {
    const soldier = units.soldier(kind);
    const operated = soldier.mounts.find((m) => m.operator_appearance)?.operator_appearance?.active;
    const found = lookup(operated?.[0] ?? soldier.appearance[0] ?? "");
    if (found?.bundle.kind !== "skinned") return null;
    const { bundle, skeleton } = found;
    const skinned = skinPositions(
      bundle.tiers[0],
      bundle.joints,
      poseWorlds(bundle, skeleton, bundle.far_pose),
    );
    // The first slot leads: furthest forward (right).
    const shift = (slots.length - 1 - k) * SQUAD_SPACING_M;
    solids.push(placed(skinned, null, shift, bundle.tiers[0].indices));
  }
  return solids;
}

function placed(
  positions: Float32Array,
  world: Mat4 | null,
  shiftX: number,
  indices: ArrayLike<number>,
): Solid {
  const out = new Float32Array(positions.length);
  const p = vec3.create();
  for (let v = 0; v < positions.length; v += 3) {
    vec3.fromBuffer(p, positions, v);
    if (world) vec3.transformMat4(p, p, world);
    out[v] = p[0] + shiftX;
    out[v + 1] = p[1];
    out[v + 2] = p[2];
  }
  return { positions: out, indices };
}

/** The side-view silhouette of `solids` as an SVG, filled in the text colour. */
export function silhouetteSvg(solids: readonly Solid[]): string {
  let [x0, z0, x1, z1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const s of solids)
    for (let v = 0; v < s.positions.length; v += 3) {
      x0 = Math.min(x0, s.positions[v]);
      x1 = Math.max(x1, s.positions[v]);
      z0 = Math.min(z0, s.positions[v + 2]);
      z1 = Math.max(z1, s.positions[v + 2]);
    }
  if (!(x1 > x0 && z1 > z0)) throw new Error("silhouette of nothing");
  const cell = Math.max(x1 - x0, z1 - z0) / CELLS;
  // One empty cell of margin all round, so every contour closes.
  const w = Math.ceil((x1 - x0) / cell) + 2;
  const h = Math.ceil((z1 - z0) / cell) + 2;
  const coverage = rasterise(solids, x0 - cell, z0 - cell, cell, w, h);
  const loops = contours(coverage, w, h).map((loop) => simplify(loop, SIMPLIFY));
  const fmt = (v: number) => Number(v.toFixed(1)).toString();
  // Samples sit at cell centres; SVG y runs down.
  const d = loops
    .filter((loop) => loop.length >= 3)
    .map((loop) => `M${loop.map(([x, y]) => `${fmt(x + 0.5)} ${fmt(h - 0.5 - y)}`).join("L")}Z`)
    .join("");
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" ` +
    `fill="currentColor" fill-rule="evenodd"><path d="${d}"/></svg>\n`
  );
}

/** Each cell's covered fraction, from `SUPERSAMPLE`² samples at fine-pixel
 *  centres; row-major from the bottom (z0). */
function rasterise(
  solids: readonly Solid[],
  x0: number,
  z0: number,
  cell: number,
  w: number,
  h: number,
): Float32Array {
  const fw = w * SUPERSAMPLE;
  const fh = h * SUPERSAMPLE;
  const fine = new Uint8Array(fw * fh);
  const px = cell / SUPERSAMPLE;
  for (const s of solids) {
    const { positions: p, indices } = s;
    for (let t = 0; t + 2 < indices.length; t += 3) {
      const [a, b, c] = [indices[t] * 3, indices[t + 1] * 3, indices[t + 2] * 3];
      const ax = (p[a] - x0) / px,
        ay = (p[a + 2] - z0) / px;
      const bx = (p[b] - x0) / px,
        by = (p[b + 2] - z0) / px;
      const cx = (p[c] - x0) / px,
        cy = (p[c + 2] - z0) / px;
      const area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
      if (area === 0) continue;
      const sign = area > 0 ? 1 : -1;
      const minX = Math.max(0, Math.floor(Math.min(ax, bx, cx)));
      const maxX = Math.min(fw - 1, Math.ceil(Math.max(ax, bx, cx)));
      const minY = Math.max(0, Math.floor(Math.min(ay, by, cy)));
      const maxY = Math.min(fh - 1, Math.ceil(Math.max(ay, by, cy)));
      for (let y = minY; y <= maxY; y++) {
        const sy = y + 0.5;
        for (let x = minX; x <= maxX; x++) {
          const sx = x + 0.5;
          const e0 = sign * ((bx - ax) * (sy - ay) - (by - ay) * (sx - ax));
          const e1 = sign * ((cx - bx) * (sy - by) - (cy - by) * (sx - bx));
          const e2 = sign * ((ax - cx) * (sy - cy) - (ay - cy) * (sx - cx));
          if (e0 >= 0 && e1 >= 0 && e2 >= 0) fine[y * fw + x] = 1;
        }
      }
    }
  }
  const coverage = new Float32Array(w * h);
  const share = 1 / (SUPERSAMPLE * SUPERSAMPLE);
  for (let y = 0; y < fh; y++)
    for (let x = 0; x < fw; x++)
      if (fine[y * fw + x])
        coverage[Math.floor(y / SUPERSAMPLE) * w + Math.floor(x / SUPERSAMPLE)] += share;
  return coverage;
}

type Point = [number, number];

/**
 * Closed contours of `field` at half coverage, each walked with the inside on
 * its left (marching squares; a saddle keeps its two corners apart). Edges
 * are numbered so each crossing is one key: a horizontal edge from sample
 * (i, j) is 2·(j·w + i), a vertical one 2·(j·w + i) + 1.
 */
function contours(field: Float32Array, w: number, h: number): Point[][] {
  const ISO = 0.5;
  const at = (i: number, j: number) => field[j * w + i];
  const hEdge = (i: number, j: number) => 2 * (j * w + i);
  const vEdge = (i: number, j: number) => 2 * (j * w + i) + 1;
  const point = (edge: number): Point => {
    const k = edge >> 1;
    const [i, j] = [k % w, Math.floor(k / w)];
    const [a, b] = edge & 1 ? [at(i, j), at(i, j + 1)] : [at(i, j), at(i + 1, j)];
    const t = (ISO - a) / (b - a);
    return edge & 1 ? [i, j + t] : [i + t, j];
  };
  // Oriented segments per case, as (from, to) among B(ottom), R(ight), T(op), L(eft).
  const B = 0,
    R = 1,
    T = 2,
    L = 3;
  const CASES: number[][] = [
    [],
    [B, L],
    [R, B],
    [R, L],
    [T, R],
    [B, L, T, R],
    [T, B],
    [T, L],
    [L, T],
    [B, T],
    [R, B, L, T],
    [R, T],
    [L, R],
    [B, R],
    [L, B],
    [],
  ];
  const next = new Map<number, number>();
  for (let j = 0; j + 1 < h; j++)
    for (let i = 0; i + 1 < w; i++) {
      const index =
        (at(i, j) >= ISO ? 1 : 0) |
        (at(i + 1, j) >= ISO ? 2 : 0) |
        (at(i + 1, j + 1) >= ISO ? 4 : 0) |
        (at(i, j + 1) >= ISO ? 8 : 0);
      const edges = [hEdge(i, j), vEdge(i + 1, j), hEdge(i, j + 1), vEdge(i, j)];
      const segs = CASES[index];
      for (let s = 0; s < segs.length; s += 2) next.set(edges[segs[s]], edges[segs[s + 1]]);
    }
  const loops: Point[][] = [];
  const starts = [...next.keys()].sort((a, b) => a - b);
  for (const start of starts) {
    if (!next.has(start)) continue;
    const loop: Point[] = [];
    let edge = start;
    while (next.has(edge)) {
      loop.push(point(edge));
      const to = next.get(edge)!;
      next.delete(edge);
      edge = to;
    }
    loops.push(loop);
  }
  return loops;
}

/** A closed loop simplified (Ramer–Douglas–Peucker) to within `tolerance`,
 *  split at its first point and the point furthest from it. */
function simplify(loop: Point[], tolerance: number): Point[] {
  if (loop.length < 4) return loop;
  let far = 0;
  let best = -1;
  for (let k = 1; k < loop.length; k++) {
    const d = Math.hypot(loop[k][0] - loop[0][0], loop[k][1] - loop[0][1]);
    if (d > best) [best, far] = [d, k];
  }
  const a = rdp(loop.slice(0, far + 1), tolerance);
  const b = rdp([...loop.slice(far), loop[0]], tolerance);
  return [...a.slice(0, -1), ...b.slice(0, -1)];
}

function rdp(points: Point[], tolerance: number): Point[] {
  if (points.length < 3) return points;
  const [a, b] = [points[0], points[points.length - 1]];
  const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
  const len = Math.hypot(dx, dy);
  let worst = -1;
  let at = 0;
  for (let k = 1; k < points.length - 1; k++) {
    const [px, py] = [points[k][0] - a[0], points[k][1] - a[1]];
    const d = len > 0 ? Math.abs(dx * py - dy * px) / len : Math.hypot(px, py);
    if (d > worst) [worst, at] = [d, k];
  }
  if (worst <= tolerance) return [a, b];
  return [
    ...rdp(points.slice(0, at + 1), tolerance).slice(0, -1),
    ...rdp(points.slice(at), tolerance),
  ];
}
