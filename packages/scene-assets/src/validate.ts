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
import { vec3, type Vec3 } from "math";
import { encodeBundle } from "./codec.ts";
import { FACTIONS, type UnitCatalog } from "./units.ts";
import { SCENERY_KINDS } from "./scenery.ts";
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
import { SOLDIER_CLASS, tierFindings, unitArtRule } from "./unitArt.ts";
import { materialFindings } from "./material.ts";
import { textureFindings } from "./texture.ts";
import { grassStripFindings } from "./grass.ts";
import { canopyFindings, sizeFindings } from "./treeFit.ts";
import { debrisFindings, pieceFindings } from "./wreckFit.ts";
import { footprintFindings, groundFindings } from "./boxFit.ts";
import { infantryFindings } from "./infantryFit.ts";
import { typeFindings } from "./vehicleFit.ts";
import {
  INFANTRY_CLIPS,
  INFANTRY_CORPSES,
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
  type StaticBundle,
  type Tolerances,
} from "./schema.ts";
import { finding, fmt } from "./finding.ts";

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

const tierStats = (tiers: MeshData[]) =>
  tiers.map((m) => ({ triangles: triangleCount(m), vertices: m.positions.length / 3 }));
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

/** One state of a static appearance, imported, built and judged against its
 *  scenery kind's row, with its findings; the state is null when it does not
 *  build. */
async function staticState(
  input: AppearanceInput,
  [state, path]: [string, string],
  materials: MaterialTable,
  context: ValidationContext,
  tolerances: Tolerances,
): Promise<{ state: StaticBundle["states"][number] | null; findings: Finding[] }> {
  const findings: Finding[] = [];
  const imported = importScene(input.files[path], path, input.entry.basis_yaw_deg);
  findings.push(...imported.findings);
  if (!imported.scene) return { state: null, findings };
  findings.push(...(await bindTextures(imported.scene, path)));
  const built = buildStaticState(imported.scene, path, materials);
  findings.push(...built.findings);
  if (!built.tiers) return { state: null, findings };
  const bounds = positionsBounds(built.tiers[0].positions);
  const judged = { name: state, tiers: built.tiers, bounds };
  const rule = SCENERY_KINDS[input.entry.scenery ?? ""];
  const debris = rule?.debris?.state === state ? rule.debris : null;
  // A wreck's tiers are a unit's: it is drawn where its vehicle was. It
  // takes the default row (no vehicle class sets a ratio of its own yet).
  // A piece or its debris is part of its whole's tiers, so only the whole
  // is held to the ratio: a thrown turret's far tier is a few boxes either way.
  if (input.entry.scenery === "wreck")
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
      ...debrisFindings(path, imported.scene, built.tiers[0], input.entry.footprint_half_m, debris),
    );
    return { state: judged, findings };
  }
  // A piece is held to its whole (`pieceFindings`), not to the ground or the box.
  if (rule?.pieces?.includes(state)) return { state: judged, findings };
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
        `${path}: stands ${fmt(bounds.max[2])} m tall, over the ${rule.top_m} m a ${input.entry.scenery} may (it has no body to hide behind)`,
        "lower the art under the kind's top_m (packages/scene-assets/src/scenery.ts)",
      ),
    );
  return { state: judged, findings };
}

/** A static appearance: every state of its scenery kind, judged one by one
 *  and then together. */
async function validateStaticAppearance(
  input: AppearanceInput,
  sources: [string, string][],
  sourceBytes: number,
  context: ValidationContext,
  tolerances: Tolerances,
): Promise<Validation<StaticBundle>> {
  const { entry } = input;
  const findings: Finding[] = [];
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
  for (const source of sources) {
    const judged = await staticState(input, source, materials, context, tolerances);
    findings.push(...judged.findings);
    if (judged.state) states.push(judged.state);
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
          kind: "static",
          tiers: sumTiers(states.map((s) => s.tiers)),
          source_bytes: sourceBytes,
          bounds: bounds ?? undefined,
        }
      : null,
    bundle: hasErrors(findings) ? null : bundle,
    preview: bundle,
  };
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

  if (kind === "static")
    return validateStaticAppearance(input, sources, sourceBytes, context, tolerances);

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
        types.length ? [...new Set(types.map((id) => units.type(id).vehicle_class))] : [null],
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
  if (!skeleton.clips.clips.some((c) => c.name === farPose.clip))
    findings.push(
      finding(
        "structure.clips",
        `${path}: far_pose clip "${farPose.clip}" is not on skeleton ${skeleton.clips.id}`,
        `name a clip of ${skeleton.clips.id} in far_pose`,
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
    // Each fall's end: the skeleton's clip roles (`INFANTRY_CLIPS`) carry them.
    corpse_poses: INFANTRY_CORPSES.map((clip) => ({ clip, phase: 1 })),
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
