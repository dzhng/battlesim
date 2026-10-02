// The appearance validator. Isomorphic: it takes bytes and data, never paths
// to read, so the CLI, the bake and tests share it. `validateAppearance` and
// `validateSkeleton` import, build and check one catalog entry; they return the
// findings, stats and — when nothing is an error — the bundle to encode.

import {
  buildArticulated,
  buildClips,
  buildSkinned,
  buildStaticState,
  MaterialTable,
  triangleCount,
} from "./build.ts";
import { quat, vec3, type Vec3 } from "math";
import { pointAt } from "./trs.ts";
import { mountMuzzles, muzzleOffset, type MountMuzzle } from "./mountMuzzle.ts";
import {
  MOUNT_NODES,
  isArticulation,
  mountRoles,
  type Articulation,
  type MountDraws,
  type UnitCatalog,
  type UnitType,
} from "./units.ts";
import { SCENERY_KINDS, requiredStates } from "./scenery.ts";
import {
  DEPLOY_EXTRAS,
  REST_ARTICULATION,
  articulate,
  articulationRig,
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
import { bindTextures, importScene } from "./scene.ts";
import { textureFindings } from "./texture.ts";
import { grassStripFindings } from "./grass.ts";
import {
  BUILDING_STATES,
  INFANTRY_CLIPS,
  INFANTRY_SOCKETS,
  UNIT_BUNDLE_KIND,
  type AppearanceEntry,
  type ArticulatedBundle,
  type ArticulatedNode,
  type Authority,
  type Bounds,
  type Bundle,
  type Finding,
  type Joint,
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
    const required = requiredStates(entry.unit, entry.scenery, BUILDING_STATES);
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
      if (entry.unit === "scenery" && SCENERY_KINDS[entry.scenery ?? ""]?.blades)
        findings.push(...grassStripFindings(path, built.tiers));
      findings.push(...groundFindings(`${path}`, bounds.min[2], tolerances));
      if (
        entry.unit === "scenery" &&
        entry.scenery !== undefined &&
        SCENERY_KINDS[entry.scenery]?.footprint.kind === "tree"
      )
        findings.push(...canopyFindings(path, bounds, context.authority, tolerances));
    }
    states.sort((a, b) => a.name.localeCompare(b.name));
    if (required)
      findings.push(...footprintFindings(entry, states, context.authority, tolerances, input.name));
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
    if (bundle) findings.push(...textureFindings(input.name, bundle));
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
      ...articulatedFindings(path, nodes, tolerances),
      ...types.flatMap((id) =>
        typeFindings(path, nodes, units, id, tolerances, entry.mounts ?? null),
      ),
    );
    const bounds = posedBounds(nodes);
    const bundle: ArticulatedBundle = { kind: "articulated", nodes, materials, textures, bounds };
    findings.push(...textureFindings(path, bundle));
    return {
      findings,
      stats: {
        kind,
        tiers: sumTiers(nodes.map((n) => n.tiers)),
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
  findings.push(...textureFindings(path, bundle));
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
    else if (Math.abs(muzzle[2] - authority.infantry_muzzle_m) > tolerances.muzzle_m)
      out.push(
        finding(
          "fit.muzzle",
          `${label}: muzzle at ${fmt(muzzle[2])} m ${pose}, physics.infantry_muzzle_m is ${authority.infantry_muzzle_m} ± ${tolerances.muzzle_m}`,
          "fix the standing-aim pose or the weapon's muzzle socket",
        ),
      );
  }
  return out;
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

/** What every articulated appearance must be, whatever type draws it: on
 *  the ground, with wheels. */
function articulatedFindings(
  label: string,
  nodes: ArticulatedNode[],
  tolerances: Tolerances,
): Finding[] {
  const out: Finding[] = [];
  if (!nodes.some((n) => n.name.startsWith("wheel_")))
    out.push(
      finding(
        "nodes.missing",
        `${label}: no "wheel_*" node`,
        `add an empty named "wheel_*" at the part's pivot`,
      ),
    );
  const minZ = positionsBounds(articulatedPositions(nodes, articulatedWorlds(nodes), 0)).min[2];
  out.push(...groundFindings(label, minZ, tolerances));
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

  // Deploying.
  if (type.capabilities.deploy) {
    const legs = nodes
      .map((n) => n.name.match(/^deploy_leg_([A-Za-z0-9]+)$/)?.[1])
      .filter((leg): leg is string => !!leg);
    if (!legs.length) missing("deploy_leg_*");
    for (const leg of legs) {
      for (const suffix of ["_jack", "_pad"])
        if (!index.has(`deploy_leg_${leg}${suffix}`)) missing(`deploy_leg_${leg}${suffix}`);
      chain(`deploy_leg_${leg}`, `deploy_leg_${leg}_jack`);
      chain(`deploy_leg_${leg}_jack`, `deploy_leg_${leg}_pad`);
    }
    for (const name of MAST_CHAIN) if (!index.has(name)) missing(name);
    for (let i = 1; i < MAST_CHAIN.length; i++) chain(MAST_CHAIN[i - 1], MAST_CHAIN[i]);
    out.push(...deployFindings(label, nodes, index, tolerances));
    up("deploy_mast");
  }

  // The hull box, without what its mounts carry beyond it.
  const excluded = roles
    .flatMap((role) => (role === "hand" ? [] : [index.get(OFF_HULL[role])]))
    .filter((i): i is number => i !== undefined);
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
      side: tolerances.hull_extent_m,
      top: tolerances.hull_top_m,
      fix: "fit the hull to the simulation's box, or widen hull_extent_m (sides) or hull_top_m (antennas, cupola) for this appearance in the catalog",
    }),
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
  const names = new Set(type.mounts.map((m) => m.name));
  const rigs = new Map<string, string>();
  for (const [mount, rig] of Object.entries(draws ?? {})) {
    if (!names.has(mount))
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: declares mount "${mount}", which ${type.id} does not have (${[...names].join(", ") || "none"})`,
          "name the type's mount exactly as its catalog row does",
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
  for (const m of type.mounts)
    if (m.muzzle_m && !draws?.[m.name])
      out.push(
        finding(
          "fit.mount_draw",
          `${label}: mount "${m.name}" is not drawn by any rig`,
          fix(m.name),
        ),
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
    const what = `${rule.name}[${i}] (${m.name})`;
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

/** A tree, unscaled, stands inside the simulation's lowest forest canopy:
 *  placement only scales it down to fit each forest, so the drawn crown never
 *  rises above the foliage that attenuates sight. */
function canopyFindings(
  label: string,
  bounds: Bounds,
  authority: Authority,
  tolerances: Tolerances,
): Finding[] {
  const top = bounds.max[2];
  return top > authority.canopy_height_m + tolerances.ground_m
    ? [
        finding(
          "fit.canopy",
          `${label}: crown top at ${fmt(top)} m, above the forests' canopy of ${authority.canopy_height_m} m (forests.rule.canopy_height_m)`,
          "lower the crown under the canopy height; placement scales each tree down to fit its forest",
        ),
      ]
    : [];
}

