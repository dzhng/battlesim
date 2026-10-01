// Grass kinds: the clump of blades the battle's grass field instances. A
// grass kind is scenery like any prop or tree (`scenery.ts`, kind "grass"):
// one GLB, four LOD tiers, baked, loaded and shown in the workbench through
// the same path. Where clumps grow, how many and how they sway is the
// renderer's field; the clump is the art.
//
// The clump is generated, not modelled: `grassClumpGlb` writes the GLB from
// the catalog entry's `grass` spec (`asset grass`), deterministically, so the
// source's hash is the spec's.
//
// The art is blade strips the field bends in the wind, so a grass kind's
// tiers keep one layout (`grassStripFindings`): every tier is the same blades,
// one after another, each a left and a right vertex at the lower end of each
// of its segments and then one tip vertex, indexed as `grassStripIndices`
// gives. Per vertex, TEXCOORD_0 holds the blade's phase (u) and the fraction
// of its height (v: 0 at the root, 1 at the tip), the wind's weight; COLOR_0
// is the blade's colour, whose mean over the clump stands for the ground it
// grows on (the field tints each clump by its own ground, relative to it).

import { vec3, type Vec3 } from "math";
import { mulberry32, random } from "math/random";
import { encodeGlb, type GltfJson } from "./glb.ts";
import type { Finding, GrassSpec, MeshData } from "./schema.ts";

/** Segments per blade in each LOD tier, finest first. */
export const GRASS_SEGMENTS = [4, 3, 2, 1] as const;
/** Blades a clump may hold: the field pads every kind to the most. */
export const GRASS_MAX_BLADES = 8;

/** Independent clump and patch height variation, shared with the field shader. */
export const GRASS_HEIGHT_VARIATION = [0.8, 1.2] as const;
export const GRASS_PATCH_HEIGHT_VARIATION = [0.7, 1.2] as const;

/** Highest vertex in any tier the field can draw, including imported clumps. */
export function grassMeshHeight(tiers: readonly MeshData[]): number {
  let maximum = 0;
  for (const mesh of tiers)
    for (let i = 2; i < mesh.positions.length; i += 3)
      maximum = Math.max(maximum, mesh.positions[i]);
  return maximum;
}

/** Maximum drawn height, using the shader's f32 inputs and multiplication. */
export function maxFieldGrassHeight(sourceHeight: number, biomeScale: number): number {
  let height = Math.fround(Math.fround(sourceHeight) * Math.fround(biomeScale));
  height = Math.fround(height * Math.fround(GRASS_HEIGHT_VARIATION[1]));
  return Math.fround(height * Math.fround(GRASS_PATCH_HEIGHT_VARIATION[1]));
}

/** Vertices of one blade of `segments` segments. */
export const grassBladeVertices = (segments: number) => 2 * segments + 1;

/** The canonical index list for `blades` blades of `segments` segments. */
export function grassStripIndices(blades: number, segments: number): Uint16Array {
  const per = grassBladeVertices(segments);
  const out = new Uint16Array(blades * (2 * segments - 1) * 3);
  let i = 0;
  for (let b = 0; b < blades; b++) {
    const o = b * per;
    for (let k = 0; k < segments - 1; k++) {
      const [l0, r0, l1, r1] = [o + 2 * k, o + 2 * k + 1, o + 2 * k + 2, o + 2 * k + 3];
      out.set([l0, r0, l1, r0, r1, l1], i);
      i += 6;
    }
    const last = o + 2 * (segments - 1);
    out.set([last, last + 1, o + per - 1], i);
    i += 3;
  }
  return out;
}

const finding = (message: string, fix: string): Finding => ({
  code: "structure.grass",
  severity: "error",
  message,
  fix,
});

