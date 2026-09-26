// Tiny, deterministic GLBs for the scene-assets tests: a valid rifleman on a
// Quaternius-shaped rig (glTF Y up, facing +Z, so the catalog's yaw is 90°), a
// tank, a supply truck and a building authored in engine space, and knobs that
// break exactly one rule each (the golden failures). Adapted from ~/dev/game's
// `bake/make-test-glb.mjs`: generated in code, never a checked-in blob.

import { encodeGlb, type GltfJson } from "@packages/scene-assets/src/glb.ts";
import { quat, type Mat4, type Quat, type Vec3 } from "math";
import { inverse, mul, trsMatrix } from "@packages/scene-assets/src/trs.ts";
import type {
  Authority,
  Catalog,
  SkeletonEntry,
  Tolerances,
} from "@packages/scene-assets/src/schema.ts";

export const AUTHORITY: Authority = {
  soldier_height_m: 1.7,
  infantry_eye_m: 1.6,
  infantry_muzzle_m: 1.4,
  tank_half_extents_m: [3.5, 1.8, 1.2],
  tank_muzzle_local_m: [3, 0, 2],
  supply_half_extents_m: [3, 1.4, 1.8],
};

export const TOLERANCES: Tolerances = {
  ground_m: 0.01,
  soldier_height_m: 0.05,
  eye_m: 0.08,
  muzzle_m: 0.1,
  hull_extent_m: 0.1,
  tank_muzzle_m: 0.05,
  muzzle_arc_m: 0.01,
};

interface NodeSpec {
  name: string;
  t?: Vec3;
  r?: Quat;
  s?: Vec3;
  mesh?: number;
  skin?: number;
  children?: number[];
  extras?: Record<string, unknown>;
}

/** A minimal glTF writer: accessors, box meshes, nodes, one skin, animations. */
export class GltfBuilder {
  json: GltfJson = {
    asset: { version: "2.0", generator: "scene-assets synthetic" },
    scene: 0,
    scenes: [{ nodes: [] }],
    nodes: [],
    meshes: [],
    accessors: [],
    bufferViews: [],
    materials: [
      {
        name: "paint",
        pbrMetallicRoughness: {
          baseColorFactor: [0.4, 0.45, 0.3, 1],
          metallicFactor: 0,
          roughnessFactor: 0.8,
        },
      },
    ],
  };
  private chunks: Uint8Array[] = [];
  private length = 0;

