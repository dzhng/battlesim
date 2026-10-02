// SceneryPlacement: where every tree and hedgerow shrub stands. Placement is
// code; the instanced unit is an appearance (`assets/catalog.json`, one
// `tree` or `hedgerow` bundle per kind), sized here only by its unscaled
// height and crown radius.
//
// Four populations, drawn alike but owned differently:
// - `forest`: the simulation's tree bodies, drawn as trees: exactly one tree
//   on each trunk (the bodies movement, cover and concealment meet), and no
//   other. A forest's trunks are placed by the one forest rule, and a trunk
//   outside every forest (a street's tree) is a tree of its own
//   (`trees.lone`): as tall as its body, of a species the biome lets stand
//   alone. Every forest crown stands under its forest's canopy over the
//   simulation's own ground and within the canopy's radius of its trunk, as
//   the simulation's foliage does: past the forest's edge too, so a tree
//   there is as wide as one inside. A crown is never drawn wider than its appearance,
//   which the asset validator holds inside that radius (`fit.canopy`): the
//   simulation's number has one owner. A trunk knocked down is gone from the
//   drawing where the side has seen the ground cleared (the scenery layer's
//   `setCleared`).
//   Which species a trunk is drawn as is the biome's: a wood is stands, each
//   mostly one family's species, with the odd tree of no family among them
//   (`speciesAt`). Every species is one size, so the mix changes only the look.
// - `understorey`: the shrubs under a tree line (a strip of forest), in rows
//   along it (`trees.understorey`). The simulation lets no sight across a
//   strip, and a row of bare boles says otherwise, so the strip is drawn
//   with a hedge under its crowns. Each shrub keeps its whole reach on the
//   strip's own ground, so nothing is drawn wider than the forest that
//   blocks sight, and stands off paving, water and other bodies. A wood has
//   none: its floor's dressing stays under a man's waist.
// - `backdrop`: scenery past the map edge, where nothing is simulated —
//   hedgerows along the patchwork's plot edges with trees standing in them,
//   and copses. It keeps `backdrop.clear_m` off the map.
// - `dressing`: what grows and lies on the simulation's forest floors with no
//   body of its own (ferns, bushes, saplings, small rocks, fallen branches:
//   `forest_floor.dressing`). Scattered over forest ground alone, inside
//   the edge the floor's verge wanders about, clear of every trunk and body
//   and of paving and water; each kind gathers in drifts. It is laid a cell
//   of ground at a time, seeded by the cell, and only when asked for
//   (`DressingField`): a map's forests hold far too many pieces to lay
//   whole. Presentation only: no piece is drawn larger than its appearance,
//   which the asset validator holds under a man's waist (`fit.dressing`), so
//   none hides what the forest does not.
//
// Rewritten from reading ~/dev/game
// game-renderer/src/battle/terrainScenery.ts and terrain/sceneryDetail.ts:
// there, forests were filled by area and a variant was hashed from placement.
import { vec2, type Vec2 } from "math";
import { polygon2 } from "math/shapes";
import { mulberry32, random, type RandomGenerator } from "math/random";
import { simplex2d } from "math/noise";
import { drawnBy } from "../models/propAppearance";
import type { WorldExports, WorldLayout } from "../worldMesh";
import type { Biome, BiomeTrees, ForestDressing } from "../terrain/biome";
import { FOREST_TRIANGLE_FLOATS, forestInside, type ForestShape } from "../terrain/forestShapes";
import { STROKE_FLOATS } from "../terrain/strokes";
import {
  buildSurfaceField,
  forestDistance,
  pavedDistance,
  SURFACE_FOOTPRINT_M,
  waterDistance,
} from "../terrain/surfaceField";
import type { TerrainSurface } from "../terrain/terrainSurface";
import type { PlotTree } from "../terrain/plots";
import { groundHeight, type TerrainGrid } from "../terrain/terrainGrid";

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
  understorey: Float32Array;
  backdrop: Float32Array;
  dressing: DressingField;
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
  /** Each trunk body's height, metres. */
  trunkHeights: Float32Array;
  /** Every other prop's footprint circle, `x, y, radius` triples: no dressing inside. */
  obstacles: Float32Array;
  /** How far `(x, y)` lies inside the forests' ground, metres (negative
   *  outside), exact within `forest_floor.dressing.edge_m` of their edges. */
  forestInside(x: number, y: number): number;
  /** How far `(x, y)` lies inside paving or water, metres (negative
   *  outside), exact within `forest_floor.dressing.clear_m` of their edges. */
  wetOrPaved(x: number, y: number): number;
  /** Whether the simulation's foliage (what sight fades across) covers the
   *  fog cell at `(x, y)` or one beside it. */
  foliageNear(x: number, y: number): boolean;
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
  const trunkHeights: number[] = [];
  const obstacles: number[] = [];
  const p = exports.props;
  for (let o = 0; o < p.length; o += layout.propStride) {
    if (drawnBy(layout, layout.propKinds[p[o + at.kind]], "forest")) {
      trunks.push(p[o + at.x], p[o + at.y]);
      trunkIds.push(p[o + at.idLo] + p[o + at.idHi] * 2 ** layout.limbBits);
      trunkHeights.push(2 * p[o + at.hz]);
    } else obstacles.push(p[o + at.x], p[o + at.y], Math.hypot(p[o + at.hx], p[o + at.hy]));
  }
  const low = ground.minHeight;
  const { clear_m, edge_m } = terrain.biome.forest_floor.dressing;
  const reach = { paved: clear_m, forest: edge_m, water: clear_m };
  const field = buildSurfaceField(terrain.site, () => reach);
  // The sparse foliage export: the grid, then a record per foliage cell.
  const [columns, , fogCellM] = exports.foliage;
  const foliage = new Set<number>();
  for (let o = FOLIAGE_HEADER; o < exports.foliage.length; o += FOLIAGE_FLOATS)
    foliage.add(exports.foliage[o + 1] * columns + exports.foliage[o]);
  return {
    ground,
    map: terrain.site.map,
    forests,
    trunks: Float32Array.from(trunks),
    trunkIds: Uint32Array.from(trunkIds),
    trunkHeights: Float32Array.from(trunkHeights),
    obstacles: Float32Array.from(obstacles),
    forestInside: (x, y) => forestDistance(field, x, y, SURFACE_FOOTPRINT_M),
    wetOrPaved: (x, y) =>
      Math.max(
        pavedDistance(field, x, y, SURFACE_FOOTPRINT_M),
        waterDistance(field, x, y, SURFACE_FOOTPRINT_M),
      ),
    foliageNear: (x, y) => {
      const [i, j] = [Math.floor(x / fogCellM), Math.floor(y / fogCellM)];
      for (let dj = -1; dj <= 1; dj++)
        for (let di = -1; di <= 1; di++)
          if (i + di >= 0 && i + di < columns && foliage.has((j + dj) * columns + i + di))
            return true;
      return false;
    },
    plots: terrain.plots,
    backdropZ: low,
  };
}

