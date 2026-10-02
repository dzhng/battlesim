// Engine-space scenes → bundle content. Geometry is merged per tier and per
// owner (the whole body, one articulated node, one building state), grouped
// into one draw range per material. Structural problems become findings.

import { mat3, mat4, quat, vec3, type Mat4, type Quat, type Vec3 } from "math";
import { decomposeTrs, inverse, mul, pointAt, trsMatrix, type Trs } from "./trs.ts";
import {
  meshTier,
  nearestAncestor,
  type Primitive,
  type Scene,
  type SceneMaterial,
} from "./scene.ts";
import {
  CHANNEL_ABSENT,
  CHANNEL_ANIMATED,
  CHANNEL_CONSTANT,
  TEXTURE_CHANNELS,
  TIER_COUNT,
  type MaterialTextures,
  type Texture,
  type ArticulatedNode,
  type Clip,
  type ClipDeclaration,
  type Finding,
  type Joint,
  type Material,
  type MeshData,
  type SkeletonClips,
  type Socket,
} from "./schema.ts";

type AddFinding = (code: Finding["code"], message: string, fix: string) => void;
const collector =
  (findings: Finding[], label: string): AddFinding =>
  (code, message, fix) =>
    findings.push({ code, severity: "error", message: `${label}: ${message}`, fix });

const DEFAULT_MATERIAL: Material = {
  name: "default",
  base_color: [0.8, 0.8, 0.8, 1],
  metallic: 0,
  roughness: 1,
  tint: 0,
  coverage: { kind: "opaque" },
};

/** Materials, and the textures they sample, deduplicated across every source
 *  feeding one bundle; a texture is one entry however many materials share it. */
export class MaterialTable {
  readonly materials: Material[] = [];
  readonly textures: Texture[] = [];
  private readonly keys = new Map<string, number>();
  private readonly textureSlots = new Map<string, number>();
  slot(source: SceneMaterial | undefined): number {
    let m: Material = DEFAULT_MATERIAL;
    if (source) {
      const { images: _images, sources, ...fields } = source;
      m = fields;
      const textures: MaterialTextures = {};
      for (const channel of TEXTURE_CHANNELS) {
        const texture = sources?.[channel];
        if (!texture) continue;
        let at = this.textureSlots.get(texture.id);
        if (at === undefined) {
          at = this.textures.length;
          this.textures.push(texture);
          this.textureSlots.set(texture.id, at);
        }
        textures[channel] = at;
      }
      if (Object.keys(textures).length) m = { ...fields, textures };
    }
    const key = JSON.stringify(m);
    let slot = this.keys.get(key);
    if (slot === undefined) {
      slot = this.materials.length;
      this.materials.push(m);
      this.keys.set(key, slot);
    }
    return slot;
  }
}

interface MeshPart {
  primitive: Primitive;
  material: number;
  /** Source vertex space → owner space. */
  transform: Mat4;
  /** Skin slot → bundle joint, for skinned primitives. */
  slots?: number[];
  /** The one joint a rigid part follows. */
  rigidJoint?: number;
}