  accessor(
    data: Float32Array | Uint16Array | Uint32Array,
    type: string,
    extra: GltfJson = {},
  ): number {
    const bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    const padded = new Uint8Array(Math.ceil(bytes.byteLength / 4) * 4);
    padded.set(bytes);
    this.json.bufferViews.push({
      buffer: 0,
      byteOffset: this.length,
      byteLength: bytes.byteLength,
    });
    this.chunks.push(padded);
    this.length += padded.byteLength;
    const componentType =
      data instanceof Float32Array ? 5126 : data instanceof Uint16Array ? 5123 : 5125;
    const width = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 }[type]!;
    this.json.accessors.push({
      bufferView: this.json.bufferViews.length - 1,
      componentType,
      type,
      count: data.length / width,
      ...extra,
    });
    return this.json.accessors.length - 1;
  }

  /** An axis-aligned box from `min` to `max`, flat-shaded; `influence(p)` skins each vertex. */
  box(
    min: Vec3,
    max: Vec3,
    influence?: (p: Vec3) => [number[], number[]],
    attributes: GltfJson = {},
  ): number {
    const positions: number[] = [];
    const normals: number[] = [];
    const joints: number[] = [];
    const weights: number[] = [];
    const indices: number[] = [];
    const faces: [number, number][] = [
      [0, 1],
      [0, -1],
      [1, 1],
      [1, -1],
      [2, 1],
      [2, -1],
    ];
    for (const [axis, sign] of faces) {
      const [u, v] = [(axis + 1) % 3, (axis + 2) % 3];
      const base = positions.length / 3;
      for (const [a, b] of [
        [0, 0],
        [1, 0],
        [1, 1],
        [0, 1],
      ]) {
        const p: Vec3 = [0, 0, 0];
        p[axis] = sign > 0 ? max[axis] : min[axis];
        p[u] = a ? max[u] : min[u];
        p[v] = b ? max[v] : min[v];
        positions.push(...p);
        const n = [0, 0, 0];
        n[axis] = sign;
        normals.push(...n);
        if (influence) {
          const [j, w] = influence(p);
          joints.push(...j);
          weights.push(...w);
        }
      }
      indices.push(
        ...(sign > 0
          ? [base, base + 1, base + 2, base, base + 2, base + 3]
          : [base, base + 2, base + 1, base, base + 3, base + 2]),
      );
    }
    const attrs: GltfJson = {
      POSITION: this.accessor(Float32Array.from(positions), "VEC3", { min, max }),
      NORMAL: this.accessor(Float32Array.from(normals), "VEC3"),
      ...attributes,
    };
    if (influence) {
      attrs.JOINTS_0 = this.accessor(Uint16Array.from(joints), "VEC4");
      attrs.WEIGHTS_0 = this.accessor(Float32Array.from(weights), "VEC4");
    }
    this.json.meshes.push({
      primitives: [
        {
          attributes: attrs,
          indices: this.accessor(Uint16Array.from(indices), "SCALAR"),
          material: 0,
        },
      ],
    });
    return this.json.meshes.length - 1;
  }

  node(spec: NodeSpec): number {
    const node: GltfJson = { name: spec.name };
    if (spec.t) node.translation = spec.t;
    if (spec.r) node.rotation = spec.r;
    if (spec.s) node.scale = spec.s;
    if (spec.mesh !== undefined) node.mesh = spec.mesh;
    if (spec.skin !== undefined) node.skin = spec.skin;
    if (spec.children?.length) node.children = spec.children;
    if (spec.extras) node.extras = spec.extras;
    this.json.nodes.push(node);
    return this.json.nodes.length - 1;
  }

  roots(...roots: number[]) {
    this.json.scenes[0].nodes = roots;
  }

  /** World matrix of a node in glTF space (parents must be linked already). */
  world(index: number): Mat4 {
    const parent = this.json.nodes.findIndex((n: GltfJson) => n.children?.includes(index));
    const n = this.json.nodes[index];
    const local = trsMatrix({
      t: n.translation ?? [0, 0, 0],
      r: n.rotation ?? [0, 0, 0, 1],
      s: n.scale ?? [1, 1, 1],
    });
    return parent < 0 ? local : mul(this.world(parent), local);
  }

  skin(joints: number[]): number {
    const inverseBinds = joints.flatMap((j) => inverse(this.world(j)));
    this.json.skins = [
      ...(this.json.skins ?? []),
      { joints, inverseBindMatrices: this.accessor(Float32Array.from(inverseBinds), "MAT4") },
    ];
    return this.json.skins.length - 1;
  }

  animation(
    name: string,
    channels: {
      node: number;
      path: "rotation" | "translation";
      times: number[];
      values: number[];
    }[],
  ) {
    const samplers: GltfJson[] = [];
    const out: GltfJson[] = [];
    for (const c of channels) {
      samplers.push({
        input: this.accessor(Float32Array.from(c.times), "SCALAR", {
          min: [c.times[0]],
          max: [c.times[c.times.length - 1]],
        }),
        output: this.accessor(Float32Array.from(c.values), c.path === "rotation" ? "VEC4" : "VEC3"),
        interpolation: "LINEAR",
      });
      out.push({ sampler: samplers.length - 1, target: { node: c.node, path: c.path } });
    }
    this.json.animations = [...(this.json.animations ?? []), { name, samplers, channels: out }];
  }

  glb(): Uint8Array {
    const bin = new Uint8Array(this.length);
    let offset = 0;
    for (const chunk of this.chunks) {
      bin.set(chunk, offset);
      offset += chunk.byteLength;
    }
    return encodeGlb(this.json, bin);
  }
}

const qx = (deg: number): Quat =>
  quat.setAxisAngle(quat.create(), [1, 0, 0], (deg * Math.PI) / 180);

// ---------------------------------------------------------------- rifleman

export interface SoldierOptions {
  lift?: number; // raise everything (breaks the ground rule)
  top?: number; // body top in metres (breaks height)
  eyeY?: number;
  muzzleY?: number;
  extraJoint?: boolean; // a weighted joint the skeleton lacks
  looseMesh?: boolean; // a mesh neither skinned nor under a joint
  fiveWeights?: boolean;
  noNormals?: boolean;
  stretch?: boolean; // nonuniform node scale
  clips?: string[]; // animations to include
  noLods?: boolean;
  animated?: boolean; // include animations at all (default true)
}

