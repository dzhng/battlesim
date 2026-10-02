import { forestInside, type ForestShape } from "../terrain/forestShapes";
// SceneryPlacement: where every tree and hedgerow shrub stands. Placement is
// code; the instanced unit is an appearance (`assets/catalog.json`, one
// `tree` or `hedgerow` bundle per kind), sized here only by its unscaled
// height and crown radius.
//
// Two populations, drawn alike but owned differently:
// - `forest`: the simulation's forests, drawn as trees: exactly one tree on
//   each of the simulation's trunks (the bodies movement, cover and
//   concealment meet, placed by the one forest rule), and no
//   other. Every crown stands under its forest's canopy over the simulation's
//   own ground and within the canopy's radius of its trunk, as the
//   simulation's foliage does: past the forest's edge too, so a tree there is
//   as wide as one inside. A crown is never drawn wider than its appearance,
//   which the asset validator holds inside that radius (`fit.canopy`): the
//   simulation's number has one owner. A trunk knocked down is gone from the
//   drawing where the side has seen the ground cleared (`treeCleared`).
//   Which species a trunk is drawn as is the biome's: a wood is stands, each
//   mostly one family's species, with the odd tree of no family among them
//   (`speciesAt`). Every species is one size, so the mix changes only the look.
// - `backdrop`: scenery past the map edge, where nothing is simulated —
//   hedgerows along the patchwork's plot edges with trees standing in them,
//   and copses. It keeps `backdrop.clear_m` off the map.
//
// Rewritten from reading ~/dev/game
// game-renderer/src/battle/terrainScenery.ts and terrain/sceneryDetail.ts:
// there, forests were filled by area and a variant was hashed from placement.
import { vec2, type Vec2 } from "math";
import { polygon2 } from "math/shapes";
import { mulberry32, random, type RandomGenerator } from "math/random";
import { drawnBy } from "../models/propAppearance";
import type { WorldExports, WorldLayout } from "../worldMesh";
import type { Biome, BiomeTrees } from "../terrain/biome";
import type { TerrainSurface } from "../terrain/terrainSurface";
import type { PlotTree } from "../terrain/plots";
import { groundHeight, type TerrainGrid } from "../terrain/terrainGrid";

/** Floats per placed tree. `kind` indexes `SceneryPlacement.kinds`; the
 *  scales apply to the appearance's own size; `r, g, b` multiply its albedo. */
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
  backdrop: Float32Array;
}

/** What placement reads of the static world, from the simulation's export. */
export interface ScenerySite {
  ground: TerrainGrid;
  /** Map box `[minX, minY, maxX, maxY]`. */
  map: readonly [number, number, number, number];
  forests: readonly ForestShape[];
  /** The simulation's trunk props: `x, y` pairs. */
  trunks: Float32Array;
  /** Original IDs in the native prop stream's ascending order. */
  trunkIds: Uint32Array;
  /** Every other prop's footprint circle, `x, y, radius` triples: no drawn trunk inside. */
  obstacles: Float32Array;
  plots: PlotTree;
  /** Height of the flat land past the map (the lowest ground). */
  backdropZ: number;
}

export function scenerySite(
  exports: WorldExports,
  layout: WorldLayout,
  terrain: TerrainSurface,
): ScenerySite {
  const ground = exports.terrain;
  const forests = terrain.site.forestShapes;
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const trunks: number[] = [];
  const trunkIds: number[] = [];
  const obstacles: number[] = [];
  const p = exports.props;
  for (let o = 0; o < p.length; o += layout.propStride) {
    if (drawnBy(layout, layout.propKinds[p[o + at.kind]], "forest")) {
      trunks.push(p[o + at.x], p[o + at.y]);
      trunkIds.push(p[o + at.idLo] + p[o + at.idHi] * 2 ** layout.limbBits);
    } else obstacles.push(p[o + at.x], p[o + at.y], Math.hypot(p[o + at.hx], p[o + at.hy]));
  }
  const low = ground.minHeight;
  return {
    ground,
    map: terrain.site.map,
    forests,
    trunks: Float32Array.from(trunks),
    trunkIds: Uint32Array.from(trunkIds),
    obstacles: Float32Array.from(obstacles),
    plots: terrain.plots,
    backdropZ: low,
  };
}

