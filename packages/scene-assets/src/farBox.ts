// The generated far mesh: what a static prop draws when it is only a few
// pixels tall, unless its source authors one (`_LOD4`). A box without its
// bottom, made from tier 3 so it covers about as many pixels as the prop does
// and carries its average look:
// - its height is tier 3's, and its footprint is sized to tier 3's own
//   plan-view coverage about where that coverage lies, so an airy prop (a
//   lamp, a swing, a washing line) becomes a slender box, not a solid block
//   the size of its bounds;
// - each face takes the opaque material that covers most of tier 3 seen from
//   its side (a wall gets its dark coping on top and its render on the
//   sides), in that material's mean baked colour, so a painted prop keeps its
//   paint slot and each instance its colour.
// A prop whose tier 3 is already about as small as the box gets none (the far
// rules want fewer triangles than tier 3), and an appearance has one in every
// state or in none.

import { vec3, type Vec3 } from "math";
import { triangleCount } from "./build.ts";
import { FAR_TIER, TIER_COUNT, type Material, type MeshData } from "./schema.ts";

/** A box without its bottom: its top and four sides, two triangles each. */
export const FAR_BOX_TRIANGLES = 10;
/** Cells across each orthographic view of tier 3. */
const GRID = 48;

const generated = new WeakSet<MeshData>();

/** `tiers` with a generated far mesh appended, when it has none and its tier
 *  3 draws more triangles than the box. */
export function withFarBox(tiers: MeshData[], materials: readonly Material[]): MeshData[] {
  if (tiers.length !== TIER_COUNT || triangleCount(tiers[3]) <= FAR_BOX_TRIANGLES) return tiers;
  const box = farBox(tiers[3], materials);
  if (!box) return tiers;
  generated.add(box);
  return [...tiers, box];
}

/** An appearance whose states cannot all have a far mesh has no generated
 *  one: every state carries a far mesh, or none does. An authored far mesh
 *  stays, for the far rules to judge. */
export function farBoxesWhole(states: { tiers: MeshData[] }[]): void {
  if (states.every((s) => s.tiers.length > FAR_TIER)) return;
  for (const s of states)
    if (s.tiers.length > FAR_TIER && generated.has(s.tiers[FAR_TIER])) s.tiers.length = FAR_TIER;
}

type Rgba = [number, number, number, number];
interface Tri {
  p: [Vec3, Vec3, Vec3];
  material: number;
  colour: Rgba;
}

function triangles(mesh: MeshData): Tri[] {
  const out: Tri[] = [];
  const P = mesh.positions,
    C = mesh.colors,
    I = mesh.indices;
  for (const d of mesh.draws)
    for (let i = d.first; i < d.first + d.count; i += 3) {
      const v = [I[i], I[i + 1], I[i + 2]];
      const colour: Rgba = [0, 0, 0, 0];
      for (const k of v) for (let c = 0; c < 4; c++) colour[c] += C[k * 4 + c] / 3;
      const p = v.map((k): Vec3 => [P[k * 3], P[k * 3 + 1], P[k * 3 + 2]]);
      out.push({ p: [p[0], p[1], p[2]], material: d.material, colour });
    }
  return out;
}

/** An orthographic view along an axis (0 x, 1 y, 2 z) from its max (+1) or
 *  min (−1) end. */
interface View {
  axis: number;
  from: 1 | -1;
}

/** The nearest triangle in each cell of `view` over the box `lo`–`hi` (−1
 *  where none is), and the view's extents. */