export const SOLDIER_CLIPS = [
  "idle",
  "walk",
  "run",
  "kneel_fire",
  "prone_pinned",
  "death",
  "stand_aim",
];

/**
 * A 1.70 m rifleman in glTF Y-up space facing +Z. The armature node scales
 * by 0.5 and the joints are laid out at double size, so the bake must fold
 * the armature into the root joint. Eye at 1.60, rifle held at 1.40.
 */
export function soldierGlb(o: SoldierOptions = {}): Uint8Array {
  const g = new GltfBuilder();
  const lift = o.lift ?? 0;
  const k = 2; // joint layout is at 2x, the armature halves it
  const eyeY = o.eyeY ?? 1.6;
  const muzzleY = o.muzzleY ?? 1.4;
  // Joints (children first so we can link): leaf, hand, head, spine, pelvis, root.
  const leaf = g.node({ name: "index_04_leaf_r", t: [0, 0, 0.05 * k] });
  const eye = g.node({ name: "eye", t: [0, (eyeY - 1.5) * k, 0.1 * k] });
  const head = g.node({ name: "Head", t: [0, 0.3 * k, 0], children: [eye] });
  const muzzle = g.node({ name: "muzzle", t: [0, (muzzleY - 1.4) * k, 0.6 * k] });
  const rifleMesh = g.box([-0.02 * k, -0.03 * k, -0.2 * k], [0.02 * k, 0.03 * k, 0.6 * k]);
  const rifle = g.node({ name: "rifle", mesh: rifleMesh, children: [muzzle] });
  const hand = g.node({ name: "hand_r", t: [-0.2 * k, 0.2 * k, 0.1 * k], children: [leaf, rifle] });
  const extra = o.extraJoint ? g.node({ name: "pouch_joint", t: [0.1 * k, 0, 0] }) : -1;
  const spine = g.node({
    name: "spine_01",
    t: [0, 0.3 * k, 0],
    children: [head, hand, ...(extra >= 0 ? [extra] : [])],
  });
  const pelvis = g.node({ name: "pelvis", t: [0, 0.9 * k, 0], children: [spine] });
  const root = g.node({ name: "root", t: [0, lift * k, 0], children: [pelvis] });
  const scale = 0.5;
  const top = o.top ?? 1.7;
  const armature = g.node({
    name: "rig",
    s: o.stretch ? [scale, scale * 2, scale] : [scale, scale, scale],
    children: [root],
  });
  const jointList = [root, pelvis, spine, head, hand, leaf, ...(extra >= 0 ? [extra] : [])];
  const skin = g.skin(jointList);
  const slot = (node: number) => jointList.indexOf(node);
  // Body: a 0.4 × 1.7 × 0.3 m box from the feet to the head top, in bind space.
  const influence = (p: Vec3): [number[], number[]] => {
    const y = p[1] - lift;
    const joint = y < 0.9 ? pelvis : y < 1.5 ? (o.extraJoint && p[0] > 0 ? extra : spine) : head;
    return [
      [slot(joint), 0, 0, 0],
      [1, 0, 0, 0],
    ];
  };
  const bodyNodes: number[] = [];
  const lods = o.noLods ? [""] : ["_LOD0", "_LOD1", "_LOD2", "_LOD3"];
  lods.forEach((suffix, tier) => {
    const extraAttrs: GltfJson = {};
    if (o.fiveWeights) {
      extraAttrs.JOINTS_1 = g.accessor(new Uint16Array(24 * 4), "VEC4");
      extraAttrs.WEIGHTS_1 = g.accessor(new Float32Array(24 * 4), "VEC4");
    }
    const mesh = g.box([-0.2, lift, -0.15], [0.2, top + lift, 0.15], influence, extraAttrs);
    if (o.noNormals) delete g.json.meshes[mesh].primitives[0].attributes.NORMAL;
    bodyNodes.push(g.node({ name: `body${suffix}`, mesh, skin }));
    if (tier === 0 && !o.noLods) {
      // LOD0 carries a pack the coarser tiers drop.
      const pack = g.box([-0.15, 1.0 + lift, -0.3], [0.15, 1.4 + lift, -0.15], () => [
        [slot(spine), 0, 0, 0],
        [1, 0, 0, 0],
      ]);
      bodyNodes.push(g.node({ name: `pack${suffix}`, mesh: pack, skin }));
    }
  });
  const loose = o.looseMesh
    ? [g.node({ name: "loose_crate", mesh: g.box([1, 0, 1], [1.2, 0.2, 1.2]) })]
    : [];
  g.roots(armature, ...bodyNodes, ...loose);
  if (o.animated !== false) addSoldierClips(g, { root, pelvis, spine }, o.clips ?? SOLDIER_CLIPS);
  return g.glb();
}

