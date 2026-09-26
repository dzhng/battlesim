// The workbench's stage: a measured ground, a 1.8 m scale figure, and the
// judging marks drawn over the model — the simulation's hit box as a
// wireframe and a gizmo at every socket. All ordinary battle-frame layers:
// the ground and figure are world meshes, lit and shadowed with the model;
// the marks are display-space overlays.

import { vec3, type Mat4, type Vec3 } from "math";
import village from "@fixtures/village.json";
import { MeshBuilder, type Mesh, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { WorldLayers, WorldMeshes } from "@packages/battle-renderer/src/scene";
import {
  terrainSurface,
  type TerrainSurface,
} from "@packages/battle-renderer/src/terrain/terrainSurface";
import { villageBiome } from "../villageBiome";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import {
  REST_ARTICULATION,
  articulate,
  articulationRig,
  restLocals,
} from "@packages/scene-assets/src/articulation";
import { sampleClip, worldTransforms } from "@packages/scene-assets/src/pose";
import { mul, trsMatrix } from "@packages/scene-assets/src/trs";
import type { Authority, Bundle, SkeletonClips, UnitKind } from "@packages/scene-assets/src/schema";

export const physics = village.physics;

/** The simulation's bodies: the fit authority the validator reads too. */
export const AUTHORITY: Authority = {
  soldier_height_m: physics.soldier_height_m,
  infantry_eye_m: physics.infantry_eye_m,
  infantry_muzzle_m: physics.infantry_muzzle_m,
  tank_half_extents_m: physics.tank_half_extents_m as Vec3,
  tank_muzzle_local_m: physics.tank_muzzle_local_m as Vec3,
  supply_half_extents_m: physics.supply_half_extents_m as Vec3,
};

/** Height of the scale figure: a 1.8 m person. */
export const FIGURE_HEIGHT_M = 1.8;
const GROUND_HALF_M = 40;
const GROUND_REACH_M = 250;
const GROUND: Rgba = [0.46, 0.47, 0.44, 1];
const GROUND_ALT: Rgba = [0.43, 0.44, 0.41, 1];
const LINE_1M: Rgba = [0.36, 0.37, 0.35, 1];
const LINE_5M: Rgba = [0.26, 0.27, 0.25, 1];
const FIGURE: Rgba = [0.5, 0.52, 0.56, 1];

/** Flat ground: 5 m checks out to the battle views' reach, with 1 m and
 *  5 m lines over the middle 80 m, for scale. */
export function benchGround(): Mesh {
  const mesh = new MeshBuilder();
  const h = GROUND_HALF_M;
  const far = GROUND_REACH_M;
  for (let x = -far; x < far; x += 5)
    for (let y = -far; y < far; y += 5) {
      const color = (x / 5 + y / 5) & 1 ? GROUND_ALT : GROUND;
      mesh.quad([x, y, 0], [x + 5, y, 0], [x + 5, y + 5, 0], [x, y + 5, 0], color);
    }
  // Grid lines sit a hair above the ground so depth keeps them on top.
  for (let k = -h; k <= h; k += 1) {
    const color = k % 5 === 0 ? LINE_5M : LINE_1M;
    const w = k % 5 === 0 ? 0.03 : 0.012;
    mesh.quad([k - w, -h, 0.002], [k + w, -h, 0.002], [k + w, h, 0.002], [k - w, h, 0.002], color);
    mesh.quad([-h, k - w, 0.002], [h, k - w, 0.002], [h, k + w, 0.002], [-h, k + w, 0.002], color);
  }
  return mesh.build();
}

/** A plain 1.8 m mannequin standing at (x, y), facing +X. */
export function scaleFigure(x: number, y: number): Mesh {
  const m = new MeshBuilder();
  const k = FIGURE_HEIGHT_M / 1.8;
  m.box(x, y + 0.1 * k, 0.43 * k, 0.07 * k, 0.07 * k, 0.43 * k, FIGURE); // legs
  m.box(x, y - 0.1 * k, 0.43 * k, 0.07 * k, 0.07 * k, 0.43 * k, FIGURE);
  m.box(x, y, 1.18 * k, 0.12 * k, 0.21 * k, 0.32 * k, FIGURE); // torso
  m.box(x, y + 0.27 * k, 1.12 * k, 0.05 * k, 0.05 * k, 0.33 * k, FIGURE); // arms
  m.box(x, y - 0.27 * k, 1.12 * k, 0.05 * k, 0.05 * k, 0.33 * k, FIGURE);
  m.prism(x, y, 1.52 * k, 0.11 * k, 0.22 * k, 10, FIGURE); // neck and head
  m.prism(x, y, 1.74 * k, 0.1 * k, 0.06 * k, 10, FIGURE);
  return m.build();
}

const NONE = new Float32Array(0);
/** The measured ground as terrain: its vertex tints are opaque, so it draws
 *  as tinted, not as the biome's patchwork. Built once; only the figure moves. */
let _bench_terrain: TerrainSurface | null = null;
function benchTerrain(): TerrainSurface {
  const h = GROUND_REACH_M;
  _bench_terrain ??= terrainSurface(
    benchGround(),
    { map: [-h, -h, h, h], roads: NONE, roadStride: 5, forests: NONE, water: NONE, buildings: [] },
    villageBiome,
  );
  return _bench_terrain;
}

export function benchWorld(figureAt: [number, number] | null): WorldLayers {
  return {
    terrain: benchTerrain(),
    props: figureAt ? scaleFigure(figureAt[0], figureAt[1]) : NONE,
    translucent: NONE,
  };
}

const HIT_BOX: Rgba = [1, 0.25, 0.85, 1];
const AXES: [Vec3, Rgba][] = [
  [
    [1, 0, 0],
    [0.95, 0.2, 0.15, 1],
  ],
  [
    [0, 1, 0],
    [0.25, 0.85, 0.25, 1],
  ],
  [
    [0, 0, 1],
    [0.25, 0.45, 1, 1],
  ],
];

/** The simulation's body for a unit kind, as wireframe edges in model space. */
export function hitBoxEdges(unit: UnitKind): [Vec3, Vec3][] {
  const edges: [Vec3, Vec3][] = [];
  if (unit === "rifle" || unit === "recon" || unit === "at") {
    const r = physics.soldier_radius_m;
    const top = physics.soldier_height_m;
    const n = 16;
    for (let i = 0; i < n; i++) {
      const a = (2 * Math.PI * i) / n;
      const b = (2 * Math.PI * (i + 1)) / n;
      for (const z of [0, top])
        edges.push([
          [Math.cos(a) * r, Math.sin(a) * r, z],
          [Math.cos(b) * r, Math.sin(b) * r, z],
        ]);
      if (i % 4 === 0)
        edges.push([
          [Math.cos(a) * r, Math.sin(a) * r, 0],
          [Math.cos(a) * r, Math.sin(a) * r, top],
        ]);
    }
    return edges;
  }
  if (unit === "building") return edges;
  const half = unit === "tank" ? physics.tank_half_extents_m : physics.supply_half_extents_m;
  const corner = (sx: number, sy: number, sz: number): Vec3 => [
    sx * half[0],
    sy * half[1],
    sz > 0 ? 2 * half[2] : 0,
  ];
  for (const s of [-1, 1]) {
    for (const t of [-1, 1]) {
      edges.push([corner(-1, s, t), corner(1, s, t)]);
      edges.push([corner(s, -1, t), corner(s, 1, t)]);
      edges.push([corner(s, t, -1), corner(s, t, 1)]);
    }
  }
  return edges;
}

export interface Socket {
  name: string;
  /** World transform of the socket in model space. */
  frame: Mat4;
}

/** Sockets in the model's current pose: a body's named sockets, a vehicle's
 *  muzzles, mast head and mount pivots. CPU-posed with scene-assets, the same
 *  functions the validator measures fit with. */
export function posedSockets(
  bundle: Exclude<Bundle, SkeletonClips>,
  skeleton: SkeletonClips | null,
  pose: ModelInstance["pose"],
): Socket[] {
  if (bundle.kind === "skinned") {
    const clip = pose.kind === "skinned" ? skeleton?.clips.find((c) => c.name === pose.clip) : null;
    const locals =
      clip && skeleton && pose.kind === "skinned"
        ? sampleClip(skeleton, clip, bundle.joints, pose.phase)
        : bundle.joints.map((j) => j.bind);
    const worlds = worldTransforms(
      bundle.joints.map((j) => j.parent),
      locals,
    );
    return bundle.sockets.map((socket) => ({
      name: socket.name,
      frame: mul(worlds[socket.joint], trsMatrix(socket.offset)),
    }));
  }
  if (bundle.kind === "articulated") {
    const input = pose.kind === "articulated" ? pose.articulation : REST_ARTICULATION;
    const nodes = bundle.nodes;
    const worlds = worldTransforms(
      nodes.map((n) => n.parent),
      articulate(restLocals(nodes), nodes, articulationRig(nodes), input),
    );
    return nodes
      .map((n, i) => ({ name: n.name, frame: worlds[i] }))
      .filter((s) => /muzzle$|_head$|^turret$|^gun$|^hmg$/.test(s.name));
  }
  return [];
}

/** Overlay marks: the hit box and socket gizmos, placed with the model. */
export function benchOverlay(
  model: { x: number; y: number; z: number; yaw: number } | null,
  unit: UnitKind | null,
  sockets: Socket[],
  show: { hitBox: boolean; sockets: boolean },
  scale: number,
): WorldMeshes {
  const mesh = new MeshBuilder();
  if (model) {
    const c = Math.cos(model.yaw);
    const s = Math.sin(model.yaw);
    const place = (p: Vec3): [number, number, number] => [
      model.x + p[0] * c - p[1] * s,
      model.y + p[0] * s + p[1] * c,
      model.z + p[2],
    ];
    if (show.hitBox && unit)
      for (const [a, b] of hitBoxEdges(unit))
        mesh.segment(place(a), place(b), 0.012 * scale, HIT_BOX);
    if (show.sockets)
      for (const socket of sockets) {
        const origin: Vec3 = [socket.frame[12], socket.frame[13], socket.frame[14]];
        for (const [axis, color] of AXES) {
          const dir = vec3.transformMat4(vec3.create(), axis, socket.frame);
          vec3.subtract(dir, dir, origin);
          vec3.normalize(dir, dir);
          const tip = vec3.scaleAndAdd(vec3.create(), origin, dir, 0.18 * scale);
          mesh.segment(place(origin), place(tip), 0.008 * scale, color);
        }
      }
  }
  return { opaque: mesh.build(), translucent: new Float32Array(0) };
}