/** The simulation's sparse foliage export (`WorldView.foliage`): the grid's
 *  `[columns, rows, cell]`, then `[column, row, canopy, depth]` per cell. */
const FOLIAGE_HEADER = 3;
const FOLIAGE_FLOATS = 4;

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

/** Every appearance `biome` places: its trees, its shrubs and its dressing. */
export function sceneryKinds(biome: Pick<Biome, "trees" | "forest_floor">): string[] {
  return [
    ...new Set([
      ...biome.trees.species.map((s) => s.appearance),
      biome.trees.hedgerows.appearance,
      biome.trees.understorey.appearance,
      ...biome.forest_floor.dressing.kinds.map((k) => k.appearance),
    ]),
  ];
}

export function placeScenery(
  site: ScenerySite,
  biome: Pick<Biome, "seed" | "trees" | "forest_floor">,
  sizes: ReadonlyMap<string, KindSize>,
): SceneryPlacement {
  const trees = biome.trees;
  const kinds = sceneryKinds(biome);
  const missing = kinds.filter((k) => !sizes.has(k));
  if (missing.length)
    throw new Error(
      `scenery: no installed appearance named ${missing.map((k) => `"${k}"`).join(", ")}`,
    );
  const size = kinds.map((k) => sizes.get(k)!);
  const pick = speciesAt(trees, kinds, biome.seed);
  return {
    kinds,
    forest: placeForests(site, trees, kinds, size, pick, biome.seed),
    understorey: placeUnderstorey(
      site,
      trees,
      biome.forest_floor.tree_line.taper_m,
      size,
      kinds.indexOf(trees.understorey.appearance),
      biome.seed,
    ),
    backdrop: placeBackdrop(
      site,
      trees,
      size,
      kinds.indexOf(trees.hedgerows.appearance),
      pick,
      biome.seed,
    ),
    dressing: dressingField(site, biome.forest_floor.dressing, kinds, size, biome.seed),
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

/** Parts the lone trees' streams from every other stream of the seed. */
const LONE_SALT = 0x10e7ee;

function placeForests(
  site: ScenerySite,
  trees: BiomeTrees,
  kinds: readonly string[],
  size: readonly KindSize[],
  pick: SpeciesPick,
  seed: number,
): Float32Array {
  const out = new Builder();
  const rules = trees.forest;
  const rng = stream(seed, 1);
  const ground = (x: number, y: number) => groundHeight(site.ground, x, y);
  /** The lowest ground round a trunk's foot at (x, y). */
  const footOf = (x: number, y: number) => {
    let foot = ground(x, y);
    for (let k = 0; k < 4; k++) {
      const a = (k * Math.PI) / 2;
      foot = Math.min(foot, ground(x + 0.5 * Math.cos(a), y + 0.5 * Math.sin(a)));
    }
    return foot;
  };
  const planted = new Uint8Array(site.trunkIds.length);
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
      const foot = footOf(x, y);
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
      planted[trunk++] = 1;
    }
  }
  // A trunk no forest generated stands alone: a tree as tall as its own
  // body, seeded by the body, so one tree's look never moves another's.
  const lone = trees.lone;
  const alone = trees.species
    .filter((s) => lone.species.includes(s.appearance))
    .map((s) => ({ ...s, kind: kinds.indexOf(s.appearance) }));
  for (let t = 0; t < planted.length; t++) {
    if (planted[t]) continue;
    const own = stream(seed ^ LONE_SALT, site.trunkIds[t]);
    const [x, y] = [site.trunks[t * 2], site.trunks[t * 2 + 1]];
    const s = weighted(own, alone);
    const top = site.trunkHeights[t] * random.float(own, lone.top[0], lone.top[1]);
    out.push(
      x,
      y,
      footOf(x, y) - SINK_M,
      random.float(own, 0, Math.PI * 2),
      random.float(own, lone.girth[0], lone.girth[1]),
      top / size[s.kind].height,
      s.kind,
      s.tint,
      own,
      trees.colour_jitter,
    );
  }
  return Float32Array.from(out.out);
}

