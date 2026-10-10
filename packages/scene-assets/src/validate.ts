// The appearance validator. Isomorphic: it takes bytes and data, never paths
// to read, so the CLI, the bake and tests share it. `validateAppearance` and
// `validateSkeleton` import, build and check one catalog entry; they return the
// findings, stats and — when nothing is an error — the bundle to encode.

import {
  buildArticulated,
  buildClips,
  buildKitModules,
  buildSkinned,
  buildStaticState,
  MaterialTable,
  triangleCount,
} from "./build.ts";
import { quat, vec3, type Vec3 } from "math";
import { pointAt } from "./trs.ts";
import { encodeBundle } from "./codec.ts";
import { mountMuzzles, muzzleOffset, type MountMuzzle } from "./mountMuzzle.ts";
import {
  FACTIONS,
  MOUNT_NODES,
  hullFixed,
  isArticulation,
  mountRoles,
  vehicleClass,
  type Articulation,
  type MountDraws,
  type MountRow,
  type UnitCatalog,
  type UnitType,
} from "./units.ts";
import { DEBRIS_NODE, SCENERY_KINDS, type DebrisAllowance, type SceneryRule } from "./scenery.ts";
import {
  DEPLOY_EXTRAS,
  REST_ARTICULATION,
  articulate,
  articulationRig,
  isRotor,
  restLocals,
} from "./articulation.ts";
import {
  articulatedPositions,
  articulatedWorlds,
  positionsBounds,
  posedBounds,
  sampleClip,
  skinPositions,
  worldTransforms,
} from "./pose.ts";
import { bindTextures, importScene, type Scene } from "./scene.ts";
import { SOLDIER_CLASS, tierFindings, unitArtRule, type DressingAllowance } from "./unitArt.ts";
import { materialFindings } from "./material.ts";
import { textureFindings } from "./texture.ts";
import { grassStripFindings } from "./grass.ts";
import {
  INFANTRY_CLIPS,
  INFANTRY_SOCKETS,
  UNIT_BUNDLE_KIND,
  type AppearanceEntry,
  type ArticulatedBundle,
  type ArticulatedNode,
  type Authority,
  type Bounds,
  type Budget,
  type Bundle,
  type Finding,
  type Joint,
  type Material,
  type MaterialRole,
  type MeshData,
  type PoseRef,
  type SkeletonClips,
  type SkeletonEntry,
  type SkinnedBundle,
  type Socket,
  type StaticBundle,
  type Tolerances,
} from "./schema.ts";

export interface ValidationContext {
  authority: Authority;
  tolerances: Tolerances;
}

export interface Stats {
  kind: Bundle["kind"];
  tiers: { triangles: number; vertices: number }[];
  joints?: number;
  nodes?: number;
  /** A kit: each module's triangles per tier, finest first. */
  modules?: { name: string; triangles: number[] }[];
  clips?: { name: string; loop: boolean; duration: number; frames: number }[];
  source_bytes: number;
  bounds?: Bounds;
}

export interface Validation<B extends Bundle> {
  findings: Finding[];
  stats: Stats | null;
  /** The bundle to encode; null while any finding is an error. */
  bundle: B | null;
  /** The bundle as built, errors or not, for the workbench to show; null
   *  only when the source could not be built at all. Never baked. */
  preview: B | null;
}

export const hasErrors = (findings: Finding[]) => findings.some((f) => f.severity === "error");

const finding = (code: Finding["code"], message: string, fix: string): Finding => ({
  code,
  severity: "error",
  message,
  fix,
});
const tierStats = (tiers: MeshData[]) =>
  tiers.map((m) => ({ triangles: triangleCount(m), vertices: m.positions.length / 3 }));
const fmt = (n: number) => n.toFixed(3);
/** What a built bundle's surfaces must be, whatever it draws: its textures
 *  and its materials. */
const surfaceFindings = (label: string, bundle: Exclude<Bundle, SkeletonClips>) => [
  ...textureFindings(label, bundle),
  ...materialFindings(label, bundle),
];

// ---------------------------------------------------------------- skeleton clips

export async function validateSkeleton(
  id: string,
  entry: SkeletonEntry,
  bytes: Uint8Array,
): Promise<Validation<SkeletonClips>> {
  const { scene, findings } = importScene(bytes, entry.source, entry.basis_yaw_deg);
  if (!scene) return { findings, stats: null, bundle: null, preview: null };
  const built = buildClips(scene, entry.source, id, entry.sample_hz, entry.clips);
  findings.push(...built.findings);
  if (!built.built) return { findings, stats: null, bundle: null, preview: null };
  findings.push(...clipRoleFindings(built.built, entry.aim_reference, entry.source));
  const clips = built.built;
  return {
    findings,
    stats: {
      kind: "clips",
      tiers: [],
      joints: clips.joints.length,
      clips: clips.clips.map((c) => ({
        name: c.name,
        loop: c.loop,
        duration: c.duration,
        frames: c.frames,
      })),
      source_bytes: bytes.byteLength,
    },
    bundle: hasErrors(findings) ? null : clips,
    preview: clips,
  };
}

function clipRoleFindings(clips: SkeletonClips, aim: PoseRef, label: string): Finding[] {
  const names = new Set(clips.clips.map((c) => c.name));
  return [...INFANTRY_CLIPS, aim.clip]
    .filter((name, i, all) => all.indexOf(name) === i && !names.has(name))
    .map((name) =>
      finding(
        "structure.clips",
        `${label}: skeleton ${clips.id} lacks clip "${name}"`,
        name === aim.clip
          ? "author the standing-aim reference pose infantry fit is measured in"
          : `author the "${name}" role on the skeleton`,
      ),
    );
}

// ---------------------------------------------------------------- appearances

export interface AppearanceInput {
  name: string;
  entry: AppearanceEntry;
  /** Source bytes by catalog path (the entry's `source` or `states`). */
  files: Record<string, Uint8Array>;
  /** Skinned: the skeleton's clips and its aim reference pose. */
  skeleton?: { clips: SkeletonClips; aim_reference: PoseRef };
  /** Articulated: the unit types to fit it to; by default every type whose
   *  `appearance` names it (`name`). */
  types?: string[];
}