/** Merge parts into one mesh with contiguous draw ranges per material. */
export function mergeParts(parts: MeshPart[], skinned: boolean): MeshData {
  const ordered = [...parts].sort((a, b) => a.material - b.material);
  const vertexCount = ordered.reduce((n, p) => n + p.primitive.positions.length / 3, 0);
  const indexCount = ordered.reduce((n, p) => n + p.primitive.indices.length, 0);
  const mesh: MeshData = {
    positions: new Float32Array(vertexCount * 3),
    normals: new Int16Array(vertexCount * 4),
    uvs: new Float32Array(vertexCount * 2),
    colors: new Uint8Array(vertexCount * 4),
    indices: vertexCount > 65536 ? new Uint32Array(indexCount) : new Uint16Array(indexCount),
    draws: [],
  };
  if (skinned) {
    mesh.joints = new Uint8Array(vertexCount * 4);
    mesh.weights = new Uint16Array(vertexCount * 4);
  }
  if (ordered.some((p) => p.primitive.tangents)) mesh.tangents = new Int16Array(vertexCount * 4);
  let base = 0;
  let first = 0;
  const position = vec3.create();
  const normal = vec3.create();
  const tangent = vec3.create();
  const normalMatrix = mat3.create();
  const linear = mat3.create();
  for (const part of ordered) {
    const p = part.primitive;
    const count = p.positions.length / 3;
    mat3.normalFromMat4(normalMatrix, part.transform);
    mat3.fromMat4(linear, part.transform);
    if (mesh.tangents && p.tangents)
      for (let v = 0; v < count; v++) {
        vec3.transformMat3(tangent, vec3.fromBuffer(tangent, p.tangents, v * 4), linear);
        if (vec3.length(tangent) < 1e-8) {
          // A sliver triangle's corner gets no tangent from its UVs: any
          // direction across its normal will do, since it covers no pixels.
          vec3.transformMat3(normal, vec3.fromBuffer(normal, p.normals, v * 3), normalMatrix);
          vec3.cross(tangent, Math.abs(normal[2]) < 0.9 ? [0, 0, 1] : [1, 0, 0], normal);
        }
        vec3.normalize(tangent, tangent);
        const o = (base + v) * 4;
        for (let c = 0; c < 3; c++)
          mesh.tangents[o + c] = Math.round(Math.max(-1, Math.min(1, tangent[c])) * 32767);
        mesh.tangents[o + 3] = p.tangents[v * 4 + 3] < 0 ? -32767 : 32767;
      }
    for (let v = 0; v < count; v++) {
      const o = base + v;
      vec3.transformMat4(position, vec3.fromBuffer(position, p.positions, v * 3), part.transform);
      vec3.toBuffer(mesh.positions, position, o * 3);
      vec3.normalize(
        normal,
        vec3.transformMat3(normal, vec3.fromBuffer(normal, p.normals, v * 3), normalMatrix),
      );
      for (let c = 0; c < 3; c++)
        mesh.normals[o * 4 + c] = Math.round(Math.max(-1, Math.min(1, normal[c])) * 32767);
      mesh.uvs[o * 2] = p.uvs[v * 2];
      mesh.uvs[o * 2 + 1] = p.uvs[v * 2 + 1];
      for (let c = 0; c < 4; c++)
        mesh.colors[o * 4 + c] = Math.round(Math.max(0, Math.min(1, p.colors[v * 4 + c])) * 255);
      if (skinned) writeInfluences(mesh, o, part, v);
    }
    const last = mesh.draws[mesh.draws.length - 1];
    if (last && last.material === part.material) last.count += p.indices.length;
    else mesh.draws.push({ material: part.material, first, count: p.indices.length });
    for (let i = 0; i < p.indices.length; i++) mesh.indices[first + i] = base + p.indices[i];
    first += p.indices.length;
    base += count;
  }
  return mesh;
}

function writeInfluences(mesh: MeshData, o: number, part: MeshPart, v: number) {
  const joints = mesh.joints!;
  const weights = mesh.weights!;
  if (part.rigidJoint !== undefined) {
    joints[o * 4] = part.rigidJoint;
    weights[o * 4] = 65535;
    return;
  }
  const w = part.primitive.weights!.subarray(v * 4, v * 4 + 4);
  const sum = w[0] + w[1] + w[2] + w[3];
  const q = Array.from(w, (x) => Math.round((x / sum) * 65535));
  const heaviest = q.indexOf(Math.max(...q));
  q[heaviest] += 65535 - (q[0] + q[1] + q[2] + q[3]);
  for (let c = 0; c < 4; c++) {
    weights[o * 4 + c] = q[c];
    joints[o * 4 + c] = q[c] > 0 ? part.slots![part.primitive.joints![v * 4 + c]] : 0;
  }
}

export const triangleCount = (mesh: MeshData) => mesh.indices.length / 3;

