// The binary bundle container. One file per bundle, named by the sha256 of its
// bytes:
//
//   0  u32  magic "BGAB"
//   4  u32  format version
//   8  u32  header byte length H (a multiple of 4)
//  12  u32  body byte length
//  16  H    header: canonical JSON (sorted keys, Float32-rounded numbers),
//           space padded; every typed array is {"$view": i}
//  16+H     body: the typed arrays, each 4-byte aligned, in view order
//
// The header's `views` lists [type, byteOffset, length] per array. Encoding is
// deterministic, so the same content always has the same hash.

import { sha256Hex } from "./glb.ts";
import { TIER_COUNT, type Bundle, type MeshData } from "./schema.ts";

const MAGIC = 0x42414742; // "BGAB" little-endian
export const FORMAT_VERSION = 1;

type Typed = Float32Array | Int16Array | Uint8Array | Uint16Array | Uint32Array;
const TYPES = {
  f32: Float32Array,
  i16: Int16Array,
  u8: Uint8Array,
  u16: Uint16Array,
  u32: Uint32Array,
} as const;
type TypeName = keyof typeof TYPES;
const typeName = (a: Typed): TypeName =>
  a instanceof Float32Array
    ? "f32"
    : a instanceof Int16Array
      ? "i16"
      : a instanceof Uint8Array
        ? "u8"
        : a instanceof Uint16Array
          ? "u16"
          : "u32";

export function encodeBundle(bundle: Bundle): Uint8Array {
  const arrays: Typed[] = [];
  const canonical = (value: unknown): unknown => {
    if (ArrayBuffer.isView(value)) {
      arrays.push(value as Typed);
      return { $view: arrays.length - 1 };
    }
    if (typeof value === "number") {
      if (!Number.isFinite(value)) throw new Error("bundle holds a nonfinite number");
      return Number.isInteger(value) ? value : Math.fround(value);
    }
    if (Array.isArray(value)) return value.map(canonical);
    if (value && typeof value === "object") {
      const out: Record<string, unknown> = {};
      for (const key of Object.keys(value).sort()) {
        const v = (value as Record<string, unknown>)[key];
        if (v !== undefined) out[key] = canonical(v);
      }
      return out;
    }
    return value;
  };
  const content = canonical(bundle) as Record<string, unknown>;
  let offset = 0;
  const views: [TypeName, number, number][] = arrays.map((a) => {
    const view: [TypeName, number, number] = [typeName(a), offset, a.length];
    offset += Math.ceil(a.byteLength / 4) * 4;
    return view;
  });
  const json = new TextEncoder().encode(JSON.stringify({ content, views }));
  const headerLength = Math.ceil(json.length / 4) * 4;
  const out = new Uint8Array(16 + headerLength + offset);
  const dv = new DataView(out.buffer);
  dv.setUint32(0, MAGIC, true);
  dv.setUint32(4, FORMAT_VERSION, true);
  dv.setUint32(8, headerLength, true);
  dv.setUint32(12, offset, true);
  out.set(json, 16);
  out.fill(0x20, 16 + json.length, 16 + headerLength);
  arrays.forEach((a, i) => {
    out.set(new Uint8Array(a.buffer, a.byteOffset, a.byteLength), 16 + headerLength + views[i][1]);
  });
  return out;
}