function rasterise(tris: Tri[], view: View, lo: Vec3, hi: Vec3) {
  const [ua, va] = [0, 1, 2].filter((a) => a !== view.axis);
  const u = Math.max(hi[ua] - lo[ua], 1e-6),
    v = Math.max(hi[va] - lo[va], 1e-6);
  const hit = new Int32Array(GRID * GRID).fill(-1);
  const depth = new Float64Array(GRID * GRID).fill(-Infinity);
  tris.forEach((t, n) => {
    const q = t.p.map((p) => [
      ((p[ua] - lo[ua]) / u) * GRID,
      ((p[va] - lo[va]) / v) * GRID,
      p[view.axis] * view.from,
    ]);
    const det =
      (q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1]);
    if (Math.abs(det) < 1e-12) return;
    const x0 = Math.max(0, Math.floor(Math.min(q[0][0], q[1][0], q[2][0])));
    const x1 = Math.min(GRID, Math.ceil(Math.max(q[0][0], q[1][0], q[2][0])));
    const y0 = Math.max(0, Math.floor(Math.min(q[0][1], q[1][1], q[2][1])));
    const y1 = Math.min(GRID, Math.ceil(Math.max(q[0][1], q[1][1], q[2][1])));
    for (let y = y0; y < y1; y++)
      for (let x = x0; x < x1; x++) {
        const [px, py] = [x + 0.5, y + 0.5];
        const b1 =
          ((px - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (py - q[0][1])) / det;
        const b2 =
          ((q[1][0] - q[0][0]) * (py - q[0][1]) - (px - q[0][0]) * (q[1][1] - q[0][1])) / det;
        if (b1 < 0 || b2 < 0 || b1 + b2 > 1) continue;
        const d = (1 - b1 - b2) * q[0][2] + b1 * q[1][2] + b2 * q[2][2];
        if (d > depth[y * GRID + x]) [depth[y * GRID + x], hit[y * GRID + x]] = [d, n];
      }
  });
  return { hit, u, v };
}

/** The opaque material covering most of a view, and its mean colour there. */
function dominant(tris: Tri[], hit: Int32Array, materials: readonly Material[]) {
  const area = new Map<number, number>();
  for (const n of hit)
    if (n >= 0) area.set(tris[n].material, (area.get(tris[n].material) ?? 0) + 1);
  const solid = (m: number) => materials[m]?.coverage.kind === "opaque" && !materials[m].interior;
  const ranked = [...area].sort((a, b) => b[1] - a[1] || a[0] - b[0]);
  const material = (ranked.find(([m]) => solid(m)) ?? ranked[0])?.[0];
  if (material === undefined) return null;
  const sum: Rgba = [0, 0, 0, 0];
  let n = 0;
  for (const k of hit)
    if (k >= 0 && tris[k].material === material) {
      for (let c = 0; c < 4; c++) sum[c] += tris[k].colour[c];
      n++;
    }
  return { material, colour: sum.map((s) => Math.round(s / n)) as Rgba };
}