/** Mesh nodes by tier; checks four `_LOD<n>` tiers, finest first. */
function tierParts(
  scene: Scene,
  add: AddFinding,
  part: (node: number, primitive: Primitive) => MeshPart | null,
): MeshPart[][] {
  const tiers: MeshPart[][] = Array.from({ length: TIER_COUNT }, () => []);
  const named = new Set<number>();
  for (const node of scene.nodes) {
    if (node.mesh === null || !node.live) continue;
    const tier = meshTier(node.name);
    if (tier !== null) named.add(tier);
    for (const primitive of scene.meshes[node.mesh]?.primitives ?? []) {
      const made = part(node.index, primitive);
      if (!made) continue;
      if (tier === null) tiers.forEach((t) => t.push(made));
      else if (tier < TIER_COUNT) tiers[tier].push(made);
    }
  }
  const expected = Array.from({ length: TIER_COUNT }, (_, i) => i);
  if (named.size !== TIER_COUNT || !expected.every((t) => named.has(t)))
    add(
      "structure.tier_count",
      `mesh tiers ${
        named.size
          ? [...named]
              .sort()
              .map((t) => `_LOD${t}`)
              .join(", ")
          : "(none named)"
      }; expected _LOD0.._LOD${TIER_COUNT - 1}`,
      `name each mesh object <part>_LOD0 (finest) to <part>_LOD${TIER_COUNT - 1}; unsuffixed meshes appear in every tier`,
    );
  return tiers;
}

function checkTierOrder(triangles: number[], add: AddFinding) {
  for (let t = 1; t < triangles.length; t++)
    if (triangles[t] > triangles[t - 1])
      add(
        "structure.tier_order",
        `tier ${t} has ${triangles[t]} triangles, more than tier ${t - 1}'s ${triangles[t - 1]}`,
        "order tiers finest first: _LOD0 is the most detailed",
      );
}

const trsOf = (m: Mat4, add: AddFinding, what: string): Trs => {
  const trs = decomposeTrs(m);
  if (trs) return trs;
  add(
    "structure.scale",
    `${what} has shear, mirroring or nonuniform scale`,
    "apply transforms in Blender before export",
  );
  return { t: [m[12], m[13], m[14]], r: [0, 0, 0, 1], s: [1, 1, 1] };
};

// ---------------------------------------------------------------- skeleton

export interface SkeletonLayout {
  /** Scene node per joint, in bundle order. */
  nodes: number[];
  joints: { name: string; parent: number }[];
  /** Per joint: the rest transform from the parent joint's space to the glTF parent's. */
  prefix: Mat4[];
}

/**
 * The skin's joints, minus unweighted `_leaf` / `_leaf_l|r` joints with nothing under them
 * (unless `keep` names them: a body keeps every joint of its skeleton),
 * parents before children. Non-joint ancestors (an armature object, the
 * basis) fold into each top joint's transform.
 */
export function skeletonLayout(
  scene: Scene,
  add: AddFinding,
  keep: ReadonlySet<string> = new Set(),
): SkeletonLayout | null {
  if (scene.skins.length !== 1) {
    add(
      "structure.skin_count",
      `${scene.skins.length} skins; a skinned appearance has exactly one`,
      "join every deforming mesh to one armature",
    );
    return null;
  }
  const skin = scene.skins[0];
  const isJoint = new Set(skin.joints);
  const weighted = new Set<number>();
  const anchored = new Set<number>();
  for (const node of scene.nodes.filter((n) => n.live)) {
    if (node.mesh !== null && node.skin === 0)
      for (const primitive of scene.meshes[node.mesh]?.primitives ?? []) {
        if (!primitive.joints || !primitive.weights) continue;
        for (let i = 0; i < primitive.joints.length; i++)
          if (primitive.weights[i] > 0) weighted.add(skin.joints[primitive.joints[i]]);
      }
    if (!isJoint.has(node.index) && node.skin === null) {
      const joint = nearestAncestor(scene, node.index, (n) => isJoint.has(n.index));
      if (joint >= 0) anchored.add(joint);
    }
  }
  const kept = new Set(skin.joints);
  let changed = true;
  while (changed) {
    changed = false;
    for (const j of kept) {
      const node = scene.nodes[j];
      const hasKeptChild = node.children.some((c) => kept.has(c));
      if (
        /_leaf(_[lr])?$/.test(node.name) &&
        !keep.has(node.name) &&
        !weighted.has(j) &&
        !anchored.has(j) &&
        !hasKeptChild
      ) {
        kept.delete(j);
        changed = true;
      }
    }
  }
  const order: number[] = [];
  const visit = (index: number) => {
    if (kept.has(index)) order.push(index);
    for (const child of scene.nodes[index].children) visit(child);
  };
  for (const root of scene.roots) visit(root);
  const slotOf = new Map(order.map((node, i) => [node, i]));
  if (order.length > 256)
    add("structure.skeleton", `${order.length} joints; at most 256`, "reduce the deform hierarchy");
  const joints = order.map((node) => {
    const parentNode = nearestAncestor(scene, node, (n) => kept.has(n.index));
    return { name: scene.nodes[node].name, parent: parentNode < 0 ? -1 : slotOf.get(parentNode)! };
  });
  const names = new Set(joints.map((j) => j.name));
  if (names.size !== joints.length)
    add("structure.skeleton", "joint names repeat", "name every joint uniquely");
  const prefix = order.map((node) => {
    const parentJoint = nearestAncestor(scene, node, (n) => kept.has(n.index));
    const gltfParent = scene.nodes[node].parent;
    const parentJointWorld = parentJoint < 0 ? mat4.create() : scene.nodes[parentJoint].world;
    const gltfParentWorld = gltfParent < 0 ? scene.basis.matrix : scene.nodes[gltfParent].world;
    return mul(inverse(parentJointWorld), gltfParentWorld);
  });
  return { nodes: order, joints, prefix };
}

