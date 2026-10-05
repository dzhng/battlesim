// Felled trees as the scenery draws them: a forest tree the side saw go
// down (`BattleFrame.setFelled`) leaves the standing forest and is drawn
// here instead, tipping over from the top of its stump the way it fell, its
// foot kicking off the stump as it lands, then lying on the ground beside
// it. The simulation gives the fallen tree no body, cover or foliage, so
// lying it is drawn as brush: its crown pressed under
// `felled.rest_height_m` and narrowed to `felled.rest_spread`, so it never
// reads as cover or hides a soldier the simulation shows. The trunk keeps
// its girth.
//
// The tip-over is one rotation about the horizontal axis across the fall,
// through the hinge (the stump's top), then, as it lands, the kick off the
// stump and the press of what lies past the trunk; `felledPoint` is the CPU
// mirror of the vertex stage's math (`frame/sceneryLayer.ts`), so tests
// hold the drawn shape to the rules.
import { VERTEX_FLOATS, type Mesh } from "../mesh";
import type { FelledRules } from "../terrain/biome";
import { HEART, INSTANCE_FLOATS, leafSeed } from "./lod";
import { TREE_FIELD, TREE_FLOATS, type KindSize } from "./placement";

/** A tree the side saw fall: its trunk's prop id, the horizontal unit
 *  direction it fell toward, and the presentation second it began to. */
export interface FelledTree {
  prop: number;
  toward: readonly [number, number];
  fellAt: number;
}

/** What a tree kind lends its fall, unscaled: its trunk's radius at the
 *  hinge, and the height its crown's leaves begin at. */
export interface FelledKind {
  trunk: number;
  crown: number;
}

/** A felled tree's pose: the way it falls, how far it has tipped, the
 *  hinge, its trunk's radius, the press up-and-down and across of what lies
 *  past its trunk, how far it has landed (0 to 1), and the height its crown
 *  begins at (above that, its bark is pressed as its leaves are). */
export interface FellPose {
  toward: readonly [number, number];
  angle: number;
  hinge: number;
  trunk: number;
  squash: number;
  spread: number;
  landed: number;
  crown: number;
}

/** Floats per falling or lying tree: the standing tree's instance (pose,
 *  shape, tint), then its `FellPose`: (toward x, y, angle, hinge), (squash,
 *  spread, trunk, landed), (crown, 0, 0, 0). */
export const FELLED_FLOATS = INSTANCE_FLOATS + 12;

const QUARTER = Math.PI / 2;
/** A landing tree's foot kicks off its stump this many trunk radii along
 *  the fall: clear of the stump, which stands bare behind it. */
export const KICK_RADII = 2;

/** The angle a tree lies at, radians from upright: past level by as much as
 *  lays a trunk `length` metres long, its foot resting on the ground a
 *  `trunk` radius up, with its top on the ground. */
export function restAngle(trunk: number, length: number): number {
  return QUARTER + Math.asin(Math.min(1, trunk / Math.max(length, 1e-3)));
}

/** How far the tree has tipped from upright, radians, `age` seconds after
 *  it began to fall: from rest, gathering speed as a toppling body does,
 *  striking the ground (`rest`) at `fall_s`; then a rock back up and down
 *  again over `settle_s`; then lying. Upright before it began. */
export function fallAngle(rules: FelledRules, age: number, rest: number): number {
  if (age <= 0) return 0;
  if (age < rules.fall_s) return rest * (age / rules.fall_s) ** 2;
  const u = rules.settle_s > 0 ? (age - rules.fall_s) / rules.settle_s : 1;
  if (u >= 1) return rest;
  return rest - ((rules.settle_deg * Math.PI) / 180) * Math.sin(Math.PI * u) * (1 - u);
}

/** Whether a tree that began to fall `age` seconds ago still moves. */
function stillFalling(rules: FelledRules, age: number): boolean {
  return age < rules.fall_s + rules.settle_s;
}

/** How far a tree tipped `angle` of its `rest` has landed: none in its
 *  first half, as its crown nears the ground, then easing to all of it on
 *  the ground. */
