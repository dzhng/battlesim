// The GLB container, content hashing, and Git LFS pointer detection. The
// container parse is adapted from ~/dev/game's soldier-assets `bake/gltf.mjs`
// (see the reuse manifest); everything else here is ours.

const GLB_MAGIC = 0x46546c67; // 'glTF'
const CHUNK_JSON = 0x4e4f534a; // 'JSON'
const CHUNK_BIN = 0x004e4942; // 'BIN\0'

/** A problem that stops a file being read at all. */
export class AssetError extends Error {
  readonly code: "structure.unreadable" | "structure.lfs_pointer";
  readonly fix: string;
  constructor(code: AssetError["code"], message: string, fix: string) {
    super(message);
    this.code = code;
    this.fix = fix;
  }
}

// oxlint-disable-next-line typescript/no-explicit-any -- glTF JSON is validated field by field
export type GltfJson = Record<string, any>;

export function toBytes(buffer: ArrayBuffer | ArrayBufferView): Uint8Array {
  if (buffer instanceof Uint8Array) return buffer;
  if (buffer instanceof ArrayBuffer) return new Uint8Array(buffer);
  return new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength);
}

const LFS_PREFIX = "version https://git-lfs.github.com/spec/v1";

/** A Git LFS pointer's object id (the content's sha256), or null for real content. */
export function lfsPointerOid(bytes: Uint8Array): string | null {
  if (bytes.byteLength > 1024 || bytes.byteLength < LFS_PREFIX.length) return null;
  const text = new TextDecoder().decode(bytes);
  if (!text.startsWith(LFS_PREFIX)) return null;
  return text.match(/^oid sha256:([0-9a-f]{64})$/m)?.[1] ?? null;
}

/** The exact command that fetches one LFS path; never a bare pull. */
export const lfsPullCommand = (path: string) => `git lfs pull --include="${path}"`;

export function assertNotLfsPointer(bytes: Uint8Array, path: string): void {
  if (lfsPointerOid(bytes) !== null)
    throw new AssetError(
      "structure.lfs_pointer",
      `${path} is a Git LFS pointer, not its content`,
      `run: ${lfsPullCommand(path)}`,
    );
}

export async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

/** The content hash of a file, read from its LFS pointer when only the pointer is checked out. */
export async function contentSha256(bytes: Uint8Array): Promise<string> {
  return lfsPointerOid(bytes) ?? sha256Hex(bytes);
}

/** Split a .glb into its glTF JSON and binary chunk. */
export function parseGlb(
  input: ArrayBuffer | ArrayBufferView,
  path = "input",
): {
  json: GltfJson;
  bin: Uint8Array | null;
} {
  const bytes = toBytes(input);
  assertNotLfsPointer(bytes, path);
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const unreadable = (why: string) =>
    new AssetError("structure.unreadable", `${path}: ${why}`, "export a binary glTF 2.0 (.glb)");
  if (bytes.byteLength < 12 || dv.getUint32(0, true) !== GLB_MAGIC)
    throw unreadable("not a GLB file (bad magic header)");
  if (dv.getUint32(4, true) !== 2) throw unreadable(`GLB version ${dv.getUint32(4, true)}, not 2`);
  let offset = 12;
  let json: GltfJson | null = null;
  let bin: Uint8Array | null = null;
  while (offset + 8 <= bytes.byteLength) {
    const length = dv.getUint32(offset, true);
    const type = dv.getUint32(offset + 4, true);
    const chunk = bytes.subarray(offset + 8, offset + 8 + length);
    if (type === CHUNK_JSON) {
      try {
        json = JSON.parse(new TextDecoder().decode(chunk));
      } catch {
        throw unreadable("JSON chunk does not parse");
      }
    } else if (type === CHUNK_BIN) bin = chunk;
    offset += 8 + length;
  }
  if (!json) throw unreadable("no JSON chunk");
  return { json, bin };
}

/** Pack glTF JSON and one binary chunk into a .glb (used by tests and tools). */
export function encodeGlb(json: GltfJson, bin: Uint8Array): Uint8Array {
  const pad = (n: number) => (4 - (n % 4)) % 4;
  const body = { ...json, buffers: [{ byteLength: bin.byteLength }] };
  const jsonBytes = new TextEncoder().encode(JSON.stringify(body));
  const jsonLength = jsonBytes.length + pad(jsonBytes.length);
  const binLength = bin.byteLength + pad(bin.byteLength);
  const total = 12 + 8 + jsonLength + 8 + binLength;
  const out = new Uint8Array(total);
  const dv = new DataView(out.buffer);
  dv.setUint32(0, GLB_MAGIC, true);
  dv.setUint32(4, 2, true);
  dv.setUint32(8, total, true);
  dv.setUint32(12, jsonLength, true);
  dv.setUint32(16, CHUNK_JSON, true);
  out.set(jsonBytes, 20);
  out.fill(0x20, 20 + jsonBytes.length, 20 + jsonLength);
  const binStart = 20 + jsonLength;
  dv.setUint32(binStart, binLength, true);
  dv.setUint32(binStart + 4, CHUNK_BIN, true);
  out.set(bin, binStart + 8);
  return out;
}