function addSoldierClips(
  g: GltfBuilder,
  j: { root: number; pelvis: number; spine: number },
  names: string[],
) {
  const sway = (deg: number) => [...qx(0), ...qx(deg), ...qx(0)];
  for (const name of names) {
    if (name === "death")
      // Falls backwards: the pelvis pitches 90° about the character's left-right axis.
      g.animation(name, [
        { node: j.pelvis, path: "rotation", times: [0, 1], values: [...qx(0), ...qx(-90)] },
        { node: j.pelvis, path: "translation", times: [0, 1], values: [0, 1.8, 0, 0, 0.3, 0] },
      ]);
    else if (name === "stand_aim")
      g.animation(name, [{ node: j.spine, path: "rotation", times: [0], values: [...qx(0)] }]);
    else
      g.animation(name, [
        {
          node: j.spine,
          path: "rotation",
          times: [0, 0.5, 1],
          values: sway(name === "run" ? 10 : 4),
        },
      ]);
  }
}

export const SKELETON_ENTRY: SkeletonEntry = {
  source: "assets/source/test-rig.glb",
  basis_yaw_deg: 90,
  sample_hz: 30,
  aim_reference: { clip: "stand_aim", phase: 0 },
  clips: Object.fromEntries(
    SOLDIER_CLIPS.map((name) => [name, { loop: !["death", "stand_aim"].includes(name) }]),
  ),
};

// ---------------------------------------------------------------- vehicles (engine space)

/** Engine space (Z up, +X forward) → glTF (Y up): (x, y, z) → (x, z, −y). */
const g3 = ([x, y, z]: Vec3): Vec3 => [x, z, -y];
const gBox = (b: GltfBuilder, min: Vec3, max: Vec3) => {
  const [a, c] = [g3(min), g3(max)];
  return b.box(
    [Math.min(a[0], c[0]), Math.min(a[1], c[1]), Math.min(a[2], c[2])],
    [Math.max(a[0], c[0]), Math.max(a[1], c[1]), Math.max(a[2], c[2])],
  );
};

export interface TankOptions {
  muzzleX?: number; // world x of the muzzle (the realistic gun is 5.9)
  turretX?: number; // turret pivot off the hull origin (breaks the arc)
  hullHalfY?: number;
  omit?: string;
  muzzleUnderTurret?: boolean;
  duplicateWheel?: boolean;
  noTrackProperties?: boolean;
  lods?: "ok" | "none" | "inverted";
  twoRoots?: boolean;
  skinned?: boolean;
  lift?: number;
  flip?: boolean; // upside down (breaks up)
}

/**
 * A tank in engine space: hull 7 × 3.6 m to z 1.6, turret to z 2.4 pivoting
 * on the hull origin, gun trunnion (1, 0, 2), muzzle at the rule's (3, 0, 2).
 */