export function lyingShare(angle: number, rest: number): number {
  const u = Math.min(1, Math.max(0, (angle / rest - 0.5) / 0.5));
  return u * u * (3 - 2 * u);
}

/** The press that keeps a lying crown of `reach` metres (its farthest leaf
 *  from the trunk's axis) under `rest_height_m` where its axis lies `axis`
 *  metres up. */
function restSquash(rules: FelledRules, reach: number, axis: number): number {
  return Math.min(1, Math.max(0.02, (rules.rest_height_m - axis) / Math.max(reach, 1e-3)));
}

/** `d` metres from the trunk's axis, with what lies past `radius` scaled
 *  by `k`. */
function pastTrunk(d: number, radius: number, k: number): number {
  const m = Math.abs(d);
  return Math.sign(d) * (Math.min(m, radius) + Math.max(m - radius, 0) * k);
}

/** The CPU mirror of the vertex stage: a point of the appearance, already
 *  scaled and turned by the tree's yaw (`p`, relative to its foot; leaves
 *  when `foliage`), in pose `f`: tipped about the hinge, kicked off the
 *  stump and lowered onto the ground as it lands; what stood below the cut
 *  closes onto it (the stump stands there); its leaves, and its bark above
 *  the crown's base, are pressed whole, its trunk below keeps its girth. */
export function felledPoint(
  p: readonly [number, number, number],
  foliage: boolean,
  f: FellPose,
): [number, number, number] {
  const [tx, ty] = f.toward;
  const core = foliage || p[2] > f.crown ? 0 : f.trunk;
  const a = p[0] * tx + p[1] * ty;
  const [sx, sy] = [p[0] - tx * a, p[1] - ty * a];
  const side = Math.hypot(sx, sy);
  const k = side > 0 ? pastTrunk(side, core, f.spread) / side : 0;
  // Below the cut the stump stands in its place.
  const z = Math.max(p[2] - f.hinge, 0);
  const c = Math.cos(f.angle),
    s = Math.sin(f.angle);
  const along = a * c + z * s + KICK_RADII * f.trunk * f.landed;
  const up = z * c - pastTrunk(a, core, f.squash) * s - (f.hinge - f.trunk) * f.landed;
  return [tx * along + sx * k, ty * along + sy * k, f.hinge + up];
}

/** Where a tree kind's leaves begin, unscaled: its lowest leaf's height
 *  in `tier` (`tierMesh`), or the top of a tree with none. */
export function crownBase(tier: Mesh): number {
  let low = Infinity,
    top = 0;
  for (let o = 0; o < tier.length; o += VERTEX_FLOATS) {
    top = Math.max(top, tier[o + 2]);
    if (tier[o + 9] > 0.5) low = Math.min(low, tier[o + 2]);
  }
  return Math.min(low, top);
}

/** `records` (`stride` floats each) reordered so equal `keys` sit
 *  together in ascending order, and the runs they make: `[key, first,
 *  count]` triples, one per key, `first` in records. */
export function groupByKey(
  records: Float32Array,
  stride: number,
  keys: readonly number[],
): { records: Float32Array; runs: number[] } {
  const order = keys.map((_, i) => i).sort((a, b) => keys[a] - keys[b]);
  const out = new Float32Array(records.length);
  const runs: number[] = [];
  order.forEach((i, k) => {
    out.set(records.subarray(i * stride, (i + 1) * stride), k * stride);
    if (runs.length && runs[runs.length - 3] === keys[i]) runs[runs.length - 1]++;
    else runs.push(keys[i], k, 1);
  });
  return { records: out, runs };
}

/** Where each of `placed`'s trees is, by trunk id. */
export function treeIndex(ids: Uint32Array): Map<number, number> {
  const out = new Map<number, number>();
  ids.forEach((id, i) => out.set(id, i * TREE_FLOATS));
  return out;
}

/** A felled tree's pose `age` seconds after it began to fall, at its
 *  standing height `height`, crown reach `reach`, trunk radius `trunk` and
 *  crown base `crown` (all scaled). */
