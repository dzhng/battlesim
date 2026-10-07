// The runtime transport around the existing codecs, for the baker, the loader
// and the native readers alike. Every bundle, texture and the template library
// is one gzip file. A bundle travels without its textures' pixels: each
// texture is its own file, stored once however many bundles name it (the
// catalog's texture table). Encoded files and original content have separate
// content addresses; only the original's names art, and a reader checks both.
import { encodeBundle, joinTextures, splitTextures } from "./codec.ts";
import { lfsPointerOid, lfsPullCommand, sha256Hex } from "./glb.ts";
import {
  bundlePath,
  fetchedOnRequest,
  KIT_BUNDLE_MAX_BYTES,
  RUNTIME_DIR,
  templateLibraryPath,
  texturePath,
  type Bundle,
  type GzipTransport,
  type RuntimeCatalog,
  type Texture,
} from "./schema.ts";
import { decodeTexture, encodeTexture } from "./texture.ts";

export async function packGzip(raw: Uint8Array): Promise<{
  bytes: Uint8Array;
  transport: GzipTransport;
}> {
  const stream = new Blob([raw as Uint8Array<ArrayBuffer>])
    .stream()
    .pipeThrough(new CompressionStream("gzip"));
  const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
  return {
    bytes,
    transport: { hash: await sha256Hex(bytes), bytes: bytes.length, raw_bytes: raw.length },
  };
}

/** A required gzip record, checked before its download or allocation. */
export function gzipTransport(
  catalog: RuntimeCatalog,
  rawHash: string,
  maxRaw = Infinity,
): GzipTransport {
  const record = catalog.gzip?.[rawHash];
  if (!record) throw new Error(`content ${rawHash}: missing gzip transport`);
  if (
    !/^[0-9a-f]{64}$/.test(record.hash) ||
    !Number.isSafeInteger(record.bytes) ||
    record.bytes <= 0 ||
    !Number.isSafeInteger(record.raw_bytes) ||
    record.raw_bytes <= 0 ||
    record.raw_bytes > maxRaw
  )
    throw new Error(`content ${rawHash}: invalid gzip lengths or hash`);
  return record;
}

/** What the bake and the workbench previews publish: the runtime catalog and
 *  its files by path under the runtime directory. */
export interface Publication {
  runtime: RuntimeCatalog;
  files: Map<string, Uint8Array>;
}

/** Publish `raw`, whose content hash is `hash`, as one gzip file at `path`. */
export async function publishGzip(
  out: Publication,
  hash: string,
  raw: Uint8Array,
  path: (hash: string) => string,
): Promise<void> {
  const packed = await packGzip(raw);
  (out.runtime.gzip ??= {})[hash] = packed.transport;
  out.files.set(path(packed.transport.hash), packed.bytes);
}

/** Publish a bundle: its content without its textures' pixels, and each
 *  texture not yet published as its own file. Returns its art identity, the
 *  hash of its whole encoding, and that encoding's length. */
export async function publishBundle(
  out: Publication,
  bundle: Bundle,
): Promise<{ hash: string; bytes: number }> {
  const art = encodeBundle(bundle);
  const hash = await sha256Hex(art);
  const { content, textures } = splitTextures(bundle);
  await publishGzip(out, hash, content, bundlePath);
  if (textures.length) (out.runtime.textures ??= {})[hash] = textures.map((t) => t.id);
  for (const t of textures)
    if (!out.runtime.gzip?.[t.id]) await publishGzip(out, t.id, encodeTexture(t), texturePath);
  return { hash, bytes: art.byteLength };
}

/** A runtime file's bytes, by its path under the runtime directory. */
export type ReadRuntime = (path: string) => Promise<Uint8Array>;

/** `read(path)`, refusing a Git LFS pointer with the pull that fetches it. */
async function readFile(read: ReadRuntime, what: string, hash: string, path: string) {
  const bytes = await read(path);
  if (lfsPointerOid(bytes) !== null)
    throw new Error(
      `${what} ${hash} is a Git LFS pointer; run: ${lfsPullCommand(`${RUNTIME_DIR}/${path}`)}`,
    );
  return bytes;
}

/** The original content `hash` of a gzip file at `path(encoded hash)`
 *  (`what` names it in a refusal): the template library, a texture. */
export async function readGzip(
  catalog: RuntimeCatalog,
  what: string,
  hash: string,
  path: (hash: string) => string,
  read: ReadRuntime,
): Promise<Uint8Array> {
  const record = gzipTransport(catalog, hash);
  return unpackGzip(await readFile(read, what, hash, path(record.hash)), record, hash);
}

/** The texture at address `id`, verified against it. */
export async function readTexture(
  catalog: RuntimeCatalog,
  id: string,
  read: ReadRuntime,
): Promise<Texture> {
  return decodeTexture(id, await readGzip(catalog, "texture", id, texturePath, read));
}

/** The bundle whose art identity is `hash`: its content joined with its
 *  textures, verified against `hash`. `texture` supplies each texture, so a
 *  reader can fetch one once for every bundle that names it. */
export async function readBundle(
  catalog: RuntimeCatalog,
  hash: string,
  read: ReadRuntime,
  texture: (id: string) => Promise<Texture> = (id) => readTexture(catalog, id, read),
  maxRaw = Infinity,
): Promise<Bundle> {
  const record = gzipTransport(catalog, hash, maxRaw);
  const [content, textures] = await Promise.all([
    readFile(read, "bundle", hash, bundlePath(record.hash)).then((bytes) =>
      inflateGzip(bytes, record, hash),
    ),
    Promise.all((catalog.textures?.[hash] ?? []).map(texture)),
  ]);
  const bundle = joinTextures(content, textures);
  if ((await sha256Hex(encodeBundle(bundle))) !== hash)
    throw new Error(`content ${hash}: decoded content hash differs`);
  return bundle;
}