export async function validateAppearance(
  input: AppearanceInput,
  context: ValidationContext,
): Promise<Validation<Bundle>> {
  const { entry } = input;
  if (entry.unit === "kit") return validateKit(input);
  const tolerances = { ...context.tolerances, ...entry.tolerances };
  const kind = UNIT_BUNDLE_KIND[entry.unit];
  const findings: Finding[] = [];
  const sources =
    kind === "static"
      ? Object.entries(entry.states ?? {})
      : [["source", entry.source ?? ""] as [string, string]];
  let sourceBytes = 0;
  for (const [, path] of sources) {
    const bytes = input.files[path];
    if (!bytes) throw new Error(`${input.name}: no bytes supplied for ${path}`);
    sourceBytes += bytes.byteLength;
  }

  if (kind === "static") {
    const materials = new MaterialTable();
    const states: { name: string; tiers: MeshData[]; bounds: Bounds }[] = [];
    const required = SCENERY_KINDS[entry.scenery ?? ""]?.states ?? null;
    if (!required)
      findings.push(
        finding(
          "structure.scenery_kind",
          `${input.name}: scenery kind "${entry.scenery ?? "(none)"}" is not one of ${Object.keys(SCENERY_KINDS).join(", ")}`,
          "name a scenery kind in the catalog entry, or add a row to SCENERY_KINDS (packages/scene-assets/src/scenery.ts)",
        ),
      );
    for (const state of required ?? [])
      if (!entry.states?.[state])
        findings.push(
          finding(
            "structure.states",
            `${input.name}: no "${state}" state`,
            `add states.${state} to the catalog entry`,
          ),
        );
    for (const [state, path] of sources) {
      const imported = importScene(input.files[path], path, entry.basis_yaw_deg);
      findings.push(...imported.findings);
      if (!imported.scene) continue;
      findings.push(...(await bindTextures(imported.scene, path)));
      const built = buildStaticState(imported.scene, path, materials);
      findings.push(...built.findings);
      if (!built.tiers) continue;
      const bounds = positionsBounds(built.tiers[0].positions);
      states.push({ name: state, tiers: built.tiers, bounds });
      const rule = SCENERY_KINDS[entry.scenery ?? ""];
      const debris = rule?.debris?.state === state ? rule.debris : null;
      // A wreck's tiers are a unit's: it is drawn where its vehicle was. It
      // takes the default row (no vehicle class sets a ratio of its own yet).
      // A piece or its debris is part of its whole's tiers, so only the whole
      // is held to the ratio: a thrown turret's far tier is a few boxes either way.
      if (entry.scenery === "wreck")
        findings.push(
          ...tierFindings(
            path,
            imported.scene,
            built.tiers.map(triangleCount),
            rule?.pieces?.includes(state) || debris ? 1 : unitArtRule(null).tier_ratio,
          ),
        );
      // Debris is held to its own allowance, not to the ground or the box.
      if (debris) {
        findings.push(
          ...debrisFindings(path, imported.scene, built.tiers[0], entry.footprint_half_m, debris),
        );
        continue;
      }
      // A piece is held to its whole (`pieceFindings`), not to the ground or the box.
      if (rule?.pieces?.includes(state)) continue;
      if (rule?.blades) findings.push(...grassStripFindings(path, built.tiers));
      findings.push(...groundFindings(`${path}`, bounds.min[2], tolerances));
      if (rule?.footprint.kind === "tree")
        findings.push(...canopyFindings(path, built.tiers, context.authority, tolerances));
      if (rule?.tier_triangles)
        findings.push(
          ...budgetFindings(
            path,
            built.tiers.map(triangleCount),
            rule,
            "the kind's row (packages/scene-assets/src/scenery.ts), with a paired frame-cost row,",
          ),
        );
      if (rule?.size) findings.push(...sizeFindings(path, built.tiers[0], rule.size));
      if (rule?.top_m !== undefined && bounds.max[2] > rule.top_m + tolerances.ground_m)
        findings.push(
          finding(
            "fit.dressing",
            `${path}: stands ${fmt(bounds.max[2])} m tall, over the ${rule.top_m} m a ${entry.scenery} may (it has no body to hide behind)`,
            "lower the art under the kind's top_m (packages/scene-assets/src/scenery.ts)",
          ),
        );
    }
    // The kind's first state leads (what an impostor and a sheet show, even
    // where a wreck's `debris` sorts before its `default`), the rest by name.
    const lead = required?.[0];
    states.sort(
      (a, b) => Number(b.name === lead) - Number(a.name === lead) || a.name.localeCompare(b.name),
    );
    const pieces = SCENERY_KINDS[entry.scenery ?? ""]?.pieces ?? [];
    const debrisState = SCENERY_KINDS[entry.scenery ?? ""]?.debris?.state;
    if (required) {
      const whole = states.filter((s) => !pieces.includes(s.name) && s.name !== debrisState);
      findings.push(...footprintFindings(entry, whole, tolerances, input.name));
      findings.push(...pieceFindings(input.name, states, required[0], pieces));
    }
    const bounds = states.reduce<Bounds | null>((b, s) => union(b, s.bounds), null);
    const bundle: StaticBundle | null = bounds
      ? {
          kind: "static",
          states,
          materials: materials.materials,
          textures: materials.textures,
          bounds,
        }
      : null;
    if (bundle) findings.push(...surfaceFindings(input.name, bundle));
    return {
      findings,
      stats: states.length
        ? {
            kind,
            tiers: sumTiers(states.map((s) => s.tiers)),
            source_bytes: sourceBytes,
            bounds: bounds ?? undefined,
          }
        : null,
      bundle: hasErrors(findings) ? null : bundle,
      preview: bundle,
    };
  }

  const path = entry.source ?? "";
  const imported = importScene(input.files[path], path, entry.basis_yaw_deg);
  findings.push(...imported.findings);
  if (!imported.scene) return { findings, stats: null, bundle: null, preview: null };
  findings.push(...(await bindTextures(imported.scene, path)));

  if (kind === "articulated") {
    const built = buildArticulated(imported.scene, path);
    findings.push(...built.findings);
    if (!built.built) return { findings, stats: null, bundle: null, preview: null };
    const { nodes, materials, textures } = built.built;
    const units = context.authority.units;
    const types = input.types ?? units.ids.filter((id) => units.type(id).appearance === input.name);
    findings.push(
      ...articulatedFindings(path, nodes, materials, tolerances),
      ...types.flatMap((id) =>
        typeFindings(path, nodes, units, id, tolerances, entry.mounts ?? null),
      ),
    );
    const bounds = posedBounds(nodes);
    const bundle: ArticulatedBundle = { kind: "articulated", nodes, materials, textures, bounds };
    const tiers = sumTiers(nodes.map((n) => n.tiers));
    findings.push(
      ...surfaceFindings(path, bundle),
      ...unitArtFindings(
        path,
        imported.scene,
        bundle,
        tiers.map((t) => t.triangles),
        // A vehicle no type draws has no class, and takes the default rules.
        types.length ? [...new Set(types.map((id) => vehicleClass(units.type(id))))] : [null],
      ),
    );
    return {
      findings,
      stats: {
        kind,
        tiers,
        nodes: nodes.length,
        source_bytes: sourceBytes,
        bounds,
      },
      bundle: hasErrors(findings) ? null : bundle,
      preview: bundle,
    };
  }

  const skeleton = input.skeleton;
  if (!skeleton) throw new Error(`${input.name}: a skinned appearance needs its skeleton's clips`);
  const built = buildSkinned(imported.scene, path, skeleton.clips.joints);
  findings.push(...built.findings);
  if (!built.built) return { findings, stats: null, bundle: null, preview: null };
  const { joints, tiers, materials, textures, sockets } = built.built;
  const farPose = entry.far_pose ?? { clip: "idle", phase: 0 };
  const corpsePose = entry.corpse_pose ?? { clip: "death", phase: 1 };
  for (const [what, pose] of [
    ["far_pose", farPose],
    ["corpse_pose", corpsePose],
  ] as const)
    if (!skeleton.clips.clips.some((c) => c.name === pose.clip))
      findings.push(
        finding(
          "structure.clips",
          `${path}: ${what} clip "${pose.clip}" is not on skeleton ${skeleton.clips.id}`,
          `name a clip of ${skeleton.clips.id} in ${what}`,
        ),
      );
  findings.push(
    ...infantryFindings(
      path,
      joints,
      tiers[0],
      sockets,
      skeleton.clips,
      skeleton.aim_reference,
      context.authority,
      tolerances,
      context.authority.units.view.units
        .flatMap((u) => u.mounts)
        .filter(
          (m) =>
            m.muzzle_m !== null &&
            m.operator_appearance?.active.includes(input.name) &&
            m.operator_appearance.active_pose !== undefined,
        ),
    ),
  );
  const bounds = animatedBounds(joints, tiers, skeleton.clips);
  const bundle: SkinnedBundle = {
    kind: "skinned",
    skeleton: skeleton.clips.id,
    joints,
    tiers,
    materials,
    textures,
    bounds,
    far_pose: farPose,
    corpse_pose: corpsePose,
    sockets,
  };
  findings.push(
    ...surfaceFindings(path, bundle),
    ...unitArtFindings(path, imported.scene, bundle, tiers.map(triangleCount), [SOLDIER_CLASS]),
  );
  return {
    findings,
    stats: {
      kind,
      tiers: tierStats(tiers),
      joints: joints.length,
      source_bytes: sourceBytes,
      bounds,
    },
    bundle: hasErrors(findings) ? null : bundle,
    preview: bundle,
  };
}

/**
 * A city kit: one GLB whose root-level empties are its modules. It bakes to a
 * static bundle whose states are the modules, each in its own frame with its
 * own bounds over every tier. A module is a piece of a building, placed many
 * times by the template art library's rows, so nothing here measures it
 * against the ground or a simulation box; how far a building's art may reach
 * past its physical parts is judged per template (`templateSource.ts`).
 */