/** Joint order of `layout` mapped onto a reference joint list by name; null when they differ. */
export function jointRemap(
  layout: SkeletonLayout,
  reference: { name: string; parent: number }[],
  add: AddFinding,
): number[] | null {
  const index = new Map(reference.map((j, i) => [j.name, i]));
  const parentName = (list: { name: string; parent: number }[], j: { parent: number }) =>
    j.parent < 0 ? null : list[j.parent].name;
  const mismatched =
    reference.length !== layout.joints.length ||
    layout.joints.some((j) => {
      const r = index.get(j.name);
      return (
        r === undefined || parentName(reference, reference[r]) !== parentName(layout.joints, j)
      );
    });
  if (mismatched) {
    const missing = reference
      .filter((j) => !layout.joints.some((l) => l.name === j.name))
      .map((j) => j.name);
    const extra = layout.joints.filter((j) => !index.has(j.name)).map((j) => j.name);
    add(
      "structure.skeleton",
      `joints differ from the skeleton's clips (missing ${missing.slice(0, 5).join(", ") || "none"}; extra ${extra.slice(0, 5).join(", ") || "none"}; or a parent differs)`,
      "export the body on the skeleton's rig, unchanged",
    );
    return null;
  }
  return layout.joints.map((j) => index.get(j.name)!);
}

export interface BuiltSkinned {
  joints: Joint[];
  tiers: MeshData[];
  materials: Material[];
  textures: Texture[];
  sockets: Socket[];
}