/** Trunks sink this far below the lowest ground around their foot. */
const SINK_M = 0.05;
/** Crown tops keep this far under the canopy. */
const CANOPY_MARGIN_M = 0.05;

const _placement_p = vec2.create();
const _placement_q = vec2.create();

/** A seeded stream per purpose, so one population's count never shifts another's. */
function stream(seed: number, salt: number): RandomGenerator {
  const state = mulberry32.create((seed ^ Math.imul(salt + 1, 0x9e3779b1)) >>> 0);
  return () => mulberry32.sample(state);
}

class Builder {
  readonly out: number[] = [];
  push(
    x: number,
    y: number,
    z: number,
    yaw: number,
    sxy: number,
    sz: number,
    kind: number,
    tint: readonly number[],
    rng: RandomGenerator,
    jitter: number,
  ) {
    const v = random.float(rng, -1, 1) * jitter;
    const w = random.float(rng, -1, 1) * jitter * 0.5;
    this.out.push(
      x,
      y,
      z,
      yaw,
      sxy,
      sz,
      kind,
      tint[0] * (1 + v) * (1 + w),
      tint[1] * (1 + v),
      tint[2] * (1 + v) * (1 - w),
    );
  }
}

export function placeScenery(
  site: ScenerySite,
  biome: Pick<Biome, "seed" | "trees">,
  sizes: ReadonlyMap<string, KindSize>,
): SceneryPlacement {
  const trees = biome.trees;
  const kinds = [
    ...new Set([...trees.species.map((s) => s.appearance), trees.hedgerows.appearance]),
  ];
  const missing = kinds.filter((k) => !sizes.has(k));
  if (missing.length)
    throw new Error(
      `scenery: no installed appearance named ${missing.map((k) => `"${k}"`).join(", ")}`,
    );
  const size = kinds.map((k) => sizes.get(k)!);
  const pick = speciesAt(trees, kinds, biome.seed);
  return {
    kinds,
    forest: placeForests(site, trees, size, pick, stream(biome.seed, 1)),
    backdrop: placeBackdrop(
      site,
      trees,
      size,
      kinds.indexOf(trees.hedgerows.appearance),
      pick,
      biome.seed,
    ),
  };
}

/** The species a tree at (x, y) is drawn as, standing `inside` metres
 *  within its forest's edge (`OPEN` where it has none to stand inside). */
type SpeciesPick = (
  rng: RandomGenerator,
  x: number,
  y: number,
  inside: number,
) => { kind: number; tint: readonly number[] };

/** No forest interior: a strip, or scenery past the map. */
const OPEN = -Infinity;

/** One of `rows` by weight. */
function weighted<T extends { weight: number }>(rng: RandomGenerator, rows: readonly T[]): T {
  let r = random.float(
    rng,
    0,
    rows.reduce((sum, s) => sum + s.weight, 0),
  );
  for (const s of rows) if ((r -= s.weight) < 0) return s;
  return rows[rows.length - 1];
}

/** The biome's mix of species over the ground. The land is cut into stands,
 *  the cells nearest seeded points `stands.size_m` apart, each one family's
 *  (drawn by the families' weights, so a species' weight is its share of all
 *  trees). A tree is first, by their weights, the odd one of no family, where
 *  that species may stand; otherwise of its stand's family (`stands.purity`
 *  of the time, else of any), a species of it by weight. */