async function validateKit(input: AppearanceInput): Promise<Validation<Bundle>> {
  const { entry, name } = input;
  const path = entry.source ?? "";
  const bytes = input.files[path];
  if (!bytes) throw new Error(`${name}: no bytes supplied for ${path}`);
  const findings: Finding[] = [];
  if (entry.footprint_half_m || entry.states)
    findings.push(
      finding(
        "kit.module",
        `${name}: a kit declares ${entry.footprint_half_m ? "footprint_half_m" : "states"}; its modules are parts of buildings, not a building`,
        "a kit entry is { unit, source, basis_yaw_deg }: its states are its modules, and fit is its templates'",
      ),
    );
  const imported = importScene(bytes, path, entry.basis_yaw_deg);
  findings.push(...imported.findings);
  if (!imported.scene) return { findings, stats: null, bundle: null, preview: null };
  findings.push(...(await bindTextures(imported.scene, path)));
  const materials = new MaterialTable();
  const built = buildKitModules(imported.scene, path, materials);
  findings.push(...built.findings);
  const states = built.modules.map((module) => ({
    name: module.name,
    tiers: module.tiers,
    bounds: module.tiers.reduce<Bounds>((b, mesh) => positionsBounds(mesh.positions, b), {
      min: [Infinity, Infinity, Infinity],
      max: [-Infinity, -Infinity, -Infinity],
    }),
  }));
  // A module with no geometry at all has no bounds, and is already an error.
  const drawn = states.filter((s) => s.bounds.min[0] <= s.bounds.max[0]);
  const bounds = drawn.reduce<Bounds | null>((b, s) => union(b, s.bounds), null);
  if (!bounds || drawn.length !== states.length)
    return { findings, stats: null, bundle: null, preview: null };
  const bundle: StaticBundle = {
    kind: "static",
    states,
    materials: materials.materials,
    textures: materials.textures,
    bounds,
  };
  findings.push(...surfaceFindings(name, bundle));
  return {
    findings,
    stats: {
      kind: "static",
      tiers: sumTiers(states.map((s) => s.tiers)),
      modules: states.map((s) => ({ name: s.name, triangles: s.tiers.map(triangleCount) })),
      source_bytes: bytes.byteLength,
      bounds,
    },
    bundle: hasErrors(findings) ? null : bundle,
    preview: bundle,
  };
}

const union = (a: Bounds | null, b: Bounds): Bounds =>
  a
    ? {
        min: a.min.map((v, i) => Math.min(v, b.min[i])) as Vec3,
        max: a.max.map((v, i) => Math.max(v, b.max[i])) as Vec3,
      }
    : b;

function sumTiers(groups: MeshData[][]): Stats["tiers"] {
  return groups[0].map((_, t) =>
    groups.reduce(
      (acc, tiers) => ({
        triangles: acc.triangles + triangleCount(tiers[t]),
        vertices: acc.vertices + tiers[t].positions.length / 3,
      }),
      { triangles: 0, vertices: 0 },
    ),
  );
}

function groundFindings(label: string, minZ: number, tolerances: Tolerances): Finding[] {
  return Math.abs(minZ) > tolerances.ground_m
    ? [
        finding(
          "basis.ground",
          `${label}: lowest point is z ${fmt(minZ)} m, not 0 ± ${tolerances.ground_m}`,
          "put the origin on the ground under the model (feet or tracks at z = 0)",
        ),
      ]
    : [];
}

/** Union of every tier's skinned bounds over every frame of every clip. */
export function animatedBounds(joints: Joint[], tiers: MeshData[], clips: SkeletonClips): Bounds {
  const parents = joints.map((j) => j.parent);
  const bounds: Bounds = {
    min: [Infinity, Infinity, Infinity],
    max: [-Infinity, -Infinity, -Infinity],
  };
  const poses = [
    worldTransforms(
      parents,
      joints.map((j) => j.bind),
    ),
  ];
  for (const clip of clips.clips)
    for (let f = 0; f < clip.frames; f++)
      poses.push(
        worldTransforms(
          parents,
          sampleClip(clips, clip, joints, clip.frames > 1 ? f / (clip.frames - 1) : 0),
        ),
      );
  for (const worlds of poses)
    for (const mesh of tiers) positionsBounds(skinPositions(mesh, joints, worlds), bounds);
  return bounds;
}

function infantryFindings(
  label: string,
  joints: Joint[],
  mesh: MeshData,
  sockets: Socket[],
  clips: SkeletonClips,
  aim: PoseRef,
  authority: Authority,
  tolerances: Tolerances,
  grounded: readonly MountRow[],
): Finding[] {
  const out: Finding[] = [];
  for (const name of INFANTRY_SOCKETS)
    if (!sockets.some((s) => s.name === name))
      out.push(
        finding(
          "nodes.missing",
          `${label}: no "${name}" socket`,
          `add an empty named "${name}" parented to the right bone (eye: Head; muzzle: the weapon on hand_r)`,
        ),
      );
  const clip = clips.clips.find((c) => c.name === aim.clip);
  const locals = clip ? sampleClip(clips, clip, joints, aim.phase) : joints.map((j) => j.bind);
  const worlds = worldTransforms(
    joints.map((j) => j.parent),
    locals,
  );
  const pose = clip ? `in "${aim.clip}"` : "at bind (no aim reference clip)";
  const bounds = positionsBounds(skinPositions(mesh, joints, worlds));
  out.push(...groundFindings(label, bounds.min[2], tolerances));
  const socketAt = (name: string) => {
    const socket = sockets.find((s) => s.name === name);
    return socket ? pointAt(worlds[socket.joint], socket.offset.t) : null;
  };
  const eye = socketAt("eye");
  const muzzle = socketAt("muzzle");
  const height = bounds.max[2];
  if (Math.abs(height - authority.soldier_height_m) > tolerances.soldier_height_m)
    out.push(
      finding(
        "fit.soldier_height",
        `${label}: stands ${fmt(height)} m ${pose}, physics.soldier_height_m is ${authority.soldier_height_m} ± ${tolerances.soldier_height_m}`,
        "scale the body to the simulation's soldier height",
      ),
    );
  if (eye) {
    if (eye[2] < height * 0.5)
      out.push(
        finding(
          "basis.up",
          `${label}: eye at z ${fmt(eye[2])} m is below half the ${fmt(height)} m height`,
          "export with +Y up in glTF (Blender's default); the bake turns it to Z up",
        ),
      );
    else if (Math.abs(eye[2] - authority.infantry_eye_m) > tolerances.eye_m)
      out.push(
        finding(
          "fit.eye",
          `${label}: eye at ${fmt(eye[2])} m ${pose}, physics.infantry_eye_m is ${authority.infantry_eye_m} ± ${tolerances.eye_m}`,
          "move the eye socket, or rescale the body",
        ),
      );
  }
  if (muzzle && clip) {
    if (muzzle[0] <= 0)
      out.push(
        finding(
          "basis.forward",
          `${label}: muzzle at x ${fmt(muzzle[0])} m ${pose}, behind the origin`,
          "face the model along +X after the catalog's basis_yaw_deg (Quaternius rigs need 90)",
        ),
      );
    else if (
      !grounded.length &&
      Math.abs(muzzle[2] - authority.infantry_muzzle_m) > tolerances.muzzle_m
    )
      out.push(
        finding(
          "fit.muzzle",
          `${label}: muzzle at ${fmt(muzzle[2])} m ${pose}, physics.infantry_muzzle_m is ${authority.infantry_muzzle_m} ± ${tolerances.muzzle_m}`,
          "fix the standing-aim pose or the weapon's muzzle socket",
        ),
      );
  }
  for (const mount of grounded) {
    const hold = mount.operator_appearance!.active_pose!;
    const heldClip = clips.clips.find((c) => c.name === hold.clip);
    if (!heldClip) {
      out.push(
        finding(
          "structure.clips",
          `${label}: active_pose clip "${hold.clip}" is absent`,
          "author the declared supported hold on this skeleton",
        ),
      );
      continue;
    }
    const socket = sockets.find((s) => s.name === "muzzle");
    if (!socket) continue;
    const heldWorlds = worldTransforms(
      joints.map((j) => j.parent),
      sampleClip(clips, heldClip, joints, hold.phase),
    );
    const actual = pointAt(heldWorlds[socket.joint], socket.offset.t);
    const expected = vec3.add([0, 0, 0], mount.pivot_m, mount.muzzle_m!);
    const error = vec3.distance(actual, expected);
    if (error > tolerances.muzzle_m)
      out.push(
        finding(
          "fit.muzzle",
          `${label}: muzzle [${actual.map(fmt).join(", ")}] in active_pose "${hold.clip}" at ${hold.phase}, mount "${mount.id}" declares [${expected.map(fmt).join(", ")}] (${fmt(error)} m off, tolerance ${tolerances.muzzle_m})`,
          "fit the supported active kit's socket to its physical mount bore",
        ),
      );
  }
  return out;
}