export function tankGlb(o: TankOptions = {}): Uint8Array {
  const b = new GltfBuilder();
  const muzzleX = o.muzzleX ?? 3;
  const turretX = o.turretX ?? 0;
  const halfY = o.hullHalfY ?? 1.8;
  const lift = o.lift ?? 0;
  const skip = (name: string) => o.omit === name;
  const empty = (
    name: string,
    t: Vec3,
    children: number[] = [],
    extras?: Record<string, unknown>,
  ) =>
    skip(name) ? -1 : b.node({ name, t: g3(t), children: children.filter((c) => c >= 0), extras });
  const part = (name: string, min: Vec3, max: Vec3) => b.node({ name, mesh: gBox(b, min, max) });

  const muzzle = empty("muzzle", [muzzleX - 1, 0, 0]);
  const barrel = part("barrel", [0, -0.08, -0.08], [muzzleX - 1, 0.08, 0.08]);
  const gun = empty("gun", [1 - turretX, 0, 0.4], [barrel, o.muzzleUnderTurret ? -1 : muzzle]);
  const hmgMuzzle = empty("hmg_muzzle", [0.8, 0, 0]);
  const hmgGun = empty(
    "hmg_gun",
    [0.1, 0, 0.1],
    [part("hmg_barrel", [0, -0.03, -0.03], [0.8, 0.03, 0.03]), hmgMuzzle],
  );
  const hmg = empty("hmg", [-0.3 - turretX, 0.6, 0.7], [hmgGun]);
  const turretShell = part("turret_shell", [-1.5, -1.2, 0], [1.5, 1.2, 0.8]);
  const turretMuzzle = o.muzzleUnderTurret
    ? b.node({ name: "muzzle", t: g3([muzzleX - turretX, 0, 0.4]) })
    : -1;
  const turret = empty("turret", [turretX, 0, 1.6], [turretShell, gun, hmg, turretMuzzle]);
  const trackExtras = o.noTrackProperties
    ? undefined
    : { track_length_m: 14.2, link_pitch_m: 0.16 };
  const track = (side: string, y: number) =>
    empty(
      `track_${side}`,
      [0, y, 0],
      [part(`track_${side}_band`, [-3.4, -0.3, 0], [3.4, 0.3, 0.9])],
      trackExtras,
    );
  const wheel = (name: string, x: number, y: number) =>
    empty(name, [x, y, 0.4], [part(`${name}_disc`, [-0.35, -0.15, -0.35], [0.35, 0.15, 0.35])]);
  const wheels = [
    wheel("wheel_L_1", 2, 1.4),
    wheel("wheel_R_1", 2, -1.4),
    wheel(o.duplicateWheel ? "wheel_L_1" : "wheel_L_2", -2, 1.4),
  ];
  const hullParts: number[] = [];
  const lodNames = o.lods === "none" ? [""] : ["_LOD0", "_LOD1", "_LOD2", "_LOD3"];
  lodNames.forEach((suffix, tier) => {
    const detailed = o.lods === "inverted" ? tier === 1 : tier === 0;
    hullParts.push(part(`hull_body${suffix}`, [-3.5, -halfY, 0.2], [3.5, halfY, 1.6]));
    if (detailed && suffix)
      hullParts.push(part(`hull_stowage${suffix}`, [-3.2, -0.5, 1.6], [-2.6, 0.5, 1.8]));
  });
  const hull = empty(
    "hull",
    [0, 0, lift],
    [...hullParts, turret, track("L", 1.5), track("R", -1.5), ...wheels],
  );
  const root = b.node({ name: "tank", children: [hull], r: o.flip ? qx(180) : undefined });
  if (o.skinned) b.skin([hull]);
  b.roots(root, ...(o.twoRoots ? [b.node({ name: "stray" })] : []));
  return b.glb();
}

export interface TruckOptions {
  omit?: string;
  reversed?: boolean;
  /** Author no deploy windows (breaks the deploy motion rule). */
  noDeployMotion?: boolean;
  /** Jack travel in metres; 0.1 puts the pads on the ground. */
  jackDrop?: number;
}

/**
 * A 6 × 2.8 × 3.6 m supply truck with four deploy legs and a three-stage
 * mast. Deploying (custom properties): the beams slide out 0.5 m over
 * progress 0–0.3, the jacks drop the pads to the ground over 0.2–0.5, the mast
 * swings up from the roof over 0.35–0.65 and telescopes over 0.6–1.
 */