export function buildSkinned(
  scene: Scene,
  label: string,
  reference: { name: string; parent: number }[] | null,
): { built: BuiltSkinned | null; findings: Finding[] } {
  const findings: Finding[] = [];
  const add = collector(findings, label);
  const layout = skeletonLayout(scene, add, new Set(reference?.map((j) => j.name)));
  if (!layout) return { built: null, findings };
  const remap = reference ? jointRemap(layout, reference, add) : layout.joints.map((_, i) => i);
  if (!remap) return { built: null, findings };
  const jointOfNode = new Map(layout.nodes.map((node, i) => [node, remap[i]]));
  const skin = scene.skins[0];
  const basisInverse = inverse(scene.basis.matrix);

  const joints: Joint[] = layout.nodes
    .map((node, i) => {
      const n = scene.nodes[node];
      const local = mul(layout.prefix[i], trsMatrix(n.local));
      const slot = skin.joints.indexOf(node);
      const joint: Joint = {
        name: n.name,
        parent: layout.joints[i].parent < 0 ? -1 : remap[layout.joints[i].parent],
        bind: trsOf(local, add, `joint "${n.name}"`),
        inverse_bind: mul(skin.inverseBinds[slot], basisInverse),
      };
      return { at: remap[i], joint };
    })
    .sort((a, b) => a.at - b.at)
    .map((entry) => entry.joint);

  const materials = new MaterialTable();
  const tiers = tierParts(scene, add, (nodeIndex, primitive) => {
    const node = scene.nodes[nodeIndex];
    const material = materials.slot(scene.materials[primitive.material]);
    if (node.skin === 0) {
      if (!primitive.joints || !primitive.weights) {
        add(
          "structure.attributes",
          `mesh "${node.name}" is skinned but lacks JOINTS_0/WEIGHTS_0`,
          "export skin weights",
        );
        return null;
      }
      for (let v = 0; v < primitive.weights.length; v += 4) {
        const w = primitive.weights.subarray(v, v + 4);
        const sum = w[0] + w[1] + w[2] + w[3];
        if (Math.abs(sum - 1) > 1e-3 || w.some((x) => x < 0)) {
          add(
            "structure.weights",
            `mesh "${node.name}" vertex ${v / 4} weights sum to ${sum.toFixed(4)}`,
            "normalise all weights in Blender",
          );
          return null;
        }
      }
      const slots = skin.joints.map((jointNode) => jointOfNode.get(jointNode) ?? 0);
      return { primitive, material, transform: scene.basis.matrix, slots };
    }
    if (node.skin !== null) return null;
    const jointNode = nearestAncestor(scene, nodeIndex, (n) => jointOfNode.has(n.index));
    if (jointNode < 0) {
      add(
        "structure.unskinned_mesh",
        `mesh "${node.name}" is neither skinned nor parented to a joint`,
        "skin it, or parent it to a bone so the bake skins it rigidly",
      );
      return null;
    }
    const joint = jointOfNode.get(jointNode)!;
    // Rigid kit: p_bind = inverse(W_joint · IB_joint) · W_node · p, all in engine space.
    const transform = mul(
      inverse(mul(scene.nodes[jointNode].world, joints[joint].inverse_bind)),
      node.world,
    );
    return { primitive, material, transform, rigidJoint: joint };
  });
  const meshes = tiers.map((parts) => mergeParts(parts, true));
  checkTierOrder(meshes.map(triangleCount), add);

  const sockets: Socket[] = [];
  for (const node of scene.nodes) {
    if (
      !node.live ||
      node.mesh !== null ||
      jointOfNode.has(node.index) ||
      skin.joints.includes(node.index)
    )
      continue;
    const jointNode = nearestAncestor(scene, node.index, (n) => jointOfNode.has(n.index));
    if (jointNode < 0) continue;
    sockets.push({
      name: node.name,
      joint: jointOfNode.get(jointNode)!,
      offset: trsOf(
        mul(inverse(scene.nodes[jointNode].world), node.world),
        add,
        `socket "${node.name}"`,
      ),
    });
  }
  return {
    built: {
      joints,
      tiers: meshes,
      materials: materials.materials,
      textures: materials.textures,
      sockets,
    },
    findings,
  };
}

// ---------------------------------------------------------------- clips

function sampleChannel(
  times: Float32Array,
  values: Float32Array,
  width: number,
  step: boolean,
  t: number,
): number[] {
  const n = times.length;
  if (t <= times[0] || n === 1) return Array.from(values.subarray(0, width));
  if (t >= times[n - 1]) return Array.from(values.subarray((n - 1) * width, n * width));
  let hi = 1;
  while (times[hi] < t) hi++;
  const lo = hi - 1;
  const a = values.subarray(lo * width, lo * width + width);
  const b = values.subarray(hi * width, hi * width + width);
  if (step) return Array.from(a);
  const w = (t - times[lo]) / (times[hi] - times[lo]);
  if (width === 4)
    return quat.slerp(quat.create(), Array.from(a) as Quat, Array.from(b) as Quat, w);
  return Array.from(a, (x, i) => x + (b[i] - x) * w);
}