/** A unit's model against the art rules of each class it draws as
 *  (`unitArt.ts`): its tiers and the class's budget. Two classes alike
 *  report a finding once. */
function unitArtFindings(
  label: string,
  scene: Scene,
  bundle: ArticulatedBundle | SkinnedBundle,
  triangles: readonly number[],
  classes: readonly (string | null)[],
): Finding[] {
  const out = classes.flatMap((cls) => {
    const rule = unitArtRule(cls);
    return [
      ...tierFindings(label, scene, triangles, rule.tier_ratio),
      ...budgetFindings(
        label,
        triangles,
        rule,
        `the ${cls ?? "default"} row of UNIT_ART (packages/scene-assets/src/unitArt.ts)`,
        bundle,
      ),
    ];
  });
  return out.filter((f, i) => out.findIndex((g) => g.message === f.message) === i);
}

// ---------------------------------------------------------------- vehicles

const MAST_CHAIN = ["deploy_mast", "deploy_mast_2", "deploy_mast_3", "deploy_mast_head"];
const ARC_BEARINGS = 12;
/** The node a mount role's parts hang from that the hull box leaves out: the
 *  gun's barrel overhangs the hull's front; the roof HMG and its ring stand
 *  above the roof. */
const OFF_HULL: Record<Articulation, string> = {
  gun: MOUNT_NODES.gun.pitch,
  hmg: MOUNT_NODES.hmg.yaw,
};

/** A node whose parts are dressing (antennas, stowage, crew): measured
 *  against its class's dressing allowance (`unitArt.ts`), not the hull's
 *  tolerance. Node extras keep numbers only, so the name is the mark. */
const DRESSING = "dressing_";
const isDressing = (name: string) => name.startsWith(DRESSING);

/**
 * One dressing node's parts, at rest, against the hull box of half extents
 * `half` standing on the ground: within `bulky_m` of every face, and above
 * that only thin (no wider than `thin_m` either way, as an antenna is) and
 * no higher than `thin_top_m` over the box's top, so dressing never reads
 * as cover the simulation lacks.
 */
function dressingFindings(
  label: string,
  positions: Float32Array,
  half: Vec3,
  { bulky_m, thin_m, thin_top_m }: DressingAllowance,
): Finding[] {
  if (!positions.length) return [];
  const b = positionsBounds(positions);
  const top = 2 * half[2];
  const faces: [string, number, number][] = [
    ["-x", -b.min[0], half[0]],
    ["+x", b.max[0], half[0]],
    ["-y", -b.min[1], half[1]],
    ["+y", b.max[1], half[1]],
  ];
  const over = faces
    .filter(([, reach, box]) => reach > box + bulky_m)
    .map(([face, reach, box]) => `${face} ${fmt(reach - box)} m past the box`);
  // What rises above the bulky allowance must be thin, and not too tall.
  const high = { min: [Infinity, Infinity], max: [-Infinity, -Infinity] };
  for (let i = 0; i < positions.length; i += 3)
    if (positions[i + 2] > top + bulky_m)
      for (const k of [0, 1]) {
        high.min[k] = Math.min(high.min[k], positions[i + k]);
        high.max[k] = Math.max(high.max[k], positions[i + k]);
      }
  const across = Math.max(high.max[0] - high.min[0], high.max[1] - high.min[1]);
  if (across > thin_m)
    over.push(`+z ${fmt(b.max[2] - top)} m over the box's top, ${fmt(across)} m across`);
  else if (b.max[2] > top + thin_top_m) over.push(`+z ${fmt(b.max[2] - top)} m over the box's top`);
  return over.length
    ? [
        finding(
          "fit.dressing",
          `${label} reaches ${over.join("; ")} (dressing: ${bulky_m} m past any face, and above that only parts at most ${thin_m} m across, up to ${thin_top_m} m)`,
          "bring bulky dressing (stowage, crew, tarps) within its allowance of the hull box, and give each tall thin part (an antenna) its own dressing node",
        ),
      ]
    : [];
}

/** Material roles that roll on the ground: a tyre turns, a track runs. */
const ROLLING_ROLES: readonly (MaterialRole | undefined)[] = ["rubber", "track"];
/** Nodes the renderer turns: a wheel spins, a track's belt scrolls. */
const isRunningGear = (name: string) => name.startsWith("wheel_") || name.startsWith("track_");

/** What every articulated appearance must be, whatever type draws it: on
 *  the ground, and what it rolls on there (a tyre or a track, by material
 *  role) under running gear the renderer turns (`wheel_*`, `track_*`). What
 *  only rests on the ground (skids, a belly, legs) needs no node. */
function articulatedFindings(
  label: string,
  nodes: ArticulatedNode[],
  materials: Material[],
  tolerances: Tolerances,
): Finding[] {
  const out: Finding[] = [];
  const worlds = articulatedWorlds(nodes);
  const minZ = positionsBounds(articulatedPositions(nodes, worlds, 0)).min[2];
  out.push(...groundFindings(label, minZ, tolerances));
  const turned = (i: number): boolean =>
    i >= 0 && (isRunningGear(nodes[i].name) || turned(nodes[i].parent));
  const p = vec3.create();
  const loose = new Set<string>();
  nodes.forEach((node, i) => {
    if (turned(i)) return;
    const mesh = node.tiers[0];
    for (const draw of mesh?.draws ?? []) {
      const role = materials[draw.material]?.role;
      if (!ROLLING_ROLES.includes(role)) continue;
      for (let k = draw.first; k < draw.first + draw.count; k++) {
        const z = vec3.transformMat4(
          p,
          vec3.fromBuffer(p, mesh.positions, mesh.indices[k] * 3),
          worlds[i],
        )[2];
        if (z <= minZ + tolerances.ground_m) {
          loose.add(`${node.name} (${role})`);
          break;
        }
      }
    }
  });
  if (loose.size)
    out.push(
      finding(
        "nodes.missing",
        `${label}: ${[...loose].join(", ")} rolls on the ground outside any "wheel_*" or "track_*" node`,
        `put each tyre under an empty named "wheel_*" at its axle, and each track under its "track_*" node`,
      ),
    );
  return out;
}

/** Whether `name` matches a part's node pattern: exact, or a prefix before a trailing `*`. */
const matchesNode = (pattern: string, name: string) =>
  pattern.endsWith("*") ? name.startsWith(pattern.slice(0, -1)) : name === pattern;

/**
 * An articulated model against one unit type that draws it, from the type's
 * own resolved numbers: its running gear (tracks or wheels), each mount by
 * the rig the model declares for it (`draws`, the catalog entry's `mounts`:
 * every mount declared once, its rig's nodes present and chained, the muzzle
 * where the type's mount row puts it at rest and through every bearing), a
 * deploy capability's legs and mast, the hull box, and each of its parts'
 * hardware.
 */
