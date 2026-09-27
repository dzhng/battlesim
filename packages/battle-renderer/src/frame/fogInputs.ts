// What renderer fog is cut from, assembled on the CPU. Three inputs, each with
// one owner:
// - `FogGeometryPresentation`: `presentation.fog_geometry`, the technique's
//   resolution and budgets (spike 02's numbers, provisional);
// - `FogWorld`: the public static map sight is cut by (the simulation's terrain
//   grid, sampled with its triangle rule, and the foliage its standing trees
//   give each 8 m cell; slice 34b), plus the sensor
//   rule numbers fog shares with the simulation's sweep;
// - `FogSight`: per publication, every own eye (`OwnUnit.sight` at the
//   published tick, never interpolated) and the occluders the side knows
//   stand: the static map's occluding props less those it has seen fall, plus
//   the obstacles, ruins and wrecks it has learned. A prop the side has not
//   learned never occludes.
import type { WorldExports, WorldLayout } from "../worldMesh";
import type { SightLobe, SightShape } from "../sightOverlay";
import type { KnownProp } from "../models/propAppearance";
import { terrainGrid, type TerrainGrid } from "../terrain/terrainGrid";

/** `presentation.fog_geometry`: resolution and budgets of the sight lights. */
export interface FogGeometryPresentation {
  /** Rays per eye of the full map, where buildings are intersected exactly. */
  azimuth_bins: number;
  /** Rays per eye of the coarse terrain march. */
  terrain_azimuth_bins: number;
  /** Log-spaced radial bins per ray, from `first_bin_m` to the eye's reach. */
  radial_bins: number;
  first_bin_m: number;
  /** The terrain march's step: `fraction` of the distance, within `[min, max]` metres. */
  terrain_step_m: readonly [number, number];
  terrain_step_fraction: number;
  /** Screen tile of the per-tile eye lists, pixels a side. */
  tile_px: number;
  /** Eyes a tile's list holds; more are dropped (and counted). */
  tile_eyes_max: number;
  /** A face probes this far outside itself, along its normal. */
  face_probe_m: number;
  /** A roof or canopy top above an eye looks this far toward it for seen air
   *  at its own height (fogTerm.ts); 0 leaves such surfaces as sight says. */
  roof_reach_m: number;
  /** Moved eyes rebuilt per frame; the rest keep their last map until their
   *  turn. New eyes are always built at once. */
  rebuild_eyes_per_frame: number;
}

export function validateFogGeometry(g: FogGeometryPresentation): FogGeometryPresentation {
  const whole = (v: number, name: string, min = 1) => {
    if (!Number.isInteger(v) || v < min)
      throw new Error(`fog_geometry.${name} must be an integer ≥ ${min}`);
  };
  whole(g.azimuth_bins, "azimuth_bins", 8);
  whole(g.terrain_azimuth_bins, "terrain_azimuth_bins", 8);
  whole(g.radial_bins, "radial_bins", 2);
  whole(g.tile_px, "tile_px");
  whole(g.tile_eyes_max, "tile_eyes_max");
  whole(g.rebuild_eyes_per_frame, "rebuild_eyes_per_frame");
  // One cull workgroup per tile: WebGPU guarantees 256 invocations.
  if (g.tile_px * g.tile_px > 256) throw new Error("fog_geometry.tile_px must be ≤ 16");
  if (!(g.first_bin_m > 0 && g.face_probe_m > 0 && g.terrain_step_fraction > 0))
    throw new Error(
      "fog_geometry: first_bin_m, face_probe_m and terrain_step_fraction must be > 0",
    );
  if (!(g.roof_reach_m >= 0)) throw new Error("fog_geometry.roof_reach_m must be ≥ 0");
  const [lo, hi] = g.terrain_step_m;
  if (!(lo > 0 && hi >= lo)) throw new Error("fog_geometry.terrain_step_m must be 0 < min ≤ max");
  return g;
}

/** The sensor rules fog shares with the simulation's sweep (`rules.sensors`). */
export interface FogSensorRules {
  fog_target_height_m: number;
  foliage_full_block: number;
}

