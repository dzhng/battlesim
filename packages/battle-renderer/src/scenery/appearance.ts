// A tree, hedgerow or dressing appearance as the scenery draws it: the static bundle's
// summer state, from the one loader (`AppearanceLibrary`), turned into the
// renderer's vertex format per tier, and what the presentation's scenery
// walk needs to know of its mesh (`sceneryFacts`): its size, and a tree's
// trunk and crown, which a felled tree falls by. The bundle is the unit;
// placement and the biome's tint only instance it.
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { MeshData, StaticBundle } from "@packages/scene-assets/src/schema";
import { VERTEX_FLOATS, type Mesh } from "../mesh";

/** The state scenery draws (winter will be the next biome's). */
export const SCENERY_STATE = "summer";
/** The material whose triangles are foliage: shaded with leaf clumps, fogged
 *  as a volume. Every other material (bark) is a plain surface. */
export const FOLIAGE_MATERIAL = "leaves";

/** The installed scenery appearances `names` needs, by name; throws naming
 *  any that is missing or is not of one of the scenery `kinds`. */
export function sceneryAppearances(
  installed: InstalledAppearances,
  names: readonly string[],
  kinds: readonly string[],
): Map<string, StaticBundle> {
  const out = new Map<string, StaticBundle>();
  for (const name of names) {
    const entry = installed.appearances.get(name);
    if (!entry) throw new Error(`scenery: appearance "${name}" is not in the catalog (asset bake)`);
    if (
      entry.bundle.kind !== "static" ||
      entry.unit !== "scenery" ||
      !kinds.includes(entry.scenery ?? "") ||
      !entry.bundle.states.some((s) => s.name === SCENERY_STATE)
    )
      throw new Error(
        `scenery: appearance "${name}" is not a ${kinds.join(" or ")} with a summer state`,
      );
    out.set(name, entry.bundle);
  }
  return out;
}

function tiers(bundle: StaticBundle): MeshData[] {
  return bundle.states.find((s) => s.name === SCENERY_STATE)!.tiers;
}

/** Unscaled height and crown reach from the trunk's axis, over the finest tier. */
export function kindSize(bundle: StaticBundle): { height: number; radius: number } {
  const p = tiers(bundle)[0].positions;
  let height = 0,
    radius = 0;
  for (let i = 0; i < p.length; i += 3) {
    height = Math.max(height, p[i + 2]);
    radius = Math.max(radius, Math.hypot(p[i], p[i + 1]));
  }
  return { height, radius };
}

/** One tier as triangles in the renderer's vertex format: linear albedo
 *  (material colour times vertex colour) in rgb, 1 in alpha for foliage. */
export function tierMesh(bundle: StaticBundle, tier: number): Mesh {
  const mesh = tiers(bundle)[tier];
  const out = new Float32Array(mesh.indices.length * VERTEX_FLOATS);
  let o = 0;
  for (const draw of mesh.draws) {
    const material = bundle.materials[draw.material];
    const foliage = material.name === FOLIAGE_MATERIAL ? 1 : 0;
    for (let i = draw.first; i < draw.first + draw.count; i++) {
      const v = mesh.indices[i];
      out[o] = mesh.positions[v * 3];
      out[o + 1] = mesh.positions[v * 3 + 1];
      out[o + 2] = mesh.positions[v * 3 + 2];
      for (let c = 0; c < 3; c++) out[o + 3 + c] = mesh.normals[v * 4 + c] / 32767;
      for (let c = 0; c < 3; c++)
        out[o + 6 + c] = material.base_color[c] * (mesh.colors[v * 4 + c] / 255);
      out[o + 9] = foliage;
      o += VERTEX_FLOATS;
    }
  }
  return out;
}

/** What the scenery walk places and fells an appearance by, four numbers:
 *  its height and reach (`kindSize`), and for a tree its trunk's radius at
 *  the stump's top `hinge` metres up and where its leaves begin (zeros for
 *  a shrub or dressing, which no fall cuts). */
export function sceneryFacts(bundle: StaticBundle, tree: boolean, hinge: number): number[] {
  const { height, radius } = kindSize(bundle);
  if (!tree) return [height, radius, 0, 0];
  const tier = tierMesh(bundle, 0);
  return [height, radius, trunkAt(tier, hinge).radius, crownBase(tier)];
}

/** A landing tree's foot kicks off its stump this many trunk radii along
 *  the fall: clear of the stump, which stands bare behind it. */
export const KICK_RADII = 2;

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