export function typeFindings(
  path: string,
  nodes: ArticulatedNode[],
  units: UnitCatalog,
  id: string,
  tolerances: Tolerances,
  draws: MountDraws | null,
): Finding[] {
  const label = `${path} (as ${id})`;
  const type = units.type(id);
  const hull = units.hull(id);
  const out: Finding[] = [];
  if (!hull) {
    out.push(
      finding(
        "fit.type_appearance",
        `${label}: unit type ${id} is a squad; its soldiers draw soldier appearances, not a vehicle`,
        `name this appearance only from a hull type's appearance`,
      ),
    );
    return out;
  }
  const index = new Map(nodes.map((n, i) => [n.name, i]));
  const isUnder = (child: number, ancestor: number) => {
    for (let p = nodes[child].parent; p >= 0; p = nodes[p].parent) if (p === ancestor) return true;
    return false;
  };
  const missing = (name: string) =>
    out.push(
      finding(
        "nodes.missing",
        `${label}: no "${name}" node`,
        `add an empty named "${name}" at the part's pivot`,
      ),
    );
  const chain = (parent: string, child: string) => {
    const [p, c] = [index.get(parent), index.get(child)];
    if (p !== undefined && c !== undefined && !isUnder(c, p))
      out.push(
        finding(
          "nodes.hierarchy",
          `${label}: "${child}" is not under "${parent}"`,
          `parent "${child}" to "${parent}"`,
        ),
      );
  };
  const up = (name: string) => {
    const node = nodes[index.get(name) ?? -1];
    if (node && node.pivot[2] <= 0)
      out.push(
        finding(
          "basis.up",
          `${label}: ${name} pivot at z ${fmt(node.pivot[2])} m`,
          "export with +Y up in glTF; the bake turns it to Z up",
        ),
      );
  };
  const worlds = articulatedWorlds(nodes);

  // Running gear.
  if ("tracked" in type.mobility) {
    for (const name of ["track_L", "track_R"]) {
      const node = nodes[index.get(name) ?? -1];
      if (!node) missing(name);
      else if (!(node.extras.track_length_m > 0 && node.extras.link_pitch_m > 0))
        out.push(
          finding(
            "nodes.track_properties",
            `${label}: "${name}" lacks positive track_length_m and link_pitch_m`,
            `set custom properties track_length_m (loop length) and link_pitch_m on "${name}" and export with custom properties`,
          ),
        );
    }
  } else if ("wheeled" in type.mobility) {
    if (!nodes.some((n) => n.name.startsWith("wheel_"))) missing("wheel_*");
    // Front wheels (wheel_F*) ahead of the rear ones (wheel_R*): the hull faces +X.
    const wheelX = (row: string) =>
      nodes.filter((n) => n.name.startsWith(`wheel_${row}`)).map((n) => n.pivot[0]);
    const [front, rear] = [wheelX("F"), wheelX("R")];
    if (front.length && rear.length && Math.min(...front) <= Math.max(...rear))
      out.push(
        finding(
          "basis.forward",
          `${label}: front wheels (wheel_F*) are not ahead of rear wheels (wheel_R*) along +X`,
          `face the ${type.name} along +X after the catalog's basis_yaw_deg`,
        ),
      );
  }

  // Mounts, by the rig the model declares for each.
  out.push(...mountDrawFindings(label, type, draws));
  const roles = mountRoles(type, draws).map((role) => (isArticulation(role) ? role : "hand"));
  const drawn: (MountNodes | null)[] = roles.map((role) =>
    role === "hand" ? null : MOUNT_NODES[role],
  );
  for (const [i, role] of roles.entries()) {
    if (role === "hand") continue;
    const n = MOUNT_NODES[role];
    for (const name of [n.yaw, n.pitch, n.muzzle]) if (!index.has(name)) missing(name);
    chain(n.yaw, n.pitch);
    chain(n.pitch, n.muzzle);
    up(n.yaw);
    if (role === "gun") {
      const muzzle = index.get(n.muzzle);
      const muzzleX = muzzle === undefined ? 1 : pointAt(worlds[muzzle], [0, 0, 0])[0];
      if (muzzleX <= 0)
        out.push(
          finding(
            "basis.forward",
            `${label}: ${type.mounts[i].name}'s muzzle at x ${fmt(muzzleX)} m, behind the hull origin`,
            "face the hull along +X after the catalog's basis_yaw_deg",
          ),
        );
    }
  }
  out.push(
    ...muzzleFindings(label, nodes, worlds, index, drawn, tolerances, {
      name: `units.${id}.mounts`,
      mounts: mountMuzzles(type.mounts),
    }),
  );

  // Stationary service does not imply legs or a mast. Declared deployment
  // hardware retains its chain and motion checks.
  const declaresDeployment = nodes.some(
    (node) =>
      node.name.startsWith("deploy_") ||
      DEPLOY_EXTRAS.some((key) => node.extras?.[key] !== undefined),
  );
  if (type.capabilities.deploy && declaresDeployment) {
    const legs = nodes
      .map((n) => n.name.match(/^deploy_leg_([A-Za-z0-9]+)(?:_(?:jack|pad))?$/)?.[1])
      .filter((leg): leg is string => !!leg);
    for (const leg of new Set(legs)) {
      if (!index.has(`deploy_leg_${leg}`)) missing(`deploy_leg_${leg}`);
      for (const suffix of ["_jack", "_pad"])
        if (!index.has(`deploy_leg_${leg}${suffix}`)) missing(`deploy_leg_${leg}${suffix}`);
      chain(`deploy_leg_${leg}`, `deploy_leg_${leg}_jack`);
      chain(`deploy_leg_${leg}_jack`, `deploy_leg_${leg}_pad`);
    }
    if (MAST_CHAIN.some((name) => index.has(name))) {
      for (const name of MAST_CHAIN) if (!index.has(name)) missing(name);
      for (let i = 1; i < MAST_CHAIN.length; i++) chain(MAST_CHAIN[i - 1], MAST_CHAIN[i]);
      up("deploy_mast");
    }
    out.push(...deployFindings(label, nodes, index, tolerances));
  }

  // The hull box, without what its mounts carry beyond it, its dressing or
  // its rotors (a rotor's disc is not its airframe's size). Dressing nested
  // in dressing is measured with the outer node.
  const dressing = nodes.flatMap((n, i) =>
    isDressing(n.name) && !nodes.some((d, j) => isDressing(d.name) && isUnder(i, j)) ? [i] : [],
  );
  const excluded = roles
    .flatMap((role) => (role === "hand" ? [] : [index.get(OFF_HULL[role])]))
    .filter((i): i is number => i !== undefined)
    .concat(dressing)
    .concat(nodes.flatMap((n, i) => (isRotor(n.name) ? [i] : [])));
  const hullPositions = articulatedPositions(
    nodes,
    worlds,
    0,
    (i) => !excluded.some((e) => i === e || isUnder(i, e)),
  );
  out.push(
    ...extentFindings(label, hullPositions, hull.half_extents_m, {
      code: "fit.hull_extents",
      rule: `units.${id}.body.hull.half_extents_m [${hull.half_extents_m.join(", ")}]`,
      tolerance: tolerances.hull_extent_m,
      fix: `fit the hull to the simulation's box; parts that stand outside it (antennas, stowage, crew) go under a "${DRESSING}*" node, held to its class's dressing allowance`,
    }),
  );
  const allowance = unitArtRule(vehicleClass(type)).dressing;
  for (const d of dressing)
    out.push(
      ...dressingFindings(
        `${label}: "${nodes[d].name}"`,
        articulatedPositions(nodes, worlds, 0, (i) => i === d || isUnder(i, d)),
        hull.half_extents_m,
        allowance,
      ),
    );

  // Each part's hardware.
  for (const part of type.parts ?? []) {
    const patterns = units.view.parts[part]?.nodes ?? [];
    const absent = patterns.filter((p) => !nodes.some((n) => matchesNode(p, n.name)));
    if (absent.length)
      out.push(
        finding(
          "fit.part_nodes",
          `${label}: lists part ${part}, but the model draws no ${absent.map((p) => `"${p}"`).join(", ")} node`,
          `a type listing part ${part} must draw its hardware: model it and name its nodes ${patterns.join(", ")}`,
        ),
      );
  }
  return out;
}

/** A hull model's mount declarations against its type: every mount with a
 *  muzzle is declared, each once, by a rig the renderer has, and nothing
 *  names a mount the type lacks. */