export function buildClips(
  scene: Scene,
  label: string,
  id: string,
  sampleHz: number,
  declared: Record<string, ClipDeclaration>,
): { built: SkeletonClips | null; findings: Finding[] } {
  const findings: Finding[] = [];
  const add = collector(findings, label);
  const layout = skeletonLayout(scene, add);
  if (!layout) return { built: null, findings };
  const clips: Clip[] = [];
  const names = scene.animations.map((a) => a.name);
  for (const name of names)
    if (typeof declared[name]?.loop !== "boolean")
      add(
        "structure.loop_flags",
        `clip "${name}" has no explicit loop flag`,
        `declare "${name}": {"loop": true|false} in the catalog skeleton entry`,
      );
  for (const name of Object.keys(declared))
    if (!names.includes(name))
      add(
        "structure.clips",
        `declared clip "${name}" is not in the source`,
        "export it, or remove the declaration",
      );

  for (const animation of scene.animations) {
    const declaration = declared[animation.name];
    if (typeof declaration?.loop !== "boolean") continue;
    const duration = Math.max(
      0,
      ...animation.channels.map((c) => c.times[c.times.length - 1] ?? 0),
    );
    const frames = Math.max(1, Math.round(duration * sampleHz) + 1);
    const at = (f: number) => Math.min(duration, f / sampleHz);
    const rotationModes = new Uint8Array(layout.nodes.length);
    const translationModes = new Uint8Array(layout.nodes.length);
    const rotations: number[] = [];
    const translations: number[] = [];
    layout.nodes.forEach((node, j) => {
      const prefix = layout.prefix[j];
      const prefixTrs = decomposeTrs(prefix) ?? {
        t: [0, 0, 0] as Vec3,
        r: [0, 0, 0, 1] as Quat,
        s: [1, 1, 1] as Vec3,
      };
      const rotation = animation.channels.find((c) => c.node === node && c.path === "rotation");
      const translation = animation.channels.find(
        (c) => c.node === node && c.path === "translation",
      );
      if (rotation) {
        let previous: Quat | null = null;
        const samples: number[] = [];
        for (let f = 0; f < frames; f++) {
          let q = quat.normalize(
            quat.create(),
            quat.multiply(
              quat.create(),
              prefixTrs.r,
              sampleChannel(rotation.times, rotation.values, 4, rotation.step, at(f)) as Quat,
            ),
          );
          if (
            previous &&
            q[0] * previous[0] + q[1] * previous[1] + q[2] * previous[2] + q[3] * previous[3] < 0
          )
            q = [-q[0], -q[1], -q[2], -q[3]];
          previous = q;
          samples.push(...q.map((x) => Math.round(Math.max(-1, Math.min(1, x)) * 32767)));
        }
        const constant = samples.every((x, i) => x === samples[i % 4]);
        rotationModes[j] = constant ? CHANNEL_CONSTANT : CHANNEL_ANIMATED;
        rotations.push(...(constant ? samples.slice(0, 4) : samples));
      }
      if (translation) {
        const samples: number[] = [];
        for (let f = 0; f < frames; f++)
          samples.push(
            ...pointAt(
              prefix,
              sampleChannel(
                translation.times,
                translation.values,
                3,
                translation.step,
                at(f),
              ) as Vec3,
            ),
          );
        const f32 = Array.from(Float32Array.from(samples));
        const constant = f32.every((x, i) => x === f32[i % 3]);
        translationModes[j] = constant ? CHANNEL_CONSTANT : CHANNEL_ANIMATED;
        translations.push(...(constant ? f32.slice(0, 3) : f32));
      }
      if (!rotation) rotationModes[j] = CHANNEL_ABSENT;
      if (!translation) translationModes[j] = CHANNEL_ABSENT;
    });
    clips.push({
      name: animation.name,
      loop: declaration.loop,
      duration,
      frames,
      ...(declaration.stride_m !== undefined ? { stride_m: declaration.stride_m } : {}),
      markers: Object.entries(declaration.markers ?? {})
        .map(([name, phase]) => ({ name, phase }))
        .sort((a, b) => a.phase - b.phase || a.name.localeCompare(b.name)),
      rotation_modes: rotationModes,
      translation_modes: translationModes,
      rotations: Int16Array.from(rotations),
      translations: Float32Array.from(translations),
    });
  }
  clips.sort((a, b) => a.name.localeCompare(b.name));
  return {
    built: { kind: "clips", id, joints: layout.joints, sample_hz: sampleHz, clips },
    findings,
  };
}

