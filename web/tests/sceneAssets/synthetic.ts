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
  GrassSpec,
  SkeletonEntry,
  Tolerances,
} from "@packages/scene-assets/src/schema.ts";
import {
  UnitCatalog,
  type MountRow,
  type MountDraws,
  type UnitType,
} from "@packages/scene-assets/src/units.ts";

/** A turret mount row as the catalog view carries it. */
const turretMount = (name: string, on: string | null, pivot: Vec3, muzzle: Vec3): MountRow => ({
  name,
  weapons: [name],
  squad: false,
  special: false,
  turret: true,
  on,
  pivot_m: pivot,
  muzzle_m: muzzle,
});

/** The synthetic tank's mounts: the cannon on the turret axis (the turret
 *  node at z 1.6, the muzzle `reach` ahead at z 2), the HMG on its roof ring. */
export const tankMounts = (reach = 3): MountRow[] => [
  turretMount("cannon", null, [0, 0, 1.6], [reach, 0, 0.4]),
  turretMount("HMG", "cannon", [-0.3, 0.6, 2.3], [0.9, 0, 0.1]),
];

/** The rigs the synthetic tank model draws its mounts with. */
export const TANK_DRAWS: MountDraws = { cannon: "gun", HMG: "hmg" };

const armor = { front: 1, side: 1, rear: 1, roof: 1 };
/** A hull type, drawn by appearance `appearance`. */
const hullType = (
  id: string,
  appearance: string,
  half: Vec3,
  mobility: UnitType["mobility"],
  rest: Partial<UnitType> = {},
): UnitType => ({
  id,
  name: id,
  description: "",
  faction: "test",
  family: "vehicles",
  roles: ["test"],
  cost: 1,
  body: {
    hull: {
      half_extents_m: half,
      eye_m: 2,
      hp: 1,
      armor: { ...armor, ricochet: armor },
      weight_class: "heavy",
      push_class: "heavy",
      wreck: "heavy_wreck",
    },
  },
  mobility,
  sensors: { ground_m: 1, sight_shape: { front: 1, side: 1, rear: 1 } },
  mounts: [],
  capabilities: {},
  sound: { profile: "vehicle", loudness_m: 1 },
  appearance,
  ...rest,
});

/** The synthetic unit catalog the test art is fitted to: a tank (drawn by
 *  "tank"), a supply truck ("truck") and a rifle squad of riflemen
 *  ("rifleman"), and a part `era` whose hardware is `era_*` nodes. `tank`
 *  overrides the tank type's fields. */
export function syntheticUnits(tank: Partial<UnitType> = {}): UnitCatalog {
  const tracked = { tracked: { offroad_kmh: 1, road_kmh: 1, turn_deg_s: 1, reverse_fraction: 1 } };
  const wheeled = {
    wheeled: {
      offroad_kmh: 1,
      road_kmh: 1,
      turn_deg_s: 1,
      turning_radius_m: 1,
      reverse_fraction: 1,
    },
  };
  return new UnitCatalog({
    documents: [],
    props: {},
    roles: {},
    parts: {
      era: { name: "Reactive armour", description: "", nodes: ["era_*"] },
    },
    soldiers: {
      rifleman: { name: "Rifleman", description: "", hp: 1, appearance: ["rifleman"], mounts: [] },
    },
    units: [
      hullType("tank", "tank", [3.5, 1.8, 1.2], tracked, { mounts: tankMounts(), ...tank }),
      hullType("supply", "truck", [3, 1.4, 1.8], wheeled, {
        capabilities: { deploy: { seconds: 1 } },
      }),
      {
        ...hullType("rifle", "", [0, 0, 0], { foot: { offroad_kmh: 4, road_kmh: 4 } }),
        body: { squad: { slots: ["rifleman", "rifleman"] } },
        appearance: undefined,
      },
    ],
  });
}

export const AUTHORITY: Authority = {
  soldier_height_m: 1.7,
  infantry_eye_m: 1.6,
  infantry_muzzle_m: 1.4,
  units: syntheticUnits(),
  canopy_height_m: 12,
  canopy_radius_m: 6.5,
  ruin_height_m: 2,
  collapse: { min_height_m: 2, height_fraction: 0.25, max_height_m: 6, max_floors: 6 },
};