function mountDrawFindings(label: string, type: UnitType, draws: MountDraws | null): Finding[] {
  const out: Finding[] = [];
  const fix = (m: string) =>
    `declare it in the appearance's catalog entry: "mounts": { "${m}": ${Object.keys(MOUNT_NODES)
      .map((a) => `"${a}"`)
      .join(" | ")} }`;
  const names = new Set(type.mounts.map((m) => m.id));
  const rigs = new Map<string, string>();
  for (const [mount, rig] of Object.entries(draws ?? {})) {
    if (!names.has(mount))
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: declares mount "${mount}", which ${type.id} does not have (${[...names].join(", ") || "none"})`,
          "name the type's mount by its catalog row's id",
        ),
      );
    if (!isArticulation(rig))
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: mount "${mount}" is drawn by "${rig}", which is not a rig`,
          fix(mount),
        ),
      );
    else if (rigs.has(rig))
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: mounts "${rigs.get(rig)}" and "${mount}" are both drawn by the ${rig} rig`,
          "give each mount its own rig; two mounts never share a muzzle",
        ),
      );
    else rigs.set(rig, mount);
  }
  // A mount fixed in the hull is drawn by it; every mount that turns needs a rig.
  for (const m of type.mounts)
    if (m.muzzle_m && !draws?.[m.id] && !hullFixed(m))
      out.push(
        finding("fit.mount_draw", `${label}: mount "${m.id}" is not drawn by any rig`, fix(m.id)),
      );
  return out;
}

/** The nodes that draw a mount: the one it yaws on and its muzzle. */
interface MountNodes {
  yaw: string;
  muzzle: string;
}

/** Each mount's drawn muzzle against the simulation's (`mountMuzzle.ts`): at
 *  rest on its row, and as its carrier (the turret it is on) and its own
 *  ring turn through every pair of bearings, where the simulation's model
 *  puts it. The drawn mount's yaw is relative to its carrier, whose yaw it
 *  adds to its world bearing. */
function muzzleFindings(
  label: string,
  nodes: ArticulatedNode[],
  worlds: ReturnType<typeof articulatedWorlds>,
  index: Map<string, number>,
  /** Each mount's nodes, parallel to `rule.mounts`; null for a hand weapon. */
  drawn: (MountNodes | null)[],
  tolerances: Tolerances,
  rule: { name: string; mounts: (MountMuzzle | null)[] },
): Finding[] {
  const out: Finding[] = [];
  const yawed = (yaws: Map<number, number>) => {
    const overrides: Record<string, ArticulatedNode["bind"]> = {};
    for (const [node, bearing] of yaws) {
      const bind = nodes[node].bind;
      const r = quat.setAxisAngle(quat.create(), [0, 0, 1], bearing);
      overrides[nodes[node].name] = { ...bind, r: quat.multiply(quat.create(), r, bind.r) };
    }
    return articulatedWorlds(nodes, overrides);
  };
  const expected = vec3.create();
  drawn.forEach((mount, i) => {
    const m = rule.mounts[i];
    if (!mount) return;
    const [yaw, muzzle] = [index.get(mount.yaw), index.get(mount.muzzle)];
    if (!m || muzzle === undefined) return;
    const what = `${rule.name}[${i}] (${m.id})`;
    const rest = pointAt(worlds[muzzle], [0, 0, 0]);
    muzzleOffset(expected, m, 0, 0);
    const error = vec3.distance(rest, expected);
    if (error > tolerances.vehicle_muzzle_m)
      out.push(
        finding(
          "fit.vehicle_muzzle",
          `${label}: ${mount.muzzle} at [${rest.map(fmt).join(", ")}], ${what} puts it at [${[...expected].map(fmt).join(", ")}] (${fmt(error)} m off, tolerance ${tolerances.vehicle_muzzle_m})`,
          "move the gun's muzzle to the row's pivot plus muzzle, or change the type's mount row in the unit catalog",
        ),
      );
    if (yaw === undefined) return;
    const carrierNodes = m.on === null ? null : drawn[m.on];
    const carrier = carrierNodes ? index.get(carrierNodes.yaw) : undefined;
    const carriedBearings = carrier === undefined ? 1 : ARC_BEARINGS;
    let worst = 0;
    for (let c = 0; c < carriedBearings; c++) {
      const carried = (2 * Math.PI * c) / ARC_BEARINGS;
      for (let k = 0; k < ARC_BEARINGS; k++) {
        const own = (2 * Math.PI * k) / ARC_BEARINGS;
        const yaws = new Map([[yaw, own]]);
        if (carrier !== undefined) yaws.set(carrier, carried);
        const at = pointAt(yawed(yaws)[muzzle], [0, 0, 0]);
        muzzleOffset(expected, m, carried, carried + own);
        worst = Math.max(worst, vec3.distance(at, expected));
      }
    }
    if (worst > tolerances.muzzle_arc_m)
      out.push(
        finding(
          "fit.muzzle_arc",
          `${label}: as ${mount.yaw} turns, ${mount.muzzle} leaves the arc ${what} swings it through by up to ${fmt(worst)} m`,
          `put the ${mount.yaw}'s yaw pivot on the row's pivot_m, as the simulation's muzzle model does`,
        ),
      );
  });
  return out;
}

/** Deploying parts must be authored to move, and the legs' pads must reach
 *  the ground when deployed. */
function deployFindings(
  label: string,
  nodes: ArticulatedNode[],
  index: Map<string, number>,
  tolerances: Tolerances,
): Finding[] {
  const rig = articulationRig(nodes);
  if (!rig.deploy.length)
    return [
      finding(
        "nodes.deploy_motion",
        `${label}: no node carries a deploy window (deploy_start, deploy_end)`,
        `set custom properties ${DEPLOY_EXTRAS.join(", ")} on each deploying part (legs, jacks, mast stages)`,
      ),
    ];
  const worlds = worldTransforms(
    nodes.map((n) => n.parent),
    articulate(restLocals(nodes), nodes, rig, { ...REST_ARTICULATION, deploy: 1 }),
  );
  const out: Finding[] = [];
  for (const [name, pad] of index) {
    if (!/^deploy_leg_[A-Za-z0-9]+_pad$/.test(name)) continue;
    const under = (i: number) => {
      for (let p = i; p >= 0; p = nodes[p].parent) if (p === pad) return true;
      return false;
    };
    const positions = articulatedPositions(nodes, worlds, 0, under);
    if (!positions.length) continue;
    const low = positionsBounds(positions).min[2];
    if (Math.abs(low) > tolerances.ground_m)
      out.push(
        finding(
          "nodes.deploy_motion",
          `${label}: deployed, "${name}" rests at z ${fmt(low)} m, not on the ground (± ${tolerances.ground_m})`,
          "adjust the jack's deploy_move_z so the pad meets z = 0 at deploy 1",
        ),
      );
  }
  return out;
}

/** A tree, unscaled, stands inside the simulation's forest canopy on every
 *  tier, so the drawn crown never rises above or reaches past the foliage
 *  that attenuates sight. */
function canopyFindings(
  label: string,
  tiers: MeshData[],
  authority: Authority,
  tolerances: Tolerances,
): Finding[] {
  let top = 0;
  let reach = 0;
  for (const { positions: p } of tiers)
    for (let i = 0; i < p.length; i += 3) {
      top = Math.max(top, p[i + 2]);
      reach = Math.max(reach, Math.hypot(p[i], p[i + 1]));
    }
  const out: Finding[] = [];
  if (top > authority.canopy_height_m + tolerances.ground_m)
    out.push(
      finding(
        "fit.canopy",
        `${label}: crown top at ${fmt(top)} m, above the forests' canopy of ${authority.canopy_height_m} m (forests.rule.canopy_height_m)`,
        "lower the crown under the canopy height",
      ),
    );
  if (reach > authority.canopy_radius_m + tolerances.ground_m)
    out.push(
      finding(
        "fit.canopy",
        `${label}: crown reaches ${fmt(reach)} m from the trunk's axis, past the forests' canopy radius of ${authority.canopy_radius_m} m (forests.rule.canopy_radius_m)`,
        "narrow the crown inside the canopy radius",
      ),
    );
  return out;
}

/** The area a mesh's surface encloses where the level plane `z` cuts it:
 *  each triangle the plane crosses gives one edge of the outline, taken
 *  round the way its face looks out, so a closed surface's outline sums to
 *  its area wherever it stands. */
function sectionArea({ positions: p, indices }: MeshData, z: number): number {
  let twice = 0;
  const cut: number[] = [];
  for (let i = 0; i < indices.length; i += 3) {
    const [a, b, c] = [indices[i] * 3, indices[i + 1] * 3, indices[i + 2] * 3];
    cut.length = 0;
    for (const [from, to] of [
      [a, b],
      [b, c],
      [c, a],
    ]) {
      if (p[from + 2] < z === p[to + 2] < z) continue;
      const s = (z - p[from + 2]) / (p[to + 2] - p[from + 2]);
      cut.push(p[from] + s * (p[to] - p[from]), p[from + 1] + s * (p[to + 1] - p[from + 1]));
    }
    if (cut.length !== 4) continue;
    // The face's normal, level: the outline runs with it on its right.
    const nx =
      (p[b + 1] - p[a + 1]) * (p[c + 2] - p[a + 2]) - (p[b + 2] - p[a + 2]) * (p[c + 1] - p[a + 1]);
    const ny = (p[b + 2] - p[a + 2]) * (p[c] - p[a]) - (p[b] - p[a]) * (p[c + 2] - p[a + 2]);
    const along = (cut[2] - cut[0]) * -ny + (cut[3] - cut[1]) * nx;
    twice += Math.sign(along) * (cut[0] * cut[3] - cut[2] * cut[1]);
  }
  return Math.abs(twice) / 2;
}