// ---------------------------------------------------------------- articulated

export function buildArticulated(
  scene: Scene,
  label: string,
): {
  built: { nodes: ArticulatedNode[]; materials: Material[]; textures: Texture[] } | null;
  findings: Finding[];
} {
  const findings: Finding[] = [];
  const add = collector(findings, label);
  if (scene.skins.length)
    add(
      "structure.skin_count",
      `${scene.skins.length} skin(s); vehicles articulate by named nodes, not skins`,
      "remove the armature and parent parts to empties",
    );
  const roots = scene.roots.filter((r) => scene.nodes[r]);
  if (roots.length !== 1 || scene.nodes[roots[0]].mesh !== null) {
    add(
      "structure.root",
      `${roots.length} scene root(s); expected one empty at the model origin`,
      "parent everything to one empty at the hull origin",
    );
    return { built: null, findings };
  }
  const order: number[] = [];
  const visit = (index: number) => {
    if (scene.nodes[index].mesh === null) order.push(index);
    for (const child of scene.nodes[index].children) visit(child);
  };
  visit(roots[0]);
  const slotOf = new Map(order.map((node, i) => [node, i]));
  const seen = new Set<string>();
  for (const node of order) {
    const name = scene.nodes[node].name;
    if (seen.has(name))
      add("nodes.duplicate", `node name "${name}" repeats`, "name every empty uniquely");
    seen.add(name);
  }
  // Node frames are engine-aligned: each glTF frame conjugated by the basis,
  // so a part authored unrotated in Blender yaws about its local +Z.
  const basisInverse = inverse(scene.basis.matrix);
  const frame = (node: number) => mul(scene.nodes[node].world, basisInverse);
  const materials = new MaterialTable();
  const partsByNode = order.map(() => Array.from({ length: TIER_COUNT }, () => [] as MeshPart[]));
  const tiers = tierParts(scene, add, (nodeIndex, primitive) => {
    const owner = nearestAncestor(scene, nodeIndex, (n) => n.mesh === null);
    const transform = mul(inverse(frame(owner)), scene.nodes[nodeIndex].world);
    return {
      primitive,
      material: materials.slot(scene.materials[primitive.material]),
      transform,
      rigidJoint: slotOf.get(owner)!,
    };
  });
  tiers.forEach((parts, t) => {
    for (const part of parts)
      partsByNode[part.rigidJoint!][t].push({ ...part, rigidJoint: undefined });
  });
  const nodes: ArticulatedNode[] = order.map((node, i) => {
    const n = scene.nodes[node];
    const parent = nearestAncestor(scene, node, (x) => x.mesh === null);
    const local = parent < 0 ? frame(node) : mul(inverse(frame(parent)), frame(node));
    const extras: Record<string, number> = {};
    for (const [key, value] of Object.entries(n.extras))
      if (typeof value === "number") extras[key] = value;
    return {
      name: n.name,
      parent: parent < 0 ? -1 : slotOf.get(parent)!,
      pivot: [n.world[12], n.world[13], n.world[14]],
      bind: trsOf(local, add, `node "${n.name}"`),
      extras,
      tiers: partsByNode[i].map((parts) => mergeParts(parts, false)),
    };
  });
  checkTierOrder(
    Array.from({ length: TIER_COUNT }, (_, t) =>
      nodes.reduce((sum, n) => sum + triangleCount(n.tiers[t]), 0),
    ),
    add,
  );
  return {
    built: { nodes, materials: materials.materials, textures: materials.textures },
    findings,
  };
}

// ---------------------------------------------------------------- static

export function buildStaticState(
  scene: Scene,
  label: string,
  materials: MaterialTable,
): {
  tiers: MeshData[] | null;
  findings: Finding[];
} {
  const findings: Finding[] = [];
  const add = collector(findings, label);
  if (scene.skins.length)
    add(
      "structure.skin_count",
      `${scene.skins.length} skin(s); buildings are static`,
      "apply the armature and remove it",
    );
  const tiers = tierParts(scene, add, (nodeIndex, primitive) => ({
    primitive,
    material: materials.slot(scene.materials[primitive.material]),
    transform: scene.nodes[nodeIndex].world,
  }));
  const meshes = tiers.map((parts) => mergeParts(parts, false));
  checkTierOrder(meshes.map(triangleCount), add);
  return { tiers: meshes, findings };
}

