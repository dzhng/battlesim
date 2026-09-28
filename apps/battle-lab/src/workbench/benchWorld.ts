// The workbench's stage: a measured ground, a soldier-height scale figure, and the
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
import { SCENERY_KINDS, propsDrawnBy } from "@packages/scene-assets/src/scenery";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances";
import {
  REST_ARTICULATION,
  articulate,
  articulationRig,
  restLocals,
} from "@packages/scene-assets/src/articulation";
import { sampleClip, worldTransforms } from "@packages/scene-assets/src/pose";
import { mul, trsMatrix } from "@packages/scene-assets/src/trs";
import type {
  AppearanceUnit,
  Authority,
  Bundle,
  SkeletonClips,
} from "@packages/scene-assets/src/schema";
import { fixtureAuthority } from "@packages/scene-assets/src/authority";
import { UNITS } from "@packages/scene-assets/src/shippedUnits";

export const physics = village.physics;

/** The simulation's bodies and unit types: the fit authority the validator reads too. */
export const AUTHORITY: Authority = fixtureAuthority(village, UNITS);

/** Height of the scale figure: the rules' soldier. */
export const FIGURE_HEIGHT_M = physics.soldier_height_m;
/** The height the mannequin's proportions below are written at. */
const MANNEQUIN_DRAWN_M = 1.8;
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

/** A plain soldier-height mannequin standing at (x, y), facing +X. */
export function scaleFigure(x: number, y: number): Mesh {
  const m = new MeshBuilder();
  const k = FIGURE_HEIGHT_M / MANNEQUIN_DRAWN_M;
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
    {
      map: [-h, -h, h, h],
      roads: NONE,
      roadStride: 5,
      forests: NONE,
      water: NONE,
      buildings: [],
      footprints: NONE,
    },
    villageBiome,
    null,
  );
  return _bench_terrain;
}