/** Every tree is one size: its finest tier's top, and its bole's girth at
 *  breast height, stand within the kind's band (`SCENERY_KINDS`), so species
 *  differ in shape alone. */
function sizeFindings(
  label: string,
  finest: MeshData,
  size: NonNullable<SceneryRule["size"]>,
): Finding[] {
  let top = 0;
  for (let i = 2; i < finest.positions.length; i += 3) top = Math.max(top, finest.positions[i]);
  const bole = Math.sqrt(sectionArea(finest, size.breast_m) / Math.PI);
  const out: Finding[] = [];
  const outside = (value: number, nominal: number) => Math.abs(value / nominal - 1) > size.within;
  const band = (nominal: number) =>
    `${fmt(nominal * (1 - size.within))} to ${fmt(nominal * (1 + size.within))} m`;
  if (outside(top, size.top_m))
    out.push(
      finding(
        "fit.tree_size",
        `${label}: top at ${fmt(top)} m, outside every tree's ${band(size.top_m)}`,
        "build the kind to the one tree height (SCENERY_KINDS.tree.size)",
      ),
    );
  if (outside(bole, size.bole_radius_m))
    out.push(
      finding(
        "fit.tree_size",
        `${label}: the bole is ${fmt(bole)} m in radius at ${size.breast_m} m up, outside every tree's ${band(size.bole_radius_m)}`,
        "build the kind's trunk to the one girth (SCENERY_KINDS.tree.size), clear of its crown at breast height",
      ),
    );
  return out;
}

/** Each tier's triangles, and the bundle they are built into when its
 *  bytes and textures are budgeted too, against a budget; `owner` names where the budget lives,
 *  for the fix. */
export function budgetFindings(
  label: string,
  triangles: readonly number[],
  budget: Budget,
  owner: string,
  bundle?: Exclude<Bundle, SkeletonClips>,
): Finding[] {
  const out: Finding[] = [];
  const raise = (field: string) => `, or raise ${field} in ${owner} if nothing visibly suffers`;
  triangles.forEach((count, t) => {
    if (budget.tier_triangles && count > budget.tier_triangles[t])
      out.push(
        finding(
          "budget.tier_triangles",
          `${label}: tier ${t} draws ${count} triangles, over its budget of ${budget.tier_triangles[t]}`,
          `simplify the tier${raise("tier_triangles")}`,
        ),
      );
  });
  if (bundle && budget.bundle_bytes !== undefined) {
    const bytes = encodeBundle(bundle).byteLength;
    if (bytes > budget.bundle_bytes)
      out.push(
        finding(
          "budget.bundle_bytes",
          `${label}: the bundle is ${bytes} bytes, over its budget of ${budget.bundle_bytes}`,
          `thin the tiers, or shrink or share textures${raise("bundle_bytes")}`,
        ),
      );
  }
  if (bundle && budget.textures !== undefined && bundle.textures.length > budget.textures)
    out.push(
      finding(
        "budget.unit_textures",
        `${label}: the bundle carries ${bundle.textures.length} textures, over its budget of ${budget.textures}`,
        `share textures between materials${raise("textures")}`,
      ),
    );
  return out;
}

interface ExtentRule {
  code: "fit.hull_extents" | "fit.footprint";
  /** What the box is, for the message: `units.tank.body.hull.half_extents_m [..]`. */
  rule: string;
  tolerance: number;
  fix: string;
}

/** The five faces (±x, ±y, top) of `positions` against a box of half extents
 *  `half` standing on the ground, in both directions. */
function extentFindings(
  label: string,
  positions: Float32Array,
  half: Vec3,
  { code, rule, tolerance, fix }: ExtentRule,
): Finding[] {
  const b = positionsBounds(positions);
  const faces: [string, number, number][] = [
    ["-x", b.min[0], -half[0]],
    ["+x", b.max[0], half[0]],
    ["-y", b.min[1], -half[1]],
    ["+y", b.max[1], half[1]],
    ["+z", b.max[2], 2 * half[2]],
  ];
  const off = faces.filter(([, model, box]) => Math.abs(model - box) > tolerance);
  return off.length
    ? [
        finding(
          code,
          `${label}: ${off.map(([face, model, box]) => `${face} face at ${fmt(model)} m vs ${fmt(box)} m`).join("; ")} (${rule}, tolerance ${tolerance})`,
          fix,
        ),
      ]
    : [];
}

/** The `debris_*` node scene node `i` is or lies under, or null. */
function debrisNode(scene: Scene, i: number): string | null {
  for (let n = i; n >= 0; n = scene.nodes[n].parent)
    if (scene.nodes[n].name.startsWith(DEBRIS_NODE)) return scene.nodes[n].name;
  return null;
}

/** A wreck's debris state (`SceneryRule.debris`): every mesh under a
 *  `debris_*` node, and every point of its finest tier within `reach_m`
 *  across the ground of the box of half extents `half`, and no higher than
 *  `top_m`. */
function debrisFindings(
  label: string,
  scene: Scene,
  finest: MeshData,
  half: AppearanceEntry["footprint_half_m"],
  allowance: DebrisAllowance,
): Finding[] {
  const out: Finding[] = [];
  const meshes = scene.nodes.filter((n) => n.live && n.mesh !== null);
  const loose = meshes.filter((n) => debrisNode(scene, n.index) === null);
  if (loose.length)
    out.push(
      finding(
        "fit.debris",
        `${label}: mesh ${loose
          .slice(0, 3)
          .map((n) => `"${n.name}"`)
          .join(", ")} lies under no ${DEBRIS_NODE}* node; the debris state carries only debris`,
        `put each thrown piece under a ${DEBRIS_NODE}* node (wreckage.scatter), and leave the wreck's own parts in its other states`,
      ),
    );
  if (!half) return out;
  const p = finest.positions;
  let reach = 0;
  let top = -Infinity;
  for (let k = 0; k < p.length; k += 3) {
    const dx = Math.max(0, Math.abs(p[k]) - half[0]);
    const dy = Math.max(0, Math.abs(p[k + 1]) - half[1]);
    reach = Math.max(reach, Math.hypot(dx, dy));
    top = Math.max(top, p[k + 2]);
  }
  const over = [
    ...(reach > allowance.reach_m
      ? [`${fmt(reach)} m past the box, over its ${allowance.reach_m} m`]
      : []),
    ...(top > allowance.top_m ? [`${fmt(top)} m tall, over its ${allowance.top_m} m`] : []),
  ];
  const names = [...new Set(meshes.map((n) => debrisNode(scene, n.index) ?? n.name))];
  if (over.length)
    out.push(
      finding(
        "fit.debris",
        `${label}: debris (${names.slice(0, 5).join(", ")}) lies ${over.join(" and ")}`,
        "throw the debris nearer and lay it flat; the allowance is SCENERY_KINDS.wreck.debris (packages/scene-assets/src/scenery.ts)",
      ),
    );
  return out;
}

/** Each of a scenery appearance's pieces (`SceneryRule.pieces`) against the
 *  whole it was cut from, its `whole` state: inside it on every face. */
function pieceFindings(
  label: string,
  states: readonly { name: string; bounds: Bounds }[],
  whole: string,
  pieces: readonly string[],
): Finding[] {
  const of = states.find((s) => s.name === whole)?.bounds;
  if (!of) return [];
  // A millimetre: a piece is cut from the same export as its whole.
  const slack = 0.001;
  return states
    .filter((s) => pieces.includes(s.name))
    .flatMap(({ name, bounds: b }) => {
      const out = [0, 1, 2].flatMap((k) => [
        ...(b.min[k] < of.min[k] - slack ? [`-${"xyz"[k]}`] : []),
        ...(b.max[k] > of.max[k] + slack ? [`+${"xyz"[k]}`] : []),
      ]);
      return out.length
        ? [
            finding(
              "fit.piece",
              `${label} (${name}): reaches past its ${whole} state on ${out.join(", ")}`,
              `export the piece from the same build as ${whole}, where it lies in it`,
            ),
          ]
        : [];
    });
}