export function fellPose(
  rules: FelledRules,
  toward: readonly [number, number],
  age: number,
  height: number,
  reach: number,
  trunk: number,
  crown: number,
): FellPose {
  const hinge = rules.stump_height_m;
  const length = height - hinge;
  const rest = restAngle(trunk, length);
  const angle = fallAngle(rules, age, rest);
  const landed = lyingShare(angle, rest);
  // The axis lies a trunk's radius up at the foot and on the ground at the
  // top: where the crown begins it lies this high.
  const axis = trunk * (1 - Math.min(1, Math.max(0, (crown - hinge) / length)));
  return {
    toward,
    angle,
    hinge,
    trunk,
    squash: 1 + (restSquash(rules, reach, axis) - 1) * landed,
    spread: 1 + (rules.rest_spread - 1) * landed,
    landed,
    crown,
  };
}

/** The falling and lying trees at presentation second `clock`: each felled
 *  tree found in `placed` (`TREE_FLOATS` per tree, `index` by trunk id) as
 *  a `FELLED_FLOATS` record, its kind and its standing height. Whether any
 *  still moves says whether the next frame must pack them again. */
export function felledRecords(
  felled: readonly FelledTree[],
  placed: Float32Array,
  index: ReadonlyMap<number, number>,
  sizes: readonly KindSize[],
  kinds: readonly FelledKind[],
  rules: FelledRules,
  clock: number,
): { records: Float32Array; kinds: Uint16Array; heights: Float32Array; moving: boolean } {
  const found = felled.filter((f) => index.has(f.prop));
  const records = new Float32Array(found.length * FELLED_FLOATS);
  const kindOf = new Uint16Array(found.length);
  const heights = new Float32Array(found.length);
  let moving = false;
  found.forEach((f, i) => {
    const o = index.get(f.prop)!;
    const kind = placed[o + TREE_FIELD.kind];
    const [x, y] = [placed[o + TREE_FIELD.x], placed[o + TREE_FIELD.y]];
    const [sxy, sz] = [placed[o + TREE_FIELD.scaleXY], placed[o + TREE_FIELD.scaleZ]];
    const h = sizes[kind].height * sz;
    const age = clock - f.fellAt;
    moving ||= stillFalling(rules, age);
    const pose = fellPose(
      rules,
      f.toward,
      age,
      h,
      sizes[kind].radius * sxy,
      kinds[kind].trunk * sxy,
      kinds[kind].crown * sz,
    );
    kindOf[i] = kind;
    heights[i] = h;
    records.set(
      [
        x,
        y,
        placed[o + TREE_FIELD.z],
        placed[o + TREE_FIELD.yaw],
        sxy,
        sxy,
        sz,
        HEART * h,
        placed[o + TREE_FIELD.r],
        placed[o + TREE_FIELD.g],
        placed[o + TREE_FIELD.b],
        leafSeed(x, y),
        f.toward[0],
        f.toward[1],
        pose.angle,
        pose.hinge,
        pose.squash,
        pose.spread,
        pose.trunk,
        pose.landed,
        pose.crown,
        0,
        0,
        0,
      ],
      i * FELLED_FLOATS,
    );
  });
  return { records, kinds: kindOf, heights, moving };
}

/** The stumps of `felled`'s trees found in `placed`, as ordinary placed
 *  instances (`INSTANCE_FLOATS` each) of each kind's stump mesh, and their
 *  kinds: a stump stands as the tree stood, girth and all. */
export function stumpRecords(
  felled: readonly FelledTree[],
  placed: Float32Array,
  index: ReadonlyMap<number, number>,
  hinge: number,
): { records: Float32Array; kinds: Uint16Array } {
  const found = felled.filter((f) => index.has(f.prop));
  const records = new Float32Array(found.length * INSTANCE_FLOATS);
  const kinds = new Uint16Array(found.length);
  found.forEach((f, i) => {
    const o = index.get(f.prop)!;
    const [x, y] = [placed[o + TREE_FIELD.x], placed[o + TREE_FIELD.y]];
    const sxy = placed[o + TREE_FIELD.scaleXY];
    kinds[i] = placed[o + TREE_FIELD.kind];
    records.set(
      [
        x,
        y,
        placed[o + TREE_FIELD.z],
        placed[o + TREE_FIELD.yaw],
        sxy,
        sxy,
        1,
        0.5 * hinge,
        placed[o + TREE_FIELD.r],
        placed[o + TREE_FIELD.g],
        placed[o + TREE_FIELD.b],
        0,
      ],
      i * INSTANCE_FLOATS,
    );
  });
  return { records, kinds };
}