// ---------------------------------------------------------------- kit

/** A module id: what a kit's root empties are named and a template set's
 *  `modules` list by. */
export const MODULE_ID = /^[a-z0-9_]+$/;

/**
 * A city kit's modules (`blender/city/README.md`): every root-level empty is
 * one module, named by its id, and the meshes under it are its geometry in
 * that empty's own frame, so a script may lay its modules out side by side.
 * Each module carries all four tiers; an unsuffixed mesh is in every one.
 * Modules come back in id order.
 */
export function buildKitModules(
  scene: Scene,
  label: string,
  materials: MaterialTable,
): { modules: { name: string; tiers: MeshData[] }[]; findings: Finding[] } {
  const findings: Finding[] = [];
  const add = collector(findings, label);
  if (scene.skins.length)
    add(
      "structure.skin_count",
      `${scene.skins.length} skin(s); a kit's modules are static`,
      "apply the armature and remove it",
    );
  const basisInverse = inverse(scene.basis.matrix);
  const modules: { name: string; tiers: MeshData[] }[] = [];
  const seen = new Set<string>();
  for (const root of scene.roots) {
    const node = scene.nodes[root];
    if (node.mesh !== null) {
      add(
        "kit.module",
        `mesh "${node.name}" is at the root, outside any module`,
        "parent each mesh to its module's empty, a root-level empty named by the module's id",
      );
      continue;
    }
    if (!MODULE_ID.test(node.name) || seen.has(node.name)) {
      add(
        "kit.module",
        seen.has(node.name)
          ? `module "${node.name}" appears twice`
          : `root empty "${node.name}" is not a module id (lowercase letters, digits and underscores)`,
        "name each root-level empty by its module's own id",
      );
      continue;
    }
    seen.add(node.name);
    // The module's frame is engine-aligned, as an articulated node's is.
    const toModule = inverse(mul(node.world, basisInverse));
    const tiers: MeshPart[][] = Array.from({ length: TIER_COUNT }, () => []);
    const visit = (index: number) => {
      const child = scene.nodes[index];
      const tier = meshTier(child.name);
      for (const primitive of child.mesh === null
        ? []
        : (scene.meshes[child.mesh]?.primitives ?? [])) {
        const part: MeshPart = {
          primitive,
          material: materials.slot(scene.materials[primitive.material]),
          transform: mul(toModule, child.world),
        };
        if (tier === null) tiers.forEach((t) => t.push(part));
        else if (tier < TIER_COUNT) tiers[tier].push(part);
        else
          add(
            "structure.tier_count",
            `module "${node.name}": mesh "${child.name}" names tier ${tier}; tiers are _LOD0.._LOD${TIER_COUNT - 1}`,
            `name each mesh <part>_LOD0 (finest) to <part>_LOD${TIER_COUNT - 1}`,
          );
      }
      for (const next of child.children) visit(next);
    };
    for (const child of node.children) visit(child);
    const meshes = tiers.map((parts) => mergeParts(parts, false));
    const empty = meshes.flatMap((mesh, t) => (triangleCount(mesh) ? [] : [`_LOD${t}`]));
    if (empty.length)
      add(
        "structure.tier_count",
        `module "${node.name}" has no geometry in ${empty.join(", ")}; every module has all ${TIER_COUNT} tiers`,
        `give the module a mesh for each of _LOD0.._LOD${TIER_COUNT - 1}; an unsuffixed mesh is in every tier`,
      );
    checkTierOrder(meshes.map(triangleCount), (code, message, fix) =>
      add(code, `module "${node.name}": ${message}`, fix),
    );
    modules.push({ name: node.name, tiers: meshes });
  }
  if (!modules.length)
    add(
      "kit.module",
      "no module",
      "add a root-level empty per module, named by its id, with its meshes under it",
    );
  modules.sort((a, b) => (a.name < b.name ? -1 : 1));
  return { modules, findings };
}