export function truckGlb(o: TruckOptions = {}): Uint8Array {
  const b = new GltfBuilder();
  const skip = (name: string) => o.omit === name;
  const motion = (extras: Record<string, number>) => (o.noDeployMotion ? undefined : extras);
  const empty = (name: string, t: Vec3, children: number[] = [], extras?: Record<string, number>) =>
    skip(name) ? -1 : b.node({ name, t: g3(t), children: children.filter((c) => c >= 0), extras });
  const part = (name: string, min: Vec3, max: Vec3) => b.node({ name, mesh: gBox(b, min, max) });
  const legs = ["FL", "FR", "RL", "RR"].map((id) => {
    const x = id[0] === "F" ? 1.5 : -1.5;
    const y = id[1] === "L" ? 1.2 : -1.2;
    const pad = empty(
      `deploy_leg_${id}_pad`,
      [0, 0, -0.5],
      [part(`leg_${id}_pad`, [-0.15, -0.15, 0], [0.15, 0.15, 0.05])],
    );
    const jack = empty(
      `deploy_leg_${id}_jack`,
      [0, 0, 0],
      [pad],
      motion({ deploy_start: 0.2, deploy_end: 0.5, deploy_move_z: -(o.jackDrop ?? 0.1) }),
    );
    return empty(
      `deploy_leg_${id}`,
      [x, y, 0.6],
      [jack],
      motion({ deploy_start: 0, deploy_end: 0.3, deploy_move_y: Math.sign(y) * 0.5 }),
    );
  });
  const telescope = motion({ deploy_start: 0.6, deploy_end: 1, deploy_move_x: 0.6 });
  const head = empty("deploy_mast_head", [0.8, 0, 0]);
  const m3 = empty(
    "deploy_mast_3",
    [0.8, 0, 0],
    [part("mast_stage_3", [0, -0.05, 0], [0.8, 0.05, 0.1]), head],
    telescope,
  );
  const m2 = empty(
    "deploy_mast_2",
    [0.8, 0, 0],
    [part("mast_stage_2", [0, -0.07, 0], [0.8, 0.07, 0.14]), m3],
    telescope,
  );
  // Stowed: the mast lies along the roof, and swings up about its hinge.
  const mast = empty(
    "deploy_mast",
    [-2.5, 0, 3.4],
    [part("mast_tube", [-0.1, -0.1, 0], [0.8, 0.1, 0.2]), m2],
    motion({ deploy_start: 0.35, deploy_end: 0.65, deploy_turn_y: -90 }),
  );
  const sign = o.reversed ? -1 : 1;
  const wheels = ["FL", "FR", "RL", "RR"].map((id) =>
    empty(
      `wheel_${id}`,
      [sign * (id[0] === "F" ? 2 : -2), id[1] === "L" ? 1.2 : -1.2, 0.55],
      [part(`wheel_${id}_tyre`, [-0.55, -0.2, -0.55], [0.55, 0.2, 0.55])],
    ),
  );
  const bodyParts = ["_LOD0", "_LOD1", "_LOD2", "_LOD3"].map((suffix) =>
    part(`shelter${suffix}`, [-3, -1.4, 0.5], [3, 1.4, 3.4]),
  );
  const body = empty("body", [0, 0, 0], [...bodyParts, ...legs, mast, ...wheels]);
  b.roots(b.node({ name: "truck", children: [body] }));
  return b.glb();
}

/** A building state: a 10 × 8 × 6 m block (the ruin is 2 m tall). */
export function buildingGlb(height = 6, lift = 0): Uint8Array {
  const b = new GltfBuilder();
  const parts = ["_LOD0", "_LOD1", "_LOD2", "_LOD3"].map((suffix) =>
    b.node({ name: `walls${suffix}`, mesh: gBox(b, [-5, -4, lift], [5, 4, height + lift]) }),
  );
  b.roots(b.node({ name: "building", children: parts }));
  return b.glb();
}

export function testCatalog(): Catalog {
  return {
    tolerances: TOLERANCES,
    skeletons: { "test-rig": SKELETON_ENTRY },
    appearances: {
      rifleman: {
        unit: "rifle",
        source: "assets/source/test-rifleman.glb",
        basis_yaw_deg: 90,
        skeleton: "test-rig",
      },
      tank: { unit: "tank", source: "assets/source/test-tank.glb", basis_yaw_deg: 0 },
      truck: { unit: "supply", source: "assets/source/test-truck.glb", basis_yaw_deg: 0 },
      house: {
        unit: "building",
        states: {
          intact: "assets/source/test-house.glb",
          ruin: "assets/source/test-house-ruin.glb",
        },
        basis_yaw_deg: 0,
      },
    },
  };
}

export function testSources(): Record<string, Uint8Array> {
  return {
    "assets/source/test-rig.glb": soldierGlb(),
    "assets/source/test-rifleman.glb": soldierGlb({ animated: false }),
    "assets/source/test-tank.glb": tankGlb(),
    "assets/source/test-truck.glb": truckGlb(),
    "assets/source/test-house.glb": buildingGlb(6),
    "assets/source/test-house-ruin.glb": buildingGlb(2),
  };
}
