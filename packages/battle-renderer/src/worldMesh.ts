// Presentation of the exported authoritative geometry, in layers. The ground
// is the terrain surface (`terrain/terrainSurface.ts`): the simulation's own
// triangles under the biome's material. Props and water are drawn from their
// exported shapes; forests are the scenery's trees (`scenery/placement.ts`),
// which draw the simulation's trunks too. Colours are presentation only.
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { grassAppearancesOf } from "./terrain/grassField";
import { MeshBuilder, type Mesh, type Rgba } from "./mesh";
import type { WorldLayers, WorldScenery } from "./scene";
import type { Biome } from "./terrain/biome";
import { BLOCKED, buildTerrainSurface, OPEN, type TerrainSurface } from "./terrain/terrainSurface";
import { kindSize, sceneryAppearances } from "./scenery/appearance";
import { placeScenery, scenerySite } from "./scenery/placement";

/** Parsed `world_layout()` from the WASM boundary. */
export interface WorldLayout {
  surfaceKinds: string[];
  propKinds: string[];
  /** Per mover class ("infantry", "vehicle"), the prop kinds that stop it. */
  blockingPropKinds: Record<string, string[]>;
  /** The prop kinds that hide what lies behind them from sight. */
  occludingPropKinds: string[];
  flags: { forest: number; blocked: number };
  propStride: number;
  areaStride: number;
  propFields: string[];
  areaFields: string[];
  roadStride: number;
  roadFields: string[];
}

export interface WorldExports {
  positions: Float32Array;
  indices: Uint32Array;
  /** Two bytes per triangle: ground kind tag, flags. */
  triangleSurfaces: Uint8Array;
  props: Float32Array;
  water: Float32Array;
  forests: Float32Array;
  /** Road segments: `ax, ay, bx, by, halfWidth` (`roadFields`). */
  roads: Float32Array;
}

/** "surface" draws the biome; "traversal" marks what ground units cannot enter. */
export type WorldOverlay = "surface" | "traversal";

/** Stops some mover classes but not others (a wreck stops vehicles only). */
const PARTLY_BLOCKED: Rgba = [0.86, 0.6, 0.22, 1];
const PROP_COLORS: Record<string, Rgba> = {
  building: [0.74, 0.7, 0.62, 1],
  wall: [0.6, 0.58, 0.55, 1],
  crate: [0.72, 0.62, 0.5, 1],
  trunk: [0.36, 0.28, 0.2, 1],
  bridgedeck: [0.58, 0.54, 0.48, 1],
  wreck: [0.25, 0.24, 0.23, 1],
  ruin: [0.55, 0.52, 0.48, 1],
};
const WATER_SURFACE: Rgba = [0.24, 0.42, 0.62, 0.72];
const SKIRT: Rgba = [0.33, 0.3, 0.26, 1];
const SKIRT_DEPTH_M = 6;
/** Drawn by the scenery's trees in the surface view; boxes only in the
 *  traversal view, where they show what blocks. */
const TREE_PROP_KINDS = ["trunk"];

function fieldReader(fields: string[], stride: number, data: Float32Array) {
  const offset = Object.fromEntries(fields.map((f, i) => [f, i]));
  return {
    count: data.length / stride,
    get: (row: number, field: string) => data[row * stride + offset[field]],
  };
}

/** Static props that can fall in battle: a route whose buildings may collapse
 *  draws them apart from the world mesh (see `buildStandingStructures`), so a
 *  collapse never rebuilds the whole world. */
const FALLIBLE_KINDS = ["building"];

/** A prop's traversal colour: how many mover classes it stops. */
function propTraversal(layout: WorldLayout, kind: string): Rgba {
  const classes = Object.values(layout.blockingPropKinds);
  const stops = classes.filter((kinds) => kinds.includes(kind)).length;
  return stops === 0 ? OPEN : stops === classes.length ? BLOCKED : PARTLY_BLOCKED;
}

function addProps(
  mesh: MeshBuilder,
  exports: WorldExports,
  layout: WorldLayout,
  overlay: WorldOverlay,
  keep: (id: number, kind: string) => boolean,
) {
  const props = fieldReader(layout.propFields, layout.propStride, exports.props);
  for (let r = 0; r < props.count; r++) {
    const kind = layout.propKinds[props.get(r, "kind")];
    if (!keep(props.get(r, "id"), kind)) continue;
    const color = overlay === "traversal" ? propTraversal(layout, kind) : PROP_COLORS[kind];
    mesh.orientedBox(
      props.get(r, "x"),
      props.get(r, "y"),
      props.get(r, "yaw"),
      [props.get(r, "hx"), props.get(r, "hy"), props.get(r, "hz")],
      props.get(r, "baseZ"),
      color,
    );
  }
}