function speciesAt(trees: BiomeTrees, kinds: readonly string[], seed: number): SpeciesPick {
  const species = trees.species.map((s) => ({ ...s, kind: kinds.indexOf(s.appearance) }));
  type Row = (typeof species)[number];
  const total = species.reduce((sum, s) => sum + s.weight, 0);
  const odd = species.filter((s) => s.family === undefined);
  const families = new Map<string, { weight: number; species: Row[] }>();
  for (const s of species) {
    if (s.family === undefined) continue;
    const family = families.get(s.family) ?? { weight: 0, species: [] };
    family.weight += s.weight;
    family.species.push(s);
    families.set(s.family, family);
  }
  const drawn = [...families.values()];
  const { size_m, purity } = trees.stands;
  const points = new Map<number, { x: number; y: number; family: (typeof drawn)[number] }>();
  /** The seeded point of stand cell (i, j), and its family. */
  const point = (i: number, j: number) => {
    const key = (i + 0x8000) * 0x10000 + (j + 0x8000);
    let p = points.get(key);
    if (!p) {
      const rng = stream(seed ^ STAND_SALT, key);
      p = {
        x: (i + random.float(rng, 0, 1)) * size_m,
        y: (j + random.float(rng, 0, 1)) * size_m,
        family: weighted(rng, drawn),
      };
      points.set(key, p);
    }
    return p;
  };
  const stand = (x: number, y: number) => {
    const [ci, cj] = [Math.floor(x / size_m), Math.floor(y / size_m)];
    let nearest = point(ci, cj);
    let best = Infinity;
    for (let j = cj - 1; j <= cj + 1; j++)
      for (let i = ci - 1; i <= ci + 1; i++) {
        const p = point(i, j);
        const d = (p.x - x) ** 2 + (p.y - y) ** 2;
        if (d < best) [nearest, best] = [p, d];
      }
    return nearest.family;
  };
  return (rng, x, y, inside) => {
    let r = random.float(rng, 0, total);
    for (const s of odd)
      if ((r -= s.weight) < 0) {
        if (inside >= (s.interior_m ?? OPEN)) return s;
        break;
      }
    const family = random.bool(rng, purity) ? stand(x, y) : weighted(rng, drawn);
    return weighted(rng, family.species);
  };
}

/** Parts the stands' seeded points from every other stream of the seed. */
const STAND_SALT = 0x57a2d5;

function placeForests(
  site: ScenerySite,
  trees: BiomeTrees,
  size: readonly KindSize[],
  pick: SpeciesPick,
  rng: RandomGenerator,
): Float32Array {
  const out = new Builder();
  const rules = trees.forest;
  const ground = (x: number, y: number) => groundHeight(site.ground, x, y);
  let trunk = 0;
  for (const f of site.forests) {
    /** Fit one tree at (x, y): its crown a share of its appearance's
     *  width, its top under the canopy over the lowest ground its crown
     *  covers. */
    const plant = (x: number, y: number) => {
      const s = pick(rng, x, y, f.kind === "stroke" ? OPEN : forestInside(f, x, y));
      const kind = size[s.kind];
      let sz = (f.canopy * random.float(rng, rules.top[0], rules.top[1])) / kind.height;
      const sxy = random.float(rng, rules.girth[0], rules.girth[1]);
      const radius = kind.radius * sxy;
      let foot = ground(x, y);
      for (let k = 0; k < 4; k++) {
        const a = (k * Math.PI) / 2;
        foot = Math.min(foot, ground(x + 0.5 * Math.cos(a), y + 0.5 * Math.sin(a)));
      }
      const z = foot - SINK_M;
      let floor = foot;
      for (let ring = 1; ring <= 4; ring++)
        for (let k = 0; k < 16; k++) {
          const a = (k * Math.PI) / 8;
          const r = (radius * ring) / 4;
          floor = Math.min(floor, ground(x + r * Math.cos(a), y + r * Math.sin(a)));
        }
      sz = Math.min(sz, (floor + f.canopy - CANOPY_MARGIN_M - z) / kind.height);
      out.push(
        x,
        y,
        z,
        random.float(rng, 0, Math.PI * 2),
        sxy,
        sz,
        s.kind,
        s.tint,
        rng,
        trees.colour_jitter,
      );
    };
    // Native source ranges and prop rows are ordered; each generated trunk is
    // assigned once, even where authored shapes overlap.
    const [first, end] = f.trunkRange;
    while (trunk < site.trunkIds.length && site.trunkIds[trunk] < first) trunk++;
    while (trunk < site.trunkIds.length && site.trunkIds[trunk] < end) {
      plant(site.trunks[trunk * 2], site.trunks[trunk * 2 + 1]);
      trunk++;
    }
  }
  return Float32Array.from(out.out);
}

/** Distance from (x, y) to the box, 0 inside. */
function outsideBox(box: readonly number[], x: number, y: number): number {
  return Math.hypot(Math.max(box[0] - x, x - box[2], 0), Math.max(box[1] - y, y - box[3], 0));
}