/** The far box of `tier3`, or null when tier 3 shows nothing from above. */
export function farBox(tier3: MeshData, materials: readonly Material[]): MeshData | null {
  const tris = triangles(tier3);
  const lo: Vec3 = [Infinity, Infinity, Infinity],
    hi: Vec3 = [-Infinity, -Infinity, -Infinity];
  for (const t of tris)
    for (const p of t.p)
      for (let a = 0; a < 3; a++) [lo[a], hi[a]] = [Math.min(lo[a], p[a]), Math.max(hi[a], p[a])];
  // The footprint: tier 3's plan coverage, as a box of the bounds' proportions
  // and that area, about the coverage's centre.
  const top = rasterise(tris, { axis: 2, from: 1 }, lo, hi);
  let covered = 0,
    cx = 0,
    cy = 0;
  top.hit.forEach((n, i) => {
    if (n < 0) return;
    covered++;
    cx += lo[0] + (((i % GRID) + 0.5) / GRID) * top.u;
    cy += lo[1] + ((Math.floor(i / GRID) + 0.5) / GRID) * top.v;
  });
  if (!covered) return null;
  const shrink = Math.sqrt(covered / (GRID * GRID));
  const [hx, hy] = [(top.u / 2) * shrink, (top.v / 2) * shrink];
  const [mx, my] = [cx / covered, cy / covered];
  const [x0, x1, y0, y1, z0, z1] = [mx - hx, mx + hx, my - hy, my + hy, Math.max(lo[2], 0), hi[2]];
  // Each face: its outward normal, its corners counter-clockwise from
  // outside, the view its look comes from, and its UV axes.
  const faces: { normal: Vec3; corners: Vec3[]; view: View; uv: [number, number] }[] = [
    {
      normal: [0, 0, 1],
      view: { axis: 2, from: 1 },
      uv: [0, 1],
      corners: [
        [x0, y0, z1],
        [x1, y0, z1],
        [x1, y1, z1],
        [x0, y1, z1],
      ],
    },
    {
      normal: [0, -1, 0],
      view: { axis: 1, from: -1 },
      uv: [0, 2],
      corners: [
        [x0, y0, z0],
        [x1, y0, z0],
        [x1, y0, z1],
        [x0, y0, z1],
      ],
    },
    {
      normal: [0, 1, 0],
      view: { axis: 1, from: 1 },
      uv: [0, 2],
      corners: [
        [x1, y1, z0],
        [x0, y1, z0],
        [x0, y1, z1],
        [x1, y1, z1],
      ],
    },
    {
      normal: [1, 0, 0],
      view: { axis: 0, from: 1 },
      uv: [1, 2],
      corners: [
        [x1, y0, z0],
        [x1, y1, z0],
        [x1, y1, z1],
        [x1, y0, z1],
      ],
    },
    {
      normal: [-1, 0, 0],
      view: { axis: 0, from: -1 },
      uv: [1, 2],
      corners: [
        [x0, y1, z0],
        [x0, y0, z0],
        [x0, y0, z1],
        [x0, y1, z1],
      ],
    },
  ];
  const looks = faces.map((f) => dominant(tris, rasterise(tris, f.view, lo, hi).hit, materials));
  const fallback = looks.find((l) => l !== null);
  if (!fallback) return null;
  // Faces grouped by material, one draw range each.
  const order = faces
    .map((f, i) => ({ f, look: looks[i] ?? fallback }))
    .sort((a, b) => a.look.material - b.look.material);
  const mesh: MeshData = {
    positions: new Float32Array(20 * 3),
    normals: new Int16Array(20 * 4),
    tangents: new Int16Array(20 * 4),
    uvs: new Float32Array(20 * 2),
    colors: new Uint8Array(20 * 4),
    indices: new Uint16Array(FAR_BOX_TRIANGLES * 3),
    draws: [],
  };
  order.forEach(({ f, look }, k) => {
    // UVs in metres across the face, as the scripts' box projection lays
    // them; the tangent is +u, its sign the bitangent's against +v.
    const [ua, va] = f.uv;
    const t: Vec3 = [0, 0, 0];
    t[ua] = 1;
    const sign = vec3.cross([0, 0, 0], f.normal, t)[va] < 0 ? -1 : 1;
    for (let c = 0; c < 4; c++) {
      const o = k * 4 + c;
      mesh.positions.set(f.corners[c], o * 3);
      mesh.normals.set([f.normal[0] * 32767, f.normal[1] * 32767, f.normal[2] * 32767, 0], o * 4);
      mesh.tangents!.set([t[0] * 32767, t[1] * 32767, t[2] * 32767, sign * 32767], o * 4);
      mesh.uvs.set([f.corners[c][ua], f.corners[c][va]], o * 2);
      mesh.colors.set(look.colour, o * 4);
    }
    mesh.indices.set([k * 4, k * 4 + 1, k * 4 + 2, k * 4, k * 4 + 2, k * 4 + 3], k * 6);
    const last = mesh.draws[mesh.draws.length - 1];
    if (last && last.material === look.material) last.count += 6;
    else mesh.draws.push({ material: look.material, first: k * 6, count: 6 });
  });
  return mesh;
}