export function benchWorld(figureAt: [number, number] | null): WorldLayers {
  return {
    terrain: benchTerrain(),
    props: figureAt ? scaleFigure(figureAt[0], figureAt[1]) : NONE,
    water: NONE,
    structures: [],
    scenery: null,
    grass: null,
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

/** What the simulation's `world_layout()` says of prop kinds. */
export interface PropClasses {
  /** Per mover class ("infantry", "vehicle"), the prop kinds that stop it. */
  blockingPropKinds: Record<string, string[]>;
  occludingPropKinds: string[];
}

/** The simulation's body beside a model: wireframe edges in model space and
 *  one line saying what the simulation makes of it. */
export interface Footprint {
  edges: [Vec3, Vec3][];
  label: string;
}

type Edges = [Vec3, Vec3][];

function cylinder(edges: Edges, r: number, z0: number, z1: number, n = 16) {
  for (let i = 0; i < n; i++) {
    const a = (2 * Math.PI * i) / n;
    const b = (2 * Math.PI * (i + 1)) / n;
    for (const z of [z0, z1])
      edges.push([
        [Math.cos(a) * r, Math.sin(a) * r, z],
        [Math.cos(b) * r, Math.sin(b) * r, z],
      ]);
    if (i % 4 === 0)
      edges.push([
        [Math.cos(a) * r, Math.sin(a) * r, z0],
        [Math.cos(a) * r, Math.sin(a) * r, z1],
      ]);
  }
}

/** A box of half extents `half` (along heading, across, vertical) on the ground. */
function box(edges: Edges, half: readonly number[]) {
  const corner = (sx: number, sy: number, sz: number): Vec3 => [
    sx * half[0],
    sy * half[1],
    sz > 0 ? 2 * half[2] : 0,
  ];
  for (const s of [-1, 1])
    for (const t of [-1, 1]) {
      edges.push([corner(-1, s, t), corner(1, s, t)]);
      edges.push([corner(s, -1, t), corner(s, 1, t)]);
      edges.push([corner(s, t, -1), corner(s, t, 1)]);
    }
}

const m = (v: number) => `${+v.toFixed(2)}`;

/** The map's first prop of a kind: the size the simulation places it at. */
function placedProp(kind: string): number[] | null {
  const props = village.map.props as { kind: string; half_extents: number[] }[];
  return props.find((p) => p.kind.replace(/_/g, "") === kind)?.half_extents ?? null;
}

function propLabel(kind: string, classes: PropClasses | null): string {
  if (!classes) return `${kind}`;
  const stops = Object.entries(classes.blockingPropKinds)
    .filter(([, kinds]) => kinds.includes(kind))
    .map(([mover]) => mover);
  return `${kind} · stops ${stops.length ? stops.join(" and ") : "no mover"} · ${
    classes.occludingPropKinds.includes(kind) ? "hides what is behind it" : "does not block sight"
  }`;
}

/** What the simulation knows of the thing an appearance draws; a vehicle's
 *  hit box is its unit type's (`type`) hull. */
export function footprint(
  unit: AppearanceUnit,
  scenery: string | null,
  classes: PropClasses | null,
  authored: Vec3 | null = null,
  type: string | null = null,
): Footprint {
  const edges: Edges = [];
  if (unit === "soldier") {
    cylinder(edges, physics.soldier_radius_m, 0, physics.soldier_height_m);
    return {
      edges,
      label: `soldier: ${m(physics.soldier_radius_m)} m radius, ${m(physics.soldier_height_m)} m tall`,
    };
  }
  if (unit === "vehicle") {
    const hull = type ? UNITS.hull(type) : null;
    if (!hull) return { edges, label: "a vehicle of no unit type: no hit box" };
    box(edges, hull.half_extents_m);
    return {
      edges,
      label: `${type} hit box ${hull.half_extents_m.map((h) => m(2 * h)).join(" × ")} m`,
    };
  }
  const rule = unit === "building" ? null : scenery ? SCENERY_KINDS[scenery] : undefined;
  // The prop types it draws, by the prop catalog's `drawn_by`: the first the
  // map places stands for them.
  const drawn =
    unit === "building" || rule?.footprint.kind === "prop"
      ? propsDrawnBy(UNITS.view.props, unit === "building" ? "building" : (scenery ?? ""))
      : [];
  const prop = drawn.find((p) => placedProp(p)) ?? drawn[0] ?? null;
  if (prop) {
    // The box the art is authored to (the catalog's footprint), else the
    // map's first placement of the kind.
    const half = authored ?? placedProp(prop);
    if (half) box(edges, half);
    return {
      edges,
      label: `${propLabel(prop, classes)} · ${
        half
          ? `box ${half.map((h) => m(2 * h)).join(" × ")} m (${authored ? "its footprint" : "the map's first"})`
          : "none placed in the map"
      }`,
    };
  }
  if (rule?.footprint.kind === "tree") {
    const forest = village.map.forests[0];
    const density =
      village.forests.densities[forest.density as keyof typeof village.forests.densities];
    cylinder(edges, forest.trunk_radius_m, 0, forest.trunk_height_m, 12);
    cylinder(edges, density.canopy_radius_m, forest.canopy_height_m, forest.canopy_height_m, 24);
    return {
      edges,
      label: `forest tree (${forest.density}): trunk ${m(2 * forest.trunk_radius_m)} m × ${m(forest.trunk_height_m)} m, crown ${m(density.canopy_radius_m)} m, canopy at ${m(forest.canopy_height_m)} m, ${m(density.trunk_spacing_m)} m apart · trunks stop rounds and vehicles, not soldiers; a heavy vehicle knocks them down`,
    };
  }
  return { edges, label: scenery ? `${scenery}: no simulation body` : "no simulation body" };
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
  body: Footprint | null,
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
    if (show.hitBox && body)
      for (const [a, b] of body.edges) mesh.segment(place(a), place(b), 0.012 * scale, HIT_BOX);
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