/** The static buildings still standing as a side knows them: every one except
 *  those it has seen fall (whose ruins it draws from its known props). */
export function buildStandingStructures(
  exports: WorldExports,
  layout: WorldLayout,
  fallen: ReadonlySet<number>,
): Mesh {
  const mesh = new MeshBuilder();
  addProps(
    mesh,
    exports,
    layout,
    "surface",
    (id, kind) => FALLIBLE_KINDS.includes(kind) && !fallen.has(id),
  );
  return mesh.build();
}

/** The static world's layers: the ground under `biome`, the props on it,
 *  and, given the installed appearances, the scenery and the grass kinds (the
 *  surface view only).
 *  `structures: "apart"` leaves out props that can fall, for routes that draw
 *  them with `buildStandingStructures`. */
export function buildWorldLayers(
  exports: WorldExports,
  layout: WorldLayout,
  biome: Biome,
  overlay: WorldOverlay,
  structures: "with-world" | "apart" = "with-world",
  appearances: InstalledAppearances | null = null,
): WorldLayers {
  const props = new MeshBuilder();
  const translucent = new MeshBuilder();
  addSkirt(props, exports.positions);
  addProps(
    props,
    exports,
    layout,
    overlay,
    (_, kind) =>
      (structures === "with-world" || !FALLIBLE_KINDS.includes(kind)) &&
      (overlay === "traversal" || !TREE_PROP_KINDS.includes(kind)),
  );

  // The traversal overlay shows the blocked flag alone; a water tint would muddy it.
  const water = fieldReader(
    layout.areaFields,
    layout.areaStride,
    overlay === "traversal" ? new Float32Array(0) : exports.water,
  );
  for (let r = 0; r < water.count; r++) {
    const [x, y, w, h, z] = ["x", "y", "w", "h", "z"].map((f) => water.get(r, f));
    translucent.quad([x, y, z], [x + w, y, z], [x + w, y + h, z], [x, y + h, z], WATER_SURFACE);
  }

  const terrain = buildTerrainSurface(exports, layout, biome, overlay);
  return {
    terrain,
    props: props.build(),
    translucent: translucent.build(),
    scenery:
      overlay === "surface" && appearances
        ? worldScenery(exports, layout, terrain, biome, appearances)
        : null,
    grass: overlay === "surface" && appearances ? grassAppearancesOf(appearances) : null,
  };
}

/** The trees and hedgerows of `biome.trees`, instancing installed appearances. */
function worldScenery(
  exports: WorldExports,
  layout: WorldLayout,
  terrain: TerrainSurface,
  biome: Biome,
  installed: InstalledAppearances,
): WorldScenery {
  const trees = biome.trees;
  const names = [
    ...new Set([...trees.species.map((s) => s.appearance), trees.hedgerows.appearance]),
  ];
  const appearances = sceneryAppearances(installed, names);
  const sizes = new Map([...appearances].map(([name, bundle]) => [name, kindSize(bundle)]));
  return {
    placement: placeScenery(scenerySite(exports, layout, terrain), biome, sizes),
    appearances,
    lodPx: trees.lod_px,
  };
}

/** Vertical faces down from the map's boundary vertices, so the bounded map
 *  reads as solid ground rather than a paper-thin sheet. They stand on the
 *  ground, so they draw with the props, as faces. Presentation only. */
function addSkirt(mesh: MeshBuilder, positions: Float32Array) {
  let maxX = 0,
    maxY = 0,
    minZ = Infinity;
  for (let i = 0; i < positions.length; i += 3) {
    maxX = Math.max(maxX, positions[i]);
    maxY = Math.max(maxY, positions[i + 1]);
    minZ = Math.min(minZ, positions[i + 2]);
  }
  const bottom = minZ - SKIRT_DEPTH_M;
  const edges: [number, number, number][][] = [[], [], [], []];
  for (let i = 0; i < positions.length; i += 3) {
    const v: [number, number, number] = [positions[i], positions[i + 1], positions[i + 2]];
    if (v[1] === 0) edges[0].push(v);
    if (v[0] === maxX) edges[1].push(v);
    if (v[1] === maxY) edges[2].push(v);
    if (v[0] === 0) edges[3].push(v);
  }
  edges[0].sort((a, b) => a[0] - b[0]);
  edges[1].sort((a, b) => a[1] - b[1]);
  edges[2].sort((a, b) => b[0] - a[0]);
  edges[3].sort((a, b) => b[1] - a[1]);
  for (const edge of edges) {
    for (let k = 0; k + 1 < edge.length; k++) {
      const [a, b] = [edge[k], edge[k + 1]];
      mesh.quad(a, b, [b[0], b[1], bottom], [a[0], a[1], bottom], SKIRT);
    }
  }
}