/** The files the bundle `hash` is read from: its own, then its textures'. */
export function bundleFiles(catalog: RuntimeCatalog, hash: string): string[] {
  return [
    bundlePath(gzipTransport(catalog, hash).hash),
    ...(catalog.textures?.[hash] ?? []).map((id) => texturePath(gzipTransport(catalog, id).hash)),
  ];
}

/** Every runtime file the catalog names, each once, but the catalog itself. */
export function runtimeFiles(catalog: RuntimeCatalog): string[] {
  const bundles = [
    ...Object.values(catalog.skeletons ?? {}),
    ...Object.values(catalog.appearances ?? {}).map((entry) => entry.bundle),
  ];
  return [
    ...new Set([
      ...bundles.flatMap((hash) => bundleFiles(catalog, hash)),
      ...(catalog.templates
        ? [templateLibraryPath(gzipTransport(catalog, catalog.templates.library).hash)]
        : []),
    ]),
  ];
}

/** Wire bytes of `hashes`' files and their textures', skipping and then
 *  adding to `held`: what fetching them costs a page that already has `held`. */
function wireBytes(
  catalog: RuntimeCatalog,
  hashes: Iterable<string>,
  held: Set<string>,
  maxRaw = Infinity,
): number {
  let bytes = 0;
  const once = (hash: string, max = Infinity) => {
    if (held.has(hash)) return;
    bytes += gzipTransport(catalog, hash, max).bytes;
    held.add(hash);
  };
  for (const hash of hashes) {
    once(hash, maxRaw);
    for (const id of catalog.textures?.[hash] ?? []) once(id);
  }
  return bytes;
}

/** What a page downloads with the catalog, before its first frame: the
 *  template library, the skeleton clips and every appearance not fetched on
 *  request, with their textures, each file once. `held` collects what it
 *  counted. The loader's admission and the native check share this count. */
export function catalogLoadBytes(catalog: RuntimeCatalog, held = new Set<string>()): number {
  const eager = [
    ...(catalog.templates ? [catalog.templates.library] : []),
    ...Object.values(catalog.skeletons ?? {}),
    ...Object.values(catalog.appearances ?? {})
      .filter((entry) => !fetchedOnRequest(entry))
      .map((entry) => entry.bundle),
  ];
  return wireBytes(catalog, eager, held);
}

/** What fetching the appearances fetched on request `names` (a kit with its
 *  full damage art, a regional look) downloads beyond the catalog load: each
 *  once, with the textures of theirs the load did not fetch. Map selection,
 *  loader admission and the native CLI share this count. */
export function downloadBytes(catalog: RuntimeCatalog, names: Iterable<string>): number {
  const held = new Set<string>();
  catalogLoadBytes(catalog, held);
  let bytes = 0;
  for (const name of names) {
    const entry = catalog.appearances[name];
    const what = `${onRequestLabel(entry)} "${name}"`;
    if (!entry || !fetchedOnRequest(entry))
      throw new Error(`${what} is not in the appearance catalog`);
    try {
      bytes += wireBytes(catalog, [entry.bundle], held, KIT_BUNDLE_MAX_BYTES);
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      throw new Error(`${what}: ${why}`, { cause: error });
    }
  }
  return bytes;
}

/** What an appearance fetched on request is called in an error: a kit, or a
 *  regional look. One the catalog lacks was asked for as a kit. */
export const onRequestLabel = (entry: RuntimeCatalog["appearances"][string] | undefined) =>
  entry?.regional_family !== undefined ? "regional look" : "kit";

/** Verify the transport and inflate it, cancelling as soon as it exceeds the
 *  declared length. What it inflates to is the caller's to verify. */
async function inflateGzip(
  bytes: Uint8Array,
  record: GzipTransport,
  rawHash: string,
): Promise<Uint8Array> {
  if (bytes.length !== record.bytes)
    throw new Error(`content ${rawHash}: gzip length is ${bytes.length}, expected ${record.bytes}`);
  if ((await sha256Hex(bytes)) !== record.hash)
    throw new Error(`content ${rawHash}: gzip content hash differs`);
  const raw = new Uint8Array(record.raw_bytes);
  const reader = new Blob([bytes as Uint8Array<ArrayBuffer>])
    .stream()
    .pipeThrough(new DecompressionStream("gzip"))
    .getReader();
  let at = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      if (at + value.length > raw.length)
        throw new Error(`content ${rawHash}: gzip exceeds decoded length ${raw.length}`);
      raw.set(value, at);
      at += value.length;
    }
  } catch (error) {
    await reader.cancel().catch(() => {});
    throw error;
  } finally {
    reader.releaseLock();
  }
  if (at !== raw.length)
    throw new Error(`content ${rawHash}: decoded length is ${at}, expected ${raw.length}`);
  return raw;
}

/** Verify the transport, inflate it, then verify exactly the original content
 *  before any codec reads it. */
export async function unpackGzip(
  bytes: Uint8Array,
  record: GzipTransport,
  rawHash: string,
): Promise<Uint8Array> {
  const raw = await inflateGzip(bytes, record, rawHash);
  if ((await sha256Hex(raw)) !== rawHash)
    throw new Error(`content ${rawHash}: decoded content hash differs`);
  return raw;
}