function placeBackdrop(
  site: ScenerySite,
  trees: BiomeTrees,
  size: readonly KindSize[],
  hedge: number,
  pick: SpeciesPick,
  seed: number,
): Float32Array {
  const out = new Builder();
  const { clear_m, reach_m } = trees.backdrop;
  const rules = trees.hedgerows;
  const z = site.backdropZ - SINK_M;
  const fits = (x: number, y: number, radius: number) => {
    const d = outsideBox(site.map, x, y);
    return d - radius >= clear_m && d <= reach_m;
  };
  const tree = (rng: RandomGenerator, x: number, y: number) => {
    const s = pick(rng, x, y, OPEN);
    const sz = random.float(rng, rules.tree_scale[0], rules.tree_scale[1]);
    const sxy = sz * random.float(rng, trees.forest.girth[0], trees.forest.girth[1]);
    if (!fits(x, y, size[s.kind].radius * sxy)) return;
    out.push(
      x,
      y,
      z,
      random.float(rng, 0, Math.PI * 2),
      sxy,
      sz,
      s.kind,
      s.tint,
      rng,
      trees.colour_jitter,
    );
  };
  const plots = site.plots.plots;
  for (let k = 0; k < plots.length; k++) {
    const poly = plots[k].outline;
    const n = poly.length / 2;
    // Skip plots wholly inside the clear band or wholly past reach.
    let nearest = Infinity,
      farthest = 0;
    for (let i = 0; i < poly.length; i += 2) {
      const d = outsideBox(site.map, poly[i], poly[i + 1]);
      nearest = Math.min(nearest, d);
      farthest = Math.max(farthest, d);
    }
    if (farthest < clear_m || nearest > reach_m) continue;
    const rng = stream(seed, 1000 + k);
    // Hedgerows along the plot's edges, a little inside it.
    for (let i = 0; i < n; i++) {
      const j = (i + 1) % n;
      const [ax, ay, bx, by] = [poly[i * 2], poly[i * 2 + 1], poly[j * 2], poly[j * 2 + 1]];
      const length = Math.hypot(bx - ax, by - ay);
      const hedged = random.bool(rng, rules.chance);
      if (!hedged || length < rules.min_edge_m) continue;
      const [dx, dy] = [(bx - ax) / length, (by - ay) / length];
      const yaw = Math.atan2(dy, dx);
      const at = (t: number): [number, number] => [
        ax + dx * t - dy * rules.inset_m,
        ay + dy * t + dx * rules.inset_m,
      ];
      const shrub = size[hedge];
      for (let t = rules.spacing_m / 2; t < length - rules.spacing_m / 4; t += rules.spacing_m) {
        const [x, y] = at(t + random.float(rng, -0.15, 0.15) * rules.spacing_m);
        const sxy = random.float(rng, 0.85, 1.1);
        if (!fits(x, y, shrub.radius * sxy)) continue;
        out.push(
          x,
          y,
          z,
          yaw + random.float(rng, -0.12, 0.12),
          sxy,
          random.float(rng, 0.9, 1.3),
          hedge,
          rules.tint,
          rng,
          trees.colour_jitter,
        );
      }
      for (let t = random.float(rng, 0, rules.tree_gap_m[1]); t < length; ) {
        tree(rng, ...at(t));
        t += random.float(rng, rules.tree_gap_m[0], rules.tree_gap_m[1]);
      }
    }
    // A copse: a disc of trees about the plot's centre, inside the plot.
    if (random.bool(rng, trees.copses.chance)) {
      const centre = polygon2.centroid(_placement_p, poly, n);
      const [cx, cy] = [centre[0], centre[1]];
      const radius = random.float(rng, trees.copses.radius_m[0], trees.copses.radius_m[1]);
      const step = trees.copses.spacing_m;
      for (let y = cy - radius; y <= cy + radius; y += step)
        for (let x = cx - radius; x <= cx + radius; x += step) {
          const px = x + random.float(rng, -0.35, 0.35) * step;
          const py = y + random.float(rng, -0.35, 0.35) * step;
          if (Math.hypot(px - cx, py - cy) > radius) continue;
          if (!polygon2.containsPoint(poly, n, vec2.set(_placement_q, px, py) as Vec2)) continue;
          tree(rng, px, py);
        }
    }
  }
  return Float32Array.from(out.out);
}