/** Parts the understorey's streams from every other stream of the seed. */
const UNDERSTOREY_SALT = 0x5b2ab5;
/** A shrub turns this far from its row's line, radians, either way. */
const SHRUB_TURN = 0.12;

/** The shrubs under every strip of forest: rows of hedge along its length,
 *  as many as fit its width, each row's shrubs end to end and the next row's
 *  half a spacing on. The rows draw together over the last `taperM` before
 *  the strip's ends, where the ground's band under it narrows to a point. */
function placeUnderstorey(
  site: ScenerySite,
  trees: BiomeTrees,
  taperM: number,
  size: readonly KindSize[],
  kind: number,
  seed: number,
): Float32Array {
  const out = new Builder();
  const rules = trees.understorey;
  const shrub = size[kind];
  const ground = (x: number, y: number) => groundHeight(site.ground, x, y);
  /** Whether a shrub reaching `reach` from (x, y) stands under the
   *  simulation's foliage, as a crown does (in a fog cell that has it, or
   *  beside one), and clear of paving, water and every body but a tree. */
  const clear = (x: number, y: number, reach: number) => {
    if (site.wetOrPaved(x, y) > 0 || !site.foliageNear(x, y)) return false;
    for (let k = 0; k < 8; k++) {
      const a = (k * Math.PI) / 4;
      const [rx, ry] = [x + reach * Math.cos(a), y + reach * Math.sin(a)];
      if (site.wetOrPaved(rx, ry) > 0 || !site.foliageNear(rx, ry)) return false;
    }
    const b = site.obstacles;
    for (let o = 0; o < b.length; o += 3)
      if (Math.hypot(b[o] - x, b[o + 1] - y) < b[o + 2] + reach) return false;
    return true;
  };
  site.forests.forEach((f, index) => {
    if (f.kind !== "stroke") return;
    const s = f.strokes;
    const rng = stream(seed ^ UNDERSTOREY_SALT, index);
    // The strip's stretches run end to end: where each starts along it.
    const starts: number[] = [];
    let length = 0;
    for (let o = 0; o < s.length; o += STROKE_FLOATS) {
      starts.push(length);
      length += Math.hypot(s[o + 2] - s[o], s[o + 3] - s[o + 1]);
    }
    const half = s[4];
    const room = half - shrub.radius * rules.length[1];
    const rows = room > 0 ? 1 + Math.floor((2 * room) / rules.row_m) : 1;
    const count = Math.floor(length / rules.spacing_m);
    for (let row = 0; row < rows; row++) {
      const offset = (row - (rows - 1) / 2) * rules.row_m;
      for (let n = 0, stretch = 0; n < count; n++) {
        const along =
          ((n + 0.5 + (row % 2) * 0.5 + random.float(rng, -0.15, 0.15)) * length) / count;
        const sway = random.float(rng, -rules.sway_m, rules.sway_m);
        const long = random.float(rng, rules.length[0], rules.length[1]);
        const tall = random.float(rng, rules.height[0], rules.height[1]);
        const turn = random.float(rng, -SHRUB_TURN, SHRUB_TURN);
        if (random.bool(rng, rules.gap) || along >= length) continue;
        while (stretch + 1 < starts.length && starts[stretch + 1] <= along) stretch++;
        const o = stretch * STROKE_FLOATS;
        const run = Math.hypot(s[o + 2] - s[o], s[o + 3] - s[o + 1]);
        const [dx, dy] = [(s[o + 2] - s[o]) / run, (s[o + 3] - s[o + 1]) / run];
        const aside = (offset + sway) * Math.min(1, along / taperM, (length - along) / taperM);
        const x = s[o] + dx * (along - starts[stretch]) - dy * aside;
        const y = s[o + 1] + dy * (along - starts[stretch]) + dx * aside;
        // Its whole reach on the strip's own ground: a shrub is drawn smaller
        // to fit, and not at all where it would be smaller than its kind.
        const scale = Math.min(long, forestInside(f, x, y) / shrub.radius);
        if (scale < rules.length[0] || !clear(x, y, shrub.radius * scale)) continue;
        // On the lowest ground under its length.
        const reach = (shrub.radius * scale) / 2;
        const foot = Math.min(
          ground(x, y),
          ground(x + dx * reach, y + dy * reach),
          ground(x - dx * reach, y - dy * reach),
        );
        out.push(
          x,
          y,
          foot - SINK_M,
          Math.atan2(dy, dx) + turn,
          scale,
          tall,
          kind,
          rules.tint,
          rng,
          trees.colour_jitter,
        );
      }
    }
  });
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

/** Parts the dressing's streams from every other stream of the seed. */
const DRESSING_SALT = 0xd2e551;
/** The side of the cells trunks and bodies are bucketed in, metres. */
const CLEAR_CELL_M = 8;
/** The side of a cell of dressing, metres: the ground is dressed, kept and
 *  drawn a cell at a time. */
export const DRESSING_CELL_M = 64;

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

/** Whether a point stands clear of every trunk (by `trunkM` from its axis)
 *  and every body (by `bodyM` from its footprint's circle): the circles are
 *  bucketed where they reach. */
function clearOf(site: ScenerySite, trunkM: number, bodyM: number) {
  const cells = new Map<number, number[]>();
  const add = (x: number, y: number, r: number) => {
    const [i0, i1] = [Math.floor((x - r) / CLEAR_CELL_M), Math.floor((x + r) / CLEAR_CELL_M)];
    const [j0, j1] = [Math.floor((y - r) / CLEAR_CELL_M), Math.floor((y + r) / CLEAR_CELL_M)];
    for (let j = j0; j <= j1; j++)
      for (let i = i0; i <= i1; i++) {
        let list = cells.get(cellKey(i, j));
        if (!list) cells.set(cellKey(i, j), (list = []));
        list.push(x, y, r);
      }
  };
  for (let o = 0; o < site.trunks.length; o += 2) add(site.trunks[o], site.trunks[o + 1], trunkM);
  for (let o = 0; o < site.obstacles.length; o += 3)
    add(site.obstacles[o], site.obstacles[o + 1], site.obstacles[o + 2] + bodyM);
  return (x: number, y: number) => {
    const list = cells.get(cellKey(Math.floor(x / CLEAR_CELL_M), Math.floor(y / CLEAR_CELL_M)));
    if (list)
      for (let o = 0; o < list.length; o += 3)
        if ((list[o] - x) ** 2 + (list[o + 1] - y) ** 2 < list[o + 2] ** 2) return false;
    return true;
  };
}

/** The cells of side `cellM` the forests' own primitives reach (`i, j`
 *  pairs): a rectangle, each stretch of a strip with its width, each
 *  triangle of a polygon, by its box. */
function forestCells(forests: readonly ForestShape[], cellM: number): Int32Array {
  const cells = new Map<number, [number, number]>();
  const reach = (x0: number, y0: number, x1: number, y1: number) => {
    for (let j = Math.floor(y0 / cellM); j <= Math.floor(y1 / cellM); j++)
      for (let i = Math.floor(x0 / cellM); i <= Math.floor(x1 / cellM); i++)
        cells.set(cellKey(i, j), [i, j]);
  };
  for (const forest of forests) {
    if (forest.kind === "rectangle") {
      const [x, y, w, h] = forest.rect!;
      reach(x, y, x + w, y + h);
    }
    const s = forest.strokes;
    for (let o = 0; o < s.length; o += STROKE_FLOATS) {
      const half = s[o + 4];
      reach(
        Math.min(s[o], s[o + 2]) - half,
        Math.min(s[o + 1], s[o + 3]) - half,
        Math.max(s[o], s[o + 2]) + half,
        Math.max(s[o + 1], s[o + 3]) + half,
      );
    }
    const t = forest.triangles;
    for (let o = 0; o < t.length; o += FOREST_TRIANGLE_FLOATS)
      reach(
        Math.min(t[o], t[o + 2], t[o + 4]),
        Math.min(t[o + 1], t[o + 3], t[o + 5]),
        Math.max(t[o], t[o + 2], t[o + 4]),
        Math.max(t[o + 1], t[o + 3], t[o + 5]),
      );
  }
  return Int32Array.from([...cells.values()].flat());
}

/** How thick a kind's drift lies, 0 to 1, from its field's value in [-1, 1]:
 *  half the ground lies in a drift, with a short ramp at its edge. */
const drifted = (value: number) => Math.min(1, Math.max(0, value / 0.3 + 0.5));

function dressingField(
  site: ScenerySite,
  rules: ForestDressing,
  kinds: readonly string[],
  size: readonly KindSize[],
  seed: number,
): DressingField {
  const rows = rules.kinds.map((row) => ({ ...row, kind: kinds.indexOf(row.appearance) }));
  const clear = clearOf(site, rules.trunk_clear_m, rules.clear_m);
  // Each kind's drifts are its own smooth field over the ground, so a drift
  // runs on from one cell, and one wood, into the next.
  const drifts = rows.map((_, k) => simplex2d.create((seed ^ DRESSING_SALT) + k));
  const m = DRESSING_CELL_M;
  const capacity = Math.round((m * m * rules.per_ha) / 10000);
  let high = 0;
  for (const h of site.ground.heights) high = Math.max(high, h);
  return {
    cellM: m,
    capacity,
    kinds: [...new Set(rows.map((row) => row.kind))],
    topM: Math.max(0, ...rows.map((row) => size[row.kind].height * row.scale[1])),
    reachM: Math.max(0, ...rows.map((row) => size[row.kind].radius * row.scale[1])),
    ground: [site.ground.minHeight, high],
    cells: forestCells(site.forests, m),
    place(i, j) {
      const out = new Builder();
      // Seeded by the cell alone: a cell is the same whenever it is laid.
      const rng = stream(seed ^ DRESSING_SALT, cellKey(i, j));
      for (let n = 0; n < capacity; n++) {
        const [x, y] = [(i + random.float(rng, 0, 1)) * m, (j + random.float(rng, 0, 1)) * m];
        const k = rows.indexOf(weighted(rng, rows));
        const row = rows[k];
        // Between its drifts a kind thins to `1 - drift` of its pieces.
        const thick = drifted(simplex2d.sample(drifts[k], x / rules.drift_m, y / rules.drift_m));
        if (!random.bool(rng, 1 - row.drift * (1 - thick))) continue;
        if (site.forestInside(x, y) < rules.edge_m || !clear(x, y)) continue;
        if (site.wetOrPaved(x, y) > -rules.clear_m) continue;
        const scale = random.float(rng, row.scale[0], row.scale[1]);
        out.push(
          x,
          y,
          groundHeight(site.ground, x, y) - SINK_M,
          random.float(rng, 0, Math.PI * 2),
          scale,
          scale * random.float(rng, rules.squat, 1),
          row.kind,
          row.tint,
          rng,
          rules.colour_jitter,
        );
      }
      return Float32Array.from(out.out);
    },
  };
}