export const TOLERANCES: Tolerances = {
  ground_m: 0.01,
  soldier_height_m: 0.05,
  eye_m: 0.08,
  muzzle_m: 0.1,
  hull_extent_m: 0.1,
  hull_top_m: 0.1,
  footprint_m: 0.1,
  vehicle_muzzle_m: 0.05,
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
  /** Boxes carry UVs (one per face, metres) and, unless false, tangents. */
  surface: { tangents: boolean } | null = null;

  private view(bytes: Uint8Array): number {
    const padded = new Uint8Array(Math.ceil(bytes.byteLength / 4) * 4);
    padded.set(bytes);
    this.json.bufferViews.push({
      buffer: 0,
      byteOffset: this.length,
      byteLength: bytes.byteLength,
    });
    this.chunks.push(padded);
    this.length += padded.byteLength;
    return this.json.bufferViews.length - 1;
  }

  /** Embed a square RGBA8 image as a PNG; returns its glTF texture index. */
  texture(size: number, pixel: (x: number, y: number) => [number, number, number, number]): number {
    const rgba = new Uint8Array(size * size * 4);
    for (let y = 0; y < size; y++)
      for (let x = 0; x < size; x++) rgba.set(pixel(x, y), (y * size + x) * 4);
    const bufferView = this.view(encodePng(size, size, rgba));
    this.json.images = [...(this.json.images ?? []), { bufferView, mimeType: "image/png" }];
    this.json.textures = [...(this.json.textures ?? []), { source: this.json.images.length - 1 }];
    return this.json.textures.length - 1;
  }

  /** Give material 0 an albedo, a flat normal map and an ORM image, each `size` px. */
  textureMaterial(size: number) {
    const albedo = this.texture(size, (x, y) => [(x * 37) & 255, (y * 91) & 255, 90, 200]);
    const normal = this.texture(size, () => [128, 128, 255, 255]);
    const orm = this.texture(size, (x) => [255, 180 + (x & 7), 0, 255]);
    const m = this.json.materials[0];
    m.pbrMetallicRoughness.baseColorTexture = { index: albedo };
    m.pbrMetallicRoughness.metallicRoughnessTexture = { index: orm };
    m.occlusionTexture = { index: orm };
    m.normalTexture = { index: normal };
  }

  accessor(
    data: Float32Array | Uint16Array | Uint32Array,
    type: string,
    extra: GltfJson = {},
  ): number {
    const view = this.view(new Uint8Array(data.buffer, data.byteOffset, data.byteLength));
    const componentType =
      data instanceof Float32Array ? 5126 : data instanceof Uint16Array ? 5123 : 5125;
    const width = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 }[type]!;
    this.json.accessors.push({
      bufferView: view,
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
    const uvs: number[] = [];
    const tangents: number[] = [];
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
        uvs.push(p[u], p[v]);
        const t = [0, 0, 0, sign];
        t[u] = 1;
        tangents.push(...t);
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
    if (this.surface) {
      attrs.TEXCOORD_0 = this.accessor(Float32Array.from(uvs), "VEC2");
      if (this.surface.tangents) attrs.TANGENT = this.accessor(Float32Array.from(tangents), "VEC4");
    }
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
  tint?: number; // the side-tint mask weight of its material (extras.tint)
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
  if (o.tint !== undefined) g.json.materials[0].extras = { tint: o.tint };
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
export const gBox = (b: GltfBuilder, min: Vec3, max: Vec3) => {
  const [a, c] = [g3(min), g3(max)];
  return b.box(
    [Math.min(a[0], c[0]), Math.min(a[1], c[1]), Math.min(a[2], c[2])],
    [Math.max(a[0], c[0]), Math.max(a[1], c[1]), Math.max(a[2], c[2])],
  );
};

export interface TankOptions {
  muzzleX?: number; // world x of the muzzle (the realistic gun is 5.9)
  /** Where the model draws its cannon and roof HMG: `tankMounts(muzzleX)`
   *  unless given (a unit type's mount rows, cannon then HMG on it). */
  mounts?: MountRow[];
  /** Draw reactive armour tiles, `era_L` and `era_R`, on the hull sides. */
  era?: boolean;
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
  /** Top of a whip antenna on the turret roof, world z (the hit box's top is 2.4). */
  antenna?: number;
  /** Texture the paint (albedo, normal, ORM) at this size, with UVs and, unless
   *  `tangents` is false, tangents. */
  textures?: { size: number; tangents?: boolean };
}

/**
 * A tank in engine space: hull 7 × 3.6 m to z 1.6, turret to z 2.4 pivoting
 * on the hull origin, gun trunnion 1 m ahead of it; the cannon and HMG nodes
 * sit where the mount rows put them (by default the muzzle at (3, 0, 2)).
 */
export function tankGlb(o: TankOptions = {}): Uint8Array {
  const b = new GltfBuilder();
  if (o.textures) {
    b.surface = { tangents: o.textures.tangents !== false };
    b.textureMaterial(o.textures.size);
  }
  const [cannon, roofGun] = o.mounts ?? tankMounts(o.muzzleX ?? 3);
  const [cannonX, , turretZ] = cannon.pivot_m;
  const [reach, , gunZ] = cannon.muzzle_m!;
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

  const muzzle = empty("muzzle", [reach - 1, 0, 0]);
  const barrel = part("barrel", [0, -0.08, -0.08], [reach - 1, 0.08, 0.08]);
  const gun = empty("gun", [1 - turretX, 0, gunZ], [barrel, o.muzzleUnderTurret ? -1 : muzzle]);
  // The HMG's ring on the turret roof, its gun 0.1 m up and forward.
  const [hx, hy, hz] = roofGun.muzzle_m!;
  const hmgMuzzle = empty("hmg_muzzle", [hx - 0.1, hy, hz - 0.1]);
  const hmgGun = empty(
    "hmg_gun",
    [0.1, 0, 0.1],
    [part("hmg_barrel", [0, -0.03, -0.03], [hx - 0.1, 0.03, 0.03]), hmgMuzzle],
  );
  const [px, py, pz] = roofGun.pivot_m;
  const hmg = empty("hmg", [px - cannonX - turretX, py, pz - turretZ], [hmgGun]);
  // The turret shell reaches the hull box's top, z 2.4.
  const turretShell = part("turret_shell", [-1.5, -1.2, 0], [1.5, 1.2, 2.4 - turretZ]);
  const turretMuzzle = o.muzzleUnderTurret
    ? b.node({ name: "muzzle", t: g3([reach - turretX, 0, gunZ]) })
    : -1;
  const antenna =
    o.antenna !== undefined
      ? part("antenna", [-1.2, 0.8, 0.8], [-1.19, 0.81, o.antenna - turretZ])
      : -1;
  const tile = (side: string, y: number) =>
    empty(`era_${side}`, [0, y, 0.6], [part(`era_${side}_tiles`, [-2, -0.05, 0], [2, 0.05, 0.8])]);
  const era = o.era ? [tile("L", 1.75), tile("R", -1.75)] : [];
  const turret = empty(
    "turret",
    [cannonX + turretX, 0, turretZ],
    [turretShell, gun, hmg, turretMuzzle, antenna].filter((c) => c >= 0),
  );
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
    [...hullParts, ...era, turret, track("L", 1.5), track("R", -1.5), ...wheels],
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

/** A tree: a trunk and a crown whose top stands `height` metres up and
 *  reaches `radius` from the trunk's axis, four tiers. The trunk is a square
 *  post with the cross-section of a round bole of radius `bole`, standing at
 *  `boleAt`. `crowns[t]`
 *  draws tier t's crown as that many boxes (12 triangles each) instead of
 *  one. */
export function treeGlb(
  height = 11,
  lift = 0,
  o: { radius?: number; crowns?: readonly number[]; bole?: number; boleAt?: [number, number] } = {},
): Uint8Array {
  const b = new GltfBuilder();
  const r = o.radius ?? 4;
  const half = ((o.bole ?? 0.4) * Math.sqrt(Math.PI)) / 2;
  const [x, y] = o.boleAt ?? [0, 0];
  const parts = ["_LOD0", "_LOD1", "_LOD2", "_LOD3"].flatMap((suffix, t) => [
    b.node({
      name: `trunk${suffix}`,
      mesh: gBox(b, [x - half, y - half, lift], [x + half, y + half, 4 + lift]),
    }),
    ...Array.from({ length: o.crowns?.[t] ?? 1 }, (_, i) =>
      b.node({
        name: `crown${i}${suffix}`,
        mesh: gBox(b, [-r, -r, 3 + lift], [r, r, height + lift]),
      }),
    ),
  ]);
  b.roots(b.node({ name: "tree", children: parts }));
  return b.glb();
}

/**
 * A 2 m panel with UVs and tangents, four tiers, whose one material `edit`
 * changes before export (its alpha mode, its extras, textures of its own
 * from `b.texture`): the surface the material-transport tests are about.
 */
export function panelGlb(edit: (material: GltfJson, b: GltfBuilder) => void = () => {}) {
  const b = new GltfBuilder();
  b.surface = { tangents: true };
  edit(b.json.materials[0], b);
  const parts = ["_LOD0", "_LOD1", "_LOD2", "_LOD3"].map((suffix) =>
    b.node({ name: `panel${suffix}`, mesh: gBox(b, [-1, -0.1, 0], [1, 0.1, 2]) }),
  );
  b.roots(b.node({ name: "panel", children: parts }));
  return b.glb();
}

/** A GLB with its JSON chunk edited: one field of an otherwise valid source. */
export function withJson(bytes: Uint8Array, edit: (json: GltfJson) => void): Uint8Array {
  const dv = new DataView(bytes.buffer, bytes.byteOffset);
  const length = dv.getUint32(12, true);
  const json = JSON.parse(new TextDecoder().decode(bytes.subarray(20, 20 + length)));
  edit(json);
  const text = new TextEncoder().encode(JSON.stringify(json));
  const padded = Math.ceil(text.length / 4) * 4;
  const binChunk = bytes.subarray(20 + length);
  const out = new Uint8Array(20 + padded + binChunk.length);
  out.set(bytes.subarray(0, 12));
  const odv = new DataView(out.buffer);
  odv.setUint32(8, out.length, true);
  odv.setUint32(12, padded, true);
  odv.setUint32(16, 0x4e4f534a, true);
  out.set(text, 20);
  out.fill(0x20, 20 + text.length, 20 + padded);
  out.set(binChunk, 20 + padded);
  return out;
}

/** A meadow tuft for the grass generator. */
export const GRASS_SPEC: GrassSpec = {
  seed: 7,
  blades: 5,
  radius_m: 0.1,
  height_m: [0.3, 0.5],
  width_m: 0.03,
  lean: [0.1, 0.4],
  colors: { root: [0.1, 0.16, 0.05], mid: [0.25, 0.33, 0.14], tip: [0.45, 0.46, 0.25] },
  jitter: 0.1,
  wind: 1,
};

export function testCatalog(): Catalog {
  return {
    tolerances: TOLERANCES,
    sides: { blue: [1, 1, 1], red: [1.2, 0.9, 0.7] },
    skeletons: { "test-rig": SKELETON_ENTRY },
    appearances: {
      rifleman: {
        unit: "soldier",
        source: "assets/source/test-rifleman.glb",
        basis_yaw_deg: 90,
        skeleton: "test-rig",
      },
      tank: {
        unit: "vehicle",
        source: "assets/source/test-tank.glb",
        basis_yaw_deg: 0,
        mounts: TANK_DRAWS,
      },
      truck: { unit: "vehicle", source: "assets/source/test-truck.glb", basis_yaw_deg: 0 },
      house: {
        unit: "building",
        states: {
          intact: "assets/source/test-house.glb",
          ruin: "assets/source/test-house-ruin.glb",
        },
        basis_yaw_deg: 0,
        footprint_half_m: [5, 4, 3],
      },
    },
  };
}

export function testSources(): Record<string, Uint8Array> {
  return {
    "assets/source/test-rig.glb": soldierGlb(),
    "assets/source/test-rifleman.glb": soldierGlb({ animated: false, tint: 1 }),
    "assets/source/test-tank.glb": tankGlb(),
    "assets/source/test-truck.glb": truckGlb(),
    "assets/source/test-house.glb": buildingGlb(6),
    "assets/source/test-house-ruin.glb": buildingGlb(2),
  };
}

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (const b of bytes) c = CRC_TABLE[(c ^ b) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/**
 * An RGBA8 PNG with stored (uncompressed) deflate blocks: synchronous and
 * isomorphic, so the builders run in the browser scenes as well as vitest.
 */
export function encodePng(width: number, height: number, rgba: Uint8Array): Uint8Array {
  const row = width * 4;
  const raw = new Uint8Array(height * (row + 1));
  for (let y = 0; y < height; y++)
    raw.set(rgba.subarray(y * row, (y + 1) * row), y * (row + 1) + 1);
  const blocks = Math.max(1, Math.ceil(raw.length / 65535));
  const zlib = new Uint8Array(2 + raw.length + blocks * 5 + 4);
  const z = new DataView(zlib.buffer);
  zlib.set([0x78, 0x01]);
  let at = 2;
  for (let i = 0; i < blocks; i++) {
    const part = raw.subarray(i * 65535, (i + 1) * 65535);
    zlib[at] = i === blocks - 1 ? 1 : 0;
    z.setUint16(at + 1, part.length, true);
    z.setUint16(at + 3, ~part.length & 0xffff, true);
    zlib.set(part, at + 5);
    at += 5 + part.length;
  }
  let a = 1;
  let b = 0;
  for (const v of raw) {
    a = (a + v) % 65521;
    b = (b + a) % 65521;
  }
  z.setUint32(at, ((b << 16) | a) >>> 0);
  const chunk = (type: string, data: Uint8Array) => {
    const out = new Uint8Array(12 + data.length);
    const v = new DataView(out.buffer);
    v.setUint32(0, data.length);
    out.set(
      [...type].map((ch) => ch.charCodeAt(0)),
      4,
    );
    out.set(data, 8);
    v.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));
    return out;
  };
  const ihdr = new Uint8Array(13);
  const h = new DataView(ihdr.buffer);
  h.setUint32(0, width);
  h.setUint32(4, height);
  ihdr.set([8, 6, 0, 0, 0], 8);
  const parts = [
    new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", zlib),
    chunk("IEND", new Uint8Array()),
  ];
  const png = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let o = 0;
  for (const p of parts) {
    png.set(p, o);
    o += p.length;
  }
  return png;
}