interface ExtentRule {
  code: "fit.hull_extents" | "fit.footprint";
  /** What the box is, for the message: `units.tank.body.hull.half_extents_m [..]`. */
  rule: string;
  side: number;
  top: number;
  fix: string;
}

/** The five faces (±x, ±y, top) of `positions` against a box of half extents
 *  `half` standing on the ground, in both directions. */
function extentFindings(
  label: string,
  positions: Float32Array,
  half: Vec3,
  { code, rule, side, top, fix }: ExtentRule,
): Finding[] {
  const b = positionsBounds(positions);
  const faces: [string, number, number, number][] = [
    ["-x", b.min[0], -half[0], side],
    ["+x", b.max[0], half[0], side],
    ["-y", b.min[1], -half[1], side],
    ["+y", b.max[1], half[1], side],
    ["+z", b.max[2], 2 * half[2], top],
  ];
  const off = faces.filter(([, model, box, tolerance]) => Math.abs(model - box) > tolerance);
  return off.length
    ? [
        finding(
          code,
          `${label}: ${off.map(([face, model, box]) => `${face} face at ${fmt(model)} m vs ${fmt(box)} m`).join("; ")} (${rule}, tolerance ${side === top ? side : `${side}, top ${top}`})`,
          fix,
        ),
      ]
    : [];
}

/** A static appearance that stands for a simulation prop, against its box. */
function footprintFindings(
  entry: AppearanceEntry,
  states: { name: string; tiers: MeshData[] }[],
  authority: Authority,
  tolerances: Tolerances,
  label: string,
): Finding[] {
  const rule = entry.unit === "building" ? null : SCENERY_KINDS[entry.scenery ?? ""];
  if (entry.unit !== "building" && rule?.footprint.kind !== "prop") return [];
  const half = entry.footprint_half_m;
  if (!half)
    return [
      finding(
        "fit.footprint",
        `${label}: no footprint_half_m; a ${entry.unit === "building" ? "building" : `${entry.scenery} prop`} is fitted to the simulation's box`,
        "set footprint_half_m in the catalog entry to the half extents of the box the simulation places (a map placement, or a wreck's hull)",
      ),
    ];
  return states.flatMap((state) => {
    const ruin = entry.unit === "building" && state.name === "ruin";
    const box: Vec3 = ruin ? [half[0], half[1], authority.ruin_height_m / 2] : half;
    return extentFindings(`${label} (${state.name})`, state.tiers[0].positions, box, {
      code: "fit.footprint",
      rule: ruin
        ? `footprint_half_m [${half.join(", ")}] at the building's destroyed height (ruin_height_m ${authority.ruin_height_m})`
        : `footprint_half_m [${half.join(", ")}]`,
      side: tolerances.footprint_m,
      top: tolerances.footprint_m,
      fix: "fit the art to the simulation's box, or widen footprint_m for this appearance in the catalog (roof overhangs, rubble)",
    });
  });
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
  }
  return out;
}