export function decodeBundle(bytes: Uint8Array): Bundle {
  if (bytes.byteLength < 16) throw new Error("bundle is truncated");
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (dv.getUint32(0, true) !== MAGIC) throw new Error("not an appearance bundle (bad magic)");
  const version = dv.getUint32(4, true);
  if (version !== FORMAT_VERSION)
    throw new Error(`bundle format ${version}, expected ${FORMAT_VERSION}; re-bake`);
  const headerLength = dv.getUint32(8, true);
  const bodyLength = dv.getUint32(12, true);
  if (16 + headerLength + bodyLength !== bytes.byteLength)
    throw new Error("bundle length does not match its header");
  const { content, views } = JSON.parse(
    new TextDecoder().decode(bytes.subarray(16, 16 + headerLength)),
  ) as {
    content: unknown;
    views: [TypeName, number, number][];
  };
  const body = 16 + headerLength;
  const arrays = views.map(([type, offset, length]) => {
    const Ctor = TYPES[type];
    if (!Ctor) throw new Error(`unknown view type ${type}`);
    const byteLength = length * Ctor.BYTES_PER_ELEMENT;
    if (offset + byteLength > bodyLength) throw new Error("bundle view runs past the body");
    const copy = bytes.slice(body + offset, body + offset + byteLength);
    return new Ctor(copy.buffer, 0, length);
  });
  const revive = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(revive);
    if (value && typeof value === "object") {
      const record = value as Record<string, unknown>;
      if (typeof record.$view === "number") {
        const array = arrays[record.$view];
        if (!array) throw new Error(`missing view ${record.$view}`);
        return array;
      }
      return Object.fromEntries(Object.entries(record).map(([k, v]) => [k, revive(v)]));
    }
    return value;
  };
  const bundle = revive(content) as Bundle;
  assertShape(bundle);
  return bundle;
}

export async function bundleHash(bytes: Uint8Array): Promise<string> {
  return sha256Hex(bytes);
}

function assertMesh(mesh: MeshData, skinned: boolean, where: string) {
  const vertices = mesh.positions?.length / 3;
  const ok =
    mesh.positions instanceof Float32Array &&
    Number.isInteger(vertices) &&
    mesh.normals instanceof Int16Array &&
    mesh.normals.length === vertices * 4 &&
    mesh.uvs instanceof Float32Array &&
    mesh.uvs.length === vertices * 2 &&
    mesh.colors instanceof Uint8Array &&
    mesh.colors.length === vertices * 4 &&
    (mesh.indices instanceof Uint16Array || mesh.indices instanceof Uint32Array) &&
    mesh.indices.length % 3 === 0 &&
    Array.isArray(mesh.draws) &&
    (!skinned ||
      (mesh.joints instanceof Uint8Array &&
        mesh.joints.length === vertices * 4 &&
        mesh.weights instanceof Uint16Array &&
        mesh.weights.length === vertices * 4));
  if (!ok) throw new Error(`${where}: malformed mesh`);
  for (const index of mesh.indices)
    if (index >= vertices) throw new Error(`${where}: index out of range`);
  const drawn = mesh.draws.reduce((n, d) => n + d.count, 0);
  if (drawn !== mesh.indices.length)
    throw new Error(`${where}: draw ranges do not cover the indices`);
}

function assertShape(bundle: Bundle) {
  const tiers = (list: MeshData[], skinned: boolean, where: string) => {
    if (!Array.isArray(list) || list.length !== TIER_COUNT)
      throw new Error(`${where}: expected ${TIER_COUNT} tiers`);
    list.forEach((mesh, t) => assertMesh(mesh, skinned, `${where} tier ${t}`));
  };
  switch (bundle.kind) {
    case "skinned":
      if (
        !Array.isArray(bundle.joints) ||
        !bundle.far_pose ||
        !bundle.corpse_pose ||
        !bundle.bounds
      )
        throw new Error("skinned bundle is incomplete");
      tiers(bundle.tiers, true, "skinned");
      return;
    case "articulated":
      if (!Array.isArray(bundle.nodes) || !bundle.nodes.length || !bundle.bounds)
        throw new Error("articulated bundle is incomplete");
      bundle.nodes.forEach((node) => tiers(node.tiers, false, `node ${node.name}`));
      return;
    case "static":
      if (!Array.isArray(bundle.states) || !bundle.states.length || !bundle.bounds)
        throw new Error("static bundle is incomplete");
      bundle.states.forEach((state) => tiers(state.tiers, false, `state ${state.name}`));
      return;
    case "clips":
      if (!Array.isArray(bundle.joints) || !Array.isArray(bundle.clips))
        throw new Error("clips bundle is incomplete");
      return;
    default:
      throw new Error(`unknown bundle kind ${(bundle as { kind: string }).kind}`);
  }
}