/** Whether a grass kind's built tiers are the field's blade strips. */
export function grassStripFindings(label: string, tiers: readonly MeshData[]): Finding[] {
  const fix =
    "generate the clump with `bun run --cwd web asset -- grass`, or author blade strips as grass.ts describes";
  const blades = tiers[0]
    ? tiers[0].positions.length / 3 / grassBladeVertices(GRASS_SEGMENTS[0])
    : 0;
  if (!(Number.isInteger(blades) && blades >= 1 && blades <= GRASS_MAX_BLADES))
    return [
      finding(
        `${label}: LOD0 is not 1 to ${GRASS_MAX_BLADES} blades of ${GRASS_SEGMENTS[0]} segments`,
        fix,
      ),
    ];
  for (let t = 0; t < GRASS_SEGMENTS.length; t++) {
    const mesh = tiers[t];
    const per = grassBladeVertices(GRASS_SEGMENTS[t]);
    const canonical = grassStripIndices(blades, GRASS_SEGMENTS[t]);
    if (
      !mesh ||
      mesh.positions.length / 3 !== blades * per ||
      mesh.indices.length !== canonical.length ||
      mesh.indices.some((v, i) => v !== canonical[i])
    )
      return [
        finding(
          `${label}: LOD${t} is not the same ${blades} blades as ${GRASS_SEGMENTS[t]}-segment strips`,
          fix,
        ),
      ];
    for (let b = 0; b < blades; b++) {
      // Screen-width expansion can be arbitrarily large at distant cameras;
      // blade width must stay horizontal to preserve the field height bound.
      for (let k = 0; k < GRASS_SEGMENTS[t]; k++) {
        const left = (b * per + 2 * k) * 3 + 2;
        if (Math.abs(mesh.positions[left] - mesh.positions[left + 3]) > 1e-6)
          return [finding(`${label}: LOD${t} blade ${b} has a vertical width component`, fix)];
      }
      const root = mesh.uvs[b * per * 2 + 1];
      const tip = mesh.uvs[(b * per + per - 1) * 2 + 1];
      if (root !== 0 || Math.abs(tip - 1) > 1e-6)
        return [
          finding(
            `${label}: LOD${t} blade ${b} runs v ${root} to ${tip}, not 0 at the root to 1 at the tip`,
            fix,
          ),
        ];
    }
  }
  return [];
}

/** What makes a spec ungeneratable; the generator refuses on any. */
export function grassSpecErrors(spec: GrassSpec): string[] {
  const out: string[] = [];
  const range = (what: string, r: readonly number[], lo: number, hi: number) => {
    if (!(Array.isArray(r) && r.length === 2 && r[0] >= lo && r[1] >= r[0] && r[1] <= hi))
      out.push(`${what} must be [low, high] within [${lo}, ${hi}]`);
  };
  if (!(Number.isInteger(spec.blades) && spec.blades >= 1 && spec.blades <= GRASS_MAX_BLADES))
    out.push(`blades must be a whole number within [1, ${GRASS_MAX_BLADES}]`);
  range("height_m", spec.height_m, 0.02, 3);
  range("lean", spec.lean, 0, 0.8);
  if (!(spec.radius_m >= 0 && spec.radius_m <= 1)) out.push("radius_m must be within [0, 1]");
  if (!(spec.width_m > 0 && spec.width_m <= 0.2)) out.push("width_m must be within (0, 0.2]");
  if (!(spec.jitter >= 0 && spec.jitter <= 0.5)) out.push("jitter must be within [0, 0.5]");
  if (spec.head && !(spec.head.from > 0 && spec.head.from < 1 && spec.head.width >= 1))
    out.push("head.from must be within (0, 1) and head.width at least 1");
  if (spec.head && !(spec.head.chance >= 0 && spec.head.chance <= 1))
    out.push("head.chance must be within [0, 1]");
  if (spec.dry && !(spec.dry.chance >= 0 && spec.dry.chance <= 1))
    out.push("dry.chance must be within [0, 1]");
  const colours = [spec.colors?.root, spec.colors?.mid, spec.colors?.tip];
  if (spec.dry) colours.push(spec.dry.colors?.root, spec.dry.colors?.mid, spec.dry.colors?.tip);
  if (!colours.every((c) => Array.isArray(c) && c.length === 3 && c.every((v) => v >= 0 && v <= 1)))
    out.push("colors (and dry.colors) .root, .mid and .tip must be sRGB [r, g, b] in [0, 1]");
  return out;
}

