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
import { contentSha256 } from "./glb.ts";
import { quat, vec3, type Vec3 } from "math";
import { pointAt } from "./trs.ts";
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
import { importScene } from "./scene.ts";
import {
  ALLOWED_LICENCES,
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
  type ProvenanceEntry,
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
  /** The reuse manifest's `third_party` entries; omit to skip provenance. */
  provenance?: ProvenanceEntry[];
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

// ---------------------------------------------------------------- provenance

export async function validateProvenance(
  path: string,
  bytes: Uint8Array,
  entries: ProvenanceEntry[],
): Promise<Finding[]> {
  const hash = await contentSha256(bytes);
  const entry = entries.find((e) => e.sha256 === hash);
  if (!entry)
    return [
      finding(
        "provenance.unlisted",
        `${path}: sha256 ${hash} is not in the reuse manifest`,
        `add a third_party entry {path: "${path}", sha256: "${hash}", licence, accepted_by} to specs/battle-look/assets/reuse-manifest.json`,
      ),
    ];
  if (!(ALLOWED_LICENCES as readonly string[]).includes(entry.licence) || !entry.accepted_by)
    return [
      finding(
        "provenance.licence",
        `${path}: licence "${entry.licence}"${entry.accepted_by ? "" : " (not accepted)"} is not allow-listed`,
        `use art under ${ALLOWED_LICENCES.join(", ")}, with the user's acceptance recorded in accepted_by`,
      ),
    ];
  return [];
}

// ---------------------------------------------------------------- skeleton clips

export async function validateSkeleton(
  id: string,
  entry: SkeletonEntry,
  bytes: Uint8Array,
  context: Pick<ValidationContext, "provenance">,
): Promise<Validation<SkeletonClips>> {
  const { scene, findings } = importScene(bytes, entry.source, entry.basis_yaw_deg);
  if (context.provenance)
    findings.push(...(await validateProvenance(entry.source, bytes, context.provenance)));
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
    if (context.provenance)
      findings.push(...(await validateProvenance(path, bytes, context.provenance)));
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
      const built = buildStaticState(imported.scene, path, materials);
      findings.push(...built.findings);
      if (!built.tiers) continue;
      const bounds = positionsBounds(built.tiers[0].positions);
      states.push({ name: state, tiers: built.tiers, bounds });
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
      findings.push(
        ...footprintFindings(entry, states, context.authority, tolerances, input.name),
      );
    const bounds = states.reduce<Bounds | null>((b, s) => union(b, s.bounds), null);
    const bundle: StaticBundle | null = bounds
      ? { kind: "static", states, materials: materials.materials, bounds }
      : null;
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

  if (kind === "articulated") {
    const built = buildArticulated(imported.scene, path);
    findings.push(...built.findings);
    if (!built.built) return { findings, stats: null, bundle: null, preview: null };
    const { nodes, materials } = built.built;
    findings.push(
      ...articulatedFindings(
        path,
        nodes,
        entry.unit as "tank" | "supply",
        context.authority,
        tolerances,
      ),
    );
    const bounds = posedBounds(nodes);
    const bundle: ArticulatedBundle = { kind: "articulated", nodes, materials, bounds };
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
  const { joints, tiers, materials, sockets } = built.built;
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
    bounds,
    far_pose: farPose,
    corpse_pose: corpsePose,
    sockets,
  };
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

const TANK_NODES = [
  "turret",
  "gun",
  "muzzle",
  "hmg",
  "hmg_gun",
  "hmg_muzzle",
  "track_L",
  "track_R",
];
const TANK_CHAINS: [string, string][] = [
  ["turret", "gun"],
  ["gun", "muzzle"],
  ["hmg", "hmg_gun"],
  ["hmg_gun", "hmg_muzzle"],
];
const MAST_CHAIN = ["deploy_mast", "deploy_mast_2", "deploy_mast_3", "deploy_mast_head"];
const ARC_BEARINGS = 12;

function articulatedFindings(
  label: string,
  nodes: ArticulatedNode[],
  unit: "tank" | "supply",
  authority: Authority,
  tolerances: Tolerances,
): Finding[] {
  const out: Finding[] = [];
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
  if (!nodes.some((n) => n.name.startsWith("wheel_"))) missing("wheel_*");
  const worlds = articulatedWorlds(nodes);
  const all = articulatedPositions(nodes, worlds, 0);
  const minZ = positionsBounds(all).min[2];
  out.push(...groundFindings(label, minZ, tolerances));

  if (unit === "tank") {
    for (const name of TANK_NODES) if (!index.has(name)) missing(name);
    for (const [p, c] of TANK_CHAINS) chain(p, c);
    for (const name of ["track_L", "track_R"]) {
      const node = nodes[index.get(name) ?? -1];
      if (node && !(node.extras.track_length_m > 0 && node.extras.link_pitch_m > 0))
        out.push(
          finding(
            "nodes.track_properties",
            `${label}: "${name}" lacks positive track_length_m and link_pitch_m`,
            `set custom properties track_length_m (loop length) and link_pitch_m on "${name}" and export with custom properties`,
          ),
        );
    }
    const turret = index.get("turret");
    const muzzle = index.get("muzzle");
    if (turret !== undefined && nodes[turret].pivot[2] <= 0)
      out.push(
        finding(
          "basis.up",
          `${label}: turret pivot at z ${fmt(nodes[turret].pivot[2])} m`,
          "export with +Y up in glTF; the bake turns it to Z up",
        ),
      );
    if (muzzle !== undefined) {
      const rest = pointAt(worlds[muzzle], [0, 0, 0]);
      if (rest[0] <= 0)
        out.push(
          finding(
            "basis.forward",
            `${label}: muzzle at x ${fmt(rest[0])} m, behind the hull origin`,
            "face the hull along +X after the catalog's basis_yaw_deg",
          ),
        );
      const rule = authority.tank_muzzle_local_m;
      const error = vec3.distance(rest, rule);
      if (error > tolerances.tank_muzzle_m)
        out.push(
          finding(
            "fit.tank_muzzle",
            `${label}: muzzle at [${rest.map(fmt).join(", ")}], physics.tank_muzzle_local_m is [${rule.join(", ")}] (${fmt(error)} m off, tolerance ${tolerances.tank_muzzle_m})`,
            "move the gun's muzzle to the rule, or change the rule in the fixture",
          ),
        );
      if (turret !== undefined) {
        let worst = 0;
        for (let k = 0; k < ARC_BEARINGS; k++) {
          const bearing = (2 * Math.PI * k) / ARC_BEARINGS;
          const bind = nodes[turret].bind;
          const yaw = quat.setAxisAngle(quat.create(), [0, 0, 1], bearing);
          const yawed = articulatedWorlds(nodes, {
            turret: { ...bind, r: quat.multiply(quat.create(), yaw, bind.r) },
          });
          const at = pointAt(yawed[muzzle], [0, 0, 0]);
          const expected = vec3.rotateZ(vec3.create(), rest, [0, 0, 0], bearing);
          worst = Math.max(worst, vec3.distance(at, expected));
        }
        if (worst > tolerances.muzzle_arc_m)
          out.push(
            finding(
              "fit.muzzle_arc",
              `${label}: under turret yaw the muzzle leaves the simulation's arc by up to ${fmt(worst)} m`,
              "put the turret's yaw pivot on the hull origin's vertical axis, as the simulation's muzzle model does",
            ),
          );
      }
    }
    const excluded = ["gun", "hmg"]
      .map((n) => index.get(n))
      .filter((i): i is number => i !== undefined);
    const hull = articulatedPositions(
      nodes,
      worlds,
      0,
      (i) => !excluded.some((e) => i === e || isUnder(i, e)),
    );
    out.push(
      ...extentFindings(
        label,
        hull,
        authority.tank_half_extents_m,
        hullRule("tank_half_extents_m", authority.tank_half_extents_m, tolerances),
      ),
    );
  } else {
    const legs = nodes
      .map((n) => n.name.match(/^deploy_leg_([A-Za-z0-9]+)$/)?.[1])
      .filter((id): id is string => !!id);
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
    const mast = index.get("deploy_mast");
    if (mast !== undefined && nodes[mast].pivot[2] <= 0)
      out.push(
        finding(
          "basis.up",
          `${label}: deploy_mast pivot at z ${fmt(nodes[mast].pivot[2])} m`,
          "export with +Y up in glTF; the bake turns it to Z up",
        ),
      );
    const wheelX = (row: string) =>
      nodes.filter((n) => new RegExp(`^wheel_${row}`).test(n.name)).map((n) => n.pivot[0]);
    const [front, rear] = [wheelX("F"), wheelX("R")];
    if (front.length && rear.length && Math.min(...front) <= Math.max(...rear))
      out.push(
        finding(
          "basis.forward",
          `${label}: front wheels (wheel_F*) are not ahead of rear wheels (wheel_R*) along +X`,
          "face the truck along +X after the catalog's basis_yaw_deg",
        ),
      );
    out.push(
      ...extentFindings(
        label,
        all,
        authority.supply_half_extents_m,
        hullRule("supply_half_extents_m", authority.supply_half_extents_m, tolerances),
      ),
    );
  }
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
          `${label}: crown top at ${fmt(top)} m, above the forests' canopy of ${authority.canopy_height_m} m (map.forests[].canopy_height_m)`,
          "lower the crown under the canopy height; placement scales each tree down to fit its forest",
        ),
      ]
    : [];
}

interface ExtentRule {
  code: "fit.hull_extents" | "fit.footprint";
  /** What the box is, for the message: `physics.tank_half_extents_m [..]`. */
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

const hullRule = (rule: string, half: Vec3, tolerances: Tolerances): ExtentRule => ({
  code: "fit.hull_extents",
  rule: `physics.${rule} [${half.join(", ")}]`,
  side: tolerances.hull_extent_m,
  top: tolerances.hull_top_m,
  fix: "fit the hull to the simulation's box, or widen hull_extent_m (sides) or hull_top_m (antennas, cupola) for this appearance in the catalog",
});

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
        ? `footprint_half_m [${half.join(", ")}] at buildings.ruin_height_m ${authority.ruin_height_m}`
        : `footprint_half_m [${half.join(", ")}]`,
      side: tolerances.footprint_m,
      top: tolerances.footprint_m,
      fix: "fit the art to the simulation's box, or widen footprint_m for this appearance in the catalog (roof overhangs, rubble)",
    });
  });
}