/** Facets round a stump. */
const STUMP_SIDES = 10;
/** A trunk's foot ring lies this close to its base. */
const FOOT_M = 0.05;

/** A kind's trunk at `height` above its foot, unscaled: its radius and its
 *  bark's mean colour. `tier` is the appearance's finest tier in the
 *  renderer's vertex format (`tierMesh`), whose trunk is rings of bark (the
 *  non-foliage triangles): the foot ring's and the next ring's mean radii,
 *  interpolated at `height`. */
export function trunkAt(tier: Mesh, height: number): { radius: number; bark: number[] } {
  // The lowest bark ring above the foot.
  let next = Infinity;
  for (let o = 0; o < tier.length; o += VERTEX_FLOATS)
    if (tier[o + 9] < 0.5 && tier[o + 2] > FOOT_M) next = Math.min(next, tier[o + 2]);
  const rings = [
    { radius: 0, n: 0 },
    { radius: 0, n: 0 },
  ];
  const bark = [0, 0, 0];
  for (let o = 0; o < tier.length; o += VERTEX_FLOATS) {
    const z = tier[o + 2];
    if (tier[o + 9] > 0.5) continue;
    const ring = z <= FOOT_M ? rings[0] : Math.abs(z - next) <= FOOT_M ? rings[1] : null;
    if (!ring) continue;
    ring.radius += Math.hypot(tier[o], tier[o + 1]);
    ring.n++;
    for (let c = 0; c < 3; c++) bark[c] += tier[o + 6 + c];
  }
  const [foot, ring] = rings;
  if (foot.n === 0 || ring.n === 0)
    throw new Error("scenery: a tree appearance has no bark ring at its foot");
  for (let c = 0; c < 3; c++) bark[c] /= foot.n + ring.n;
  const [r0, r1] = [foot.radius / foot.n, ring.radius / ring.n];
  return { radius: r0 + (r1 - r0) * Math.min(1, height / next), bark };
}

/** A kind's stump: a short prism as wide as its trunk at `height`
 *  (`trunkAt`), in its bark's colour, its top the cut face. */
export function stumpMesh(tier: Mesh, height: number, cut: readonly number[]): Mesh {
  const { radius, bark } = trunkAt(tier, height);
  const out: number[] = [];
  const vertex = (
    x: number,
    y: number,
    z: number,
    nx: number,
    ny: number,
    nz: number,
    colour: readonly number[],
  ) => out.push(x, y, z, nx, ny, nz, colour[0], colour[1], colour[2], 0);
  for (let k = 0; k < STUMP_SIDES; k++) {
    const a0 = (k / STUMP_SIDES) * Math.PI * 2;
    const a1 = ((k + 1) / STUMP_SIDES) * Math.PI * 2;
    const am = (a0 + a1) / 2;
    const [c0, s0, c1, s1] = [Math.cos(a0), Math.sin(a0), Math.cos(a1), Math.sin(a1)];
    const [nx, ny] = [Math.cos(am), Math.sin(am)];
    const [x0, y0, x1, y1] = [radius * c0, radius * s0, radius * c1, radius * s1];
    // The side, counter-clockwise seen from outside.
    vertex(x0, y0, 0, nx, ny, 0, bark);
    vertex(x1, y1, 0, nx, ny, 0, bark);
    vertex(x1, y1, height, nx, ny, 0, bark);
    vertex(x0, y0, 0, nx, ny, 0, bark);
    vertex(x1, y1, height, nx, ny, 0, bark);
    vertex(x0, y0, height, nx, ny, 0, bark);
    // The cut face, seen from above.
    vertex(0, 0, height, 0, 0, 1, cut);
    vertex(x0, y0, height, 0, 0, 1, cut);
    vertex(x1, y1, height, 0, 0, 1, cut);
  }
  return Float32Array.from(out);
}