interface Blade {
  root: Vec3;
  /** Unit, horizontal: the blade's width runs along it. */
  across: Vec3;
  /** Unit, horizontal: the tip leans this way. */
  lean: Vec3;
  height: number;
  leanFraction: number;
  phase: number;
  shade: number;
  /** A dry stem, coloured `dry.colors`. */
  dry: boolean;
  /** Carries the seed head. */
  head: boolean;
}

function blades(spec: GrassSpec): Blade[] {
  const rng = mulberry32.create(spec.seed);
  const next = () => mulberry32.sample(rng);
  return Array.from({ length: spec.blades }, (_, b) => {
    // Roots spread evenly in angle, jittered, so a clump reads as a tuft;
    // blades lean mostly outward from its centre.
    const angle = ((b + random.float(next, 0.1, 0.9)) / spec.blades) * Math.PI * 2;
    const r = spec.radius_m * Math.sqrt(random.float(next, 0.15, 1));
    const yaw = random.float(next, 0, Math.PI * 2);
    const leanYaw = angle + random.float(next, -0.6, 0.6);
    return {
      root: vec3.fromValues(Math.cos(angle) * r, Math.sin(angle) * r, 0),
      across: vec3.fromValues(Math.cos(yaw), Math.sin(yaw), 0),
      lean: vec3.fromValues(Math.cos(leanYaw), Math.sin(leanYaw), 0),
      height: random.float(next, spec.height_m[0], spec.height_m[1]),
      leanFraction: random.float(next, spec.lean[0], spec.lean[1]),
      phase: next(),
      shade: 1 + random.float(next, -spec.jitter, spec.jitter),
      dry: next() < (spec.dry?.chance ?? 0),
      head: next() < (spec.head?.chance ?? 0),
    };
  });
}

function colourAt(spec: GrassSpec, blade: Blade, t: number): number[] {
  const { root, mid, tip } = blade.dry && spec.dry ? spec.dry.colors : spec.colors;
  const shade = blade.shade;
  const [a, b, u] = t < 0.5 ? [root, mid, t / 0.5] : [mid, tip, (t - 0.5) / 0.5];
  return [0, 1, 2].map((c) => Math.max(0, Math.min(1, (a[c] + (b[c] - a[c]) * u) * shade)));
}

/** Width at `t` of the height: tapering to the tip, swelling at a head. */
function widthAt(spec: GrassSpec, blade: Blade, t: number): number {
  let w = spec.width_m * (1 - 0.8 * t ** 1.6);
  if (spec.head && blade.head && t > spec.head.from) {
    const u = (t - spec.head.from) / (1 - spec.head.from);
    w *= 1 + (spec.head.width - 1) * Math.sin(Math.PI * Math.min(1, u * 1.15));
  }
  return w;
}

/** A quadratic lean: the tip goes out by `leanFraction` of the height, and
 *  the spine keeps roughly the blade's length. */
function spineAt(out: Vec3, blade: Blade, t: number): Vec3 {
  const out2 = blade.leanFraction * t * t;
  vec3.scaleAndAdd(out, blade.root, blade.lean, blade.height * out2);
  out[2] += blade.height * t * Math.sqrt(Math.max(0.2, 1 - out2 * out2));
  return out;
}