/** A scenery appearance that stands for a simulation prop, against its box. */
function footprintFindings(
  entry: AppearanceEntry,
  states: { name: string; tiers: MeshData[] }[],
  tolerances: Tolerances,
  label: string,
): Finding[] {
  if (SCENERY_KINDS[entry.scenery ?? ""]?.footprint.kind !== "prop") return [];
  const half = entry.footprint_half_m;
  if (!half)
    return [
      finding(
        "fit.footprint",
        `${label}: no footprint_half_m; a ${entry.scenery} prop is fitted to the simulation's box`,
        "set footprint_half_m in the catalog entry to the half extents of the box the simulation places (a map placement, or a wreck's hull)",
      ),
    ];
  return states.flatMap((state) =>
    extentFindings(`${label} (${state.name})`, state.tiers[0].positions, half, {
      code: "fit.footprint",
      rule: `footprint_half_m [${half.join(", ")}]`,
      tolerance: tolerances.footprint_m,
      fix: "fit the art to the simulation's box, or widen footprint_m for this appearance in the catalog (overhangs, rubble)",
    }),
  );
}

/** A regional look is scenery, of one of the presets' `families`: a body
 *  (a soldier, a vehicle) is the same in every region, and a kit is a region's
 *  by the templates that place it. */
export function regionalFindings(
  name: string,
  entry: Pick<AppearanceEntry, "unit" | "regional_family">,
  families: readonly string[],
): Finding[] {
  const family = entry.regional_family;
  if (family === undefined) return [];
  if (entry.unit !== "scenery")
    return [
      finding(
        "structure.regional_family",
        `${name}: a ${entry.unit} appearance names regional family "${family}"; only scenery is a region's look`,
        "remove regional_family from the catalog entry",
      ),
    ];
  if (!families.includes(family))
    return [
      finding(
        "structure.regional_family",
        `${name}: regional family "${family}" is not one of the presets' (${families.join(", ") || "none"})`,
        "name a family of parcels.regional_families in fixtures/map-presets.json",
      ),
    ];
  return [];
}

/** A prop's paints are scenery's, sRGB colours within [0, 1], and land on
 *  something: at least one of its `materials` takes a tint. */
export function paintFindings(
  name: string,
  entry: Pick<AppearanceEntry, "unit" | "paints">,
  materials: readonly { tint: number }[],
): Finding[] {
  const paints = entry.paints;
  if (paints === undefined) return [];
  const problem =
    entry.unit !== "scenery"
      ? `a ${entry.unit} appearance takes its side's tint, not paints`
      : paints.length === 0
        ? "it names no paint"
        : !paints.every((p) => p.length === 3 && p.every((c) => c >= 0 && c <= 1))
          ? "a paint is not an sRGB colour within [0, 1]"
          : !materials.some((m) => m.tint > 0)
            ? "no material of its art takes a tint, so no paint would show"
            : null;
  return problem
    ? [
        finding(
          "structure.paints",
          `${name}: paints: ${problem}`,
          "give the art's paintable material extras tint > 0, and list sRGB paints for scenery",
        ),
      ]
    : [];
}

/** A soldier's faction looks (`factions`): each a faction a battle fields,
 *  naming another soldier appearance on the same skeleton (so it poses with
 *  the same clips) that has no faction looks of its own. */
export function factionLookFindings(
  appearances: Record<string, Pick<AppearanceEntry, "unit" | "skeleton" | "factions">>,
): Finding[] {
  const out: Finding[] = [];
  const fix = "name, per faction, a soldier appearance on this one's skeleton";
  for (const [name, entry] of Object.entries(appearances))
    for (const [faction, look] of Object.entries(entry.factions ?? {})) {
      const of = appearances[look];
      const problem = !(FACTIONS as readonly string[]).includes(faction)
        ? `${faction} is no faction (${FACTIONS.join(", ")})`
        : !of
          ? `${faction} look "${look}", which the catalog lacks`
          : entry.unit !== "soldier" || of.unit !== "soldier"
            ? `faction looks are soldiers'; "${entry.unit === "soldier" ? look : name}" is a ${entry.unit === "soldier" ? of.unit : entry.unit}, not a soldier`
            : of.skeleton !== entry.skeleton
              ? `${faction} look "${look}" has skeleton ${of.skeleton}, not ${entry.skeleton}`
              : of.factions
                ? `${faction} look "${look}" has faction looks of its own`
                : null;
      if (problem) out.push(finding("structure.faction_look", `${name}: ${problem}`, fix));
    }
  return out;
}

/** Every vehicle appearance names its own wreck (`wreck`): a scenery
 *  appearance of kind `wreck` whose footprint is the hull of each unit type
 *  drawing the vehicle, so a wreck is its unit's size and no vehicle dies
 *  into another's. */
export function wreckFindings(
  appearances: Record<
    string,
    Pick<AppearanceEntry, "unit" | "scenery" | "wreck" | "footprint_half_m">
  >,
  units: UnitCatalog,
): Finding[] {
  const out: Finding[] = [];
  const wrong = (name: string, problem: string, fix: string) =>
    out.push(finding("fit.wreck", `${name}: ${problem}`, fix));
  for (const [name, entry] of Object.entries(appearances)) {
    if (entry.unit !== "vehicle") continue;
    const wreck = entry.wreck;
    const of = wreck === undefined ? undefined : appearances[wreck];
    if (wreck === undefined)
      wrong(
        name,
        "names no wreck",
        `export its wreck from its own hull and name it: "wreck": "${name}_wreck"`,
      );
    else if (!of)
      wrong(
        name,
        `names wreck "${wreck}", which the catalog lacks`,
        `add the scenery appearance "${wreck}" of kind wreck to assets/catalog.json`,
      );
    else if (of.unit !== "scenery" || of.scenery !== "wreck")
      wrong(
        name,
        `its wreck "${wreck}" is a ${of.scenery ?? of.unit}, not a wreck`,
        "name a scenery appearance of kind wreck",
      );
    else
      for (const id of units.ids) {
        const hull = units.hull(id);
        if (!hull || units.type(id).appearance !== name) continue;
        const half = of.footprint_half_m;
        if (half && half.every((h, k) => Math.abs(h - hull.half_extents_m[k]) < 1e-6)) continue;
        wrong(
          name,
          `its wreck "${wreck}" has footprint_half_m [${half?.join(", ") ?? "none"}], not unit type ${id}'s hull [${hull.half_extents_m.join(", ")}]`,
          "set the wreck's footprint_half_m to its unit's hull half extents: a wreck is its unit's size",
        );
      }
  }
  return out;
}

/** Every unit type draws appearances the catalog has, of the right kind: a
 *  hull its vehicle appearance, each soldier kind of a squad every soldier
 *  appearance of its set. */
export function typeAppearanceFindings(
  appearances: Record<string, Pick<AppearanceEntry, "unit">>,
  units: UnitCatalog,
): Finding[] {
  const out: Finding[] = [];
  const need = (id: string, name: string, unit: "soldier" | "vehicle", what: string) => {
    const entry = appearances[name];
    if (entry?.unit !== unit)
      out.push(
        finding(
          "fit.type_appearance",
          `unit type ${id}: ${what} names appearance "${name}", ${entry ? `a ${entry.unit} appearance, not a ${unit}` : "which the catalog lacks"}`,
          `add a ${unit} appearance "${name}" to assets/catalog.json, or name one that exists`,
        ),
      );
  };
  for (const id of units.ids) {
    const type = units.type(id);
    if (units.hull(id)) need(id, type.appearance ?? "(none)", "vehicle", "its hull");
    for (const kind of new Set(units.slots(id)))
      for (const name of units.soldier(kind).appearance)
        need(id, name, "soldier", `soldier kind ${kind}`);
    for (const mount of type.mounts)
      for (const name of [
        ...(mount.operator_appearance?.active ?? []),
        ...(mount.operator_appearance?.carried ?? []),
      ])
        need(id, name, "soldier", `operator of ${mount.id}`);
  }
  return out;
}