/** Foliage depth is stored per bin in steps of this, in 8 bits. */
export const FOLIAGE_STEP = 0.005;
export const FOLIAGE_MAX = 255 * FOLIAGE_STEP;

/** The public static map sight is cut by. */
export interface FogWorld extends TerrainGrid {
  /** The simulation's foliage grid (`WorldView.foliage`): `nx, ny, cell_m`,
   *  then `canopy_m, depth_per_m` per cell, row-major. */
  foliage: Float32Array;
  targetHeightM: number;
  foliageFullBlock: number;
}

/** The static world fog reads, from the simulation's own exported geometry. */
export function fogWorld(exports: WorldExports, sensors: FogSensorRules): FogWorld {
  // Foliage saturates at 8 bits: the full block must fall inside that.
  if (sensors.foliage_full_block > FOLIAGE_MAX)
    throw new Error(`fog: foliage_full_block must be ≤ ${FOLIAGE_MAX}`);
  return {
    ...terrainGrid(exports),
    foliage: exports.foliage,
    targetHeightM: sensors.fog_target_height_m,
    foliageFullBlock: sensors.foliage_full_block,
  };
}

/** An oriented box that hides what lies behind it. */
export interface FogOccluder {
  x: number;
  y: number;
  yaw: number;
  /** Half extents along and across the heading. */
  hx: number;
  hy: number;
  /** Height of its top. */
  top: number;
}

/** The occluders a side knows stand: the static map's occluding props less
 *  the ones it has seen fall, plus the learned props that occlude. */
export function knownOccluders(
  exports: WorldExports,
  layout: WorldLayout,
  known: readonly KnownProp[],
): FogOccluder[] {
  const occludes = new Set(layout.occludingPropKinds);
  const fallen = new Set(known.flatMap((p) => (p.replaces === null ? [] : [p.replaces])));
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const props = exports.props;
  const out: FogOccluder[] = [];
  for (let r = 0; r < props.length; r += layout.propStride) {
    const kind = layout.propKinds[props[r + at.kind]];
    if (!occludes.has(kind) || fallen.has(props[r + at.id])) continue;
    out.push({
      x: props[r + at.x],
      y: props[r + at.y],
      yaw: props[r + at.yaw],
      hx: props[r + at.hx],
      hy: props[r + at.hy],
      top: props[r + at.baseZ] + 2 * props[r + at.hz],
    });
  }
  for (const p of known) {
    if (!occludes.has(p.kind) || p.destroyed) continue;
    out.push({
      x: p.center[0],
      y: p.center[1],
      yaw: p.yaw,
      hx: p.half[0],
      hy: p.half[1],
      top: p.baseZ + 2 * p.half[2],
    });
  }
  return out;
}

/** One own eye at the published tick. */
export interface FogEye {
  /** Stable per unit and eye slot, so an eye that did not move keeps its map. */
  key: string;
  position: readonly [number, number, number];
  forward: number;
  shape: SightShape;
  range: number;
}

/** What the side sees from, and what it knows stands in the way. */
export interface FogSight {
  eyes: readonly FogEye[];
  /** Replaced whole when knowledge changes: a new array rebuilds every map. */
  occluders: readonly FogOccluder[];
}

/** Every eye of every own unit (a garrison gives one per occupied slot). */
export function fogEyes(own: readonly { id: number; sight: SightLobe }[]): FogEye[] {
  return own.flatMap((u) =>
    u.sight.eyes.map((position, k) => ({
      key: `${u.id}:${k}`,
      position,
      forward: u.sight.forward,
      shape: u.sight.shape,
      range: u.sight.range,
    })),
  );
}

/** How far an eye reaches in any direction: its map's radius. */
export function eyeReach(eye: FogEye): number {
  return eye.range * Math.max(eye.shape.front, eye.shape.side, eye.shape.rear);
}

/** Everything fog is drawn from; `null` draws the world clear. */
export interface FogInput {
  world: FogWorld;
  sight: FogSight;
}