/** One tier's strips, in engine space: positions, normals, uvs, colours. */
function strips(spec: GrassSpec, list: readonly Blade[], segments: number) {
  const per = grassBladeVertices(segments);
  const count = list.length * per;
  const positions = new Float32Array(count * 3);
  const normals = new Float32Array(count * 3);
  const uvs = new Float32Array(count * 2);
  const colors = new Float32Array(count * 4);
  const [spine, ahead, tangent, normal, point] = [0, 0, 0, 0, 0].map(() => vec3.create());
  list.forEach((blade, b) => {
    for (let k = 0; k <= segments; k++) {
      const tip = k === segments;
      const t = k / segments;
      spineAt(spine, blade, t);
      if (tip) vec3.subtract(tangent, spine, spineAt(ahead, blade, 0.99));
      else vec3.subtract(tangent, spineAt(ahead, blade, t + 0.01), spine);
      vec3.normalize(tangent, tangent);
      vec3.normalize(normal, vec3.cross(normal, blade.across, tangent));
      const colour = colourAt(spec, blade, t);
      (tip ? [0] : [-0.5, 0.5]).forEach((side, s) => {
        const v = b * per + (tip ? per - 1 : 2 * k + s);
        vec3.scaleAndAdd(point, spine, blade.across, side * widthAt(spec, blade, t));
        positions.set(point, v * 3);
        normals.set(normal, v * 3);
        uvs.set([blade.phase, t], v * 2);
        colors.set([...colour, 1], v * 4);
      });
    }
  });
  return { positions, normals, uvs, colors, indices: grassStripIndices(list.length, segments) };
}

/** Engine space (Z up) to glTF's (Y up): the inverse of the bake's basis. */
function toGltf(v: Float32Array): Float32Array {
  const out = new Float32Array(v.length);
  for (let i = 0; i < v.length; i += 3) out.set([v[i], v[i + 2], -v[i + 1]], i);
  return out;
}

/** The clump's GLB: one mesh per LOD tier (`clump_LOD<n>`), finest first. */
export function grassClumpGlb(name: string, spec: GrassSpec): Uint8Array {
  const errors = grassSpecErrors(spec);
  if (errors.length) throw new Error(`grass ${name}: ${errors.join("; ")}`);
  const list = blades(spec);
  const chunks: Uint8Array[] = [];
  let length = 0;
  const json: GltfJson = {
    asset: { version: "2.0", generator: `scene-assets grass (${name})` },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ name: "grass", children: GRASS_SEGMENTS.map((_, t) => t + 1) }],
    meshes: [],
    accessors: [],
    bufferViews: [],
    materials: [
      {
        name: "blade",
        pbrMetallicRoughness: {
          baseColorFactor: [1, 1, 1, 1],
          metallicFactor: 0,
          roughnessFactor: 0.9,
        },
        doubleSided: true,
      },
    ],
  };
  const accessor = (data: Float32Array | Uint16Array, type: string, extra: GltfJson = {}) => {
    const bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    const padded = new Uint8Array(Math.ceil(bytes.byteLength / 4) * 4);
    padded.set(bytes);
    json.bufferViews.push({ buffer: 0, byteOffset: length, byteLength: bytes.byteLength });
    chunks.push(padded);
    length += padded.byteLength;
    const width = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4 }[type]!;
    json.accessors.push({
      bufferView: json.bufferViews.length - 1,
      componentType: data instanceof Float32Array ? 5126 : 5123,
      type,
      count: data.length / width,
      ...extra,
    });
    return json.accessors.length - 1;
  };
  GRASS_SEGMENTS.forEach((segments, t) => {
    const s = strips(spec, list, segments);
    const positions = toGltf(s.positions);
    const min = [0, 1, 2].map((c) => Math.min(...positions.filter((_, i) => i % 3 === c)));
    const max = [0, 1, 2].map((c) => Math.max(...positions.filter((_, i) => i % 3 === c)));
    json.meshes.push({
      name: `clump_LOD${t}`,
      primitives: [
        {
          attributes: {
            POSITION: accessor(positions, "VEC3", { min, max }),
            NORMAL: accessor(toGltf(s.normals), "VEC3"),
            TEXCOORD_0: accessor(s.uvs, "VEC2"),
            COLOR_0: accessor(s.colors, "VEC4"),
          },
          indices: accessor(s.indices, "SCALAR"),
          material: 0,
        },
      ],
    });
    json.nodes.push({ name: `clump_LOD${t}`, mesh: t });
  });
  const bin = new Uint8Array(length);
  let at = 0;
  for (const c of chunks) {
    bin.set(c, at);
    at += c.byteLength;
  }
  return encodeGlb(json, bin);
}
