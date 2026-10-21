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
  catalogLoadNames,
  fetchedOnRequest,
  KIT_BUNDLE_MAX_BYTES,
  RUNTIME_DIR,
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
 *  textures. `texture` supplies each texture, so a
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
      unpackGzip(bytes, record, hash),
    ),
    Promise.all((catalog.textures?.[hash] ?? []).map(texture)),
  ]);
  return joinTextures(content, textures);
}

/** The files the bundle `hash` is read from: its own, then its textures'. */
export function bundleFiles(catalog: RuntimeCatalog, hash: string): string[] {
  return [
    bundlePath(gzipTransport(catalog, hash).hash),
    ...(catalog.cards?.[hash]
      ? [`${gzipTransport(catalog, catalog.cards[hash]).hash}/card.bin`]
      : []),
    ...(catalog.textures?.[hash] ?? []).map((id) => texturePath(gzipTransport(catalog, id).hash)),
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
    const card = catalog.cards?.[hash];
    if (card) once(card, 512 * 128 * 8 + 4096);
  }
  return bytes;
}

/** What a page whose units wear `wearing` downloads with the catalog, before
 *  its first frame: the template library, the skeleton clips and the
 *  appearances its load takes (`catalogLoadNames`), with their textures, each
 *  file once. `held` collects what it counted. The loader's admission and the
 *  native check share this count. */
export function catalogLoadBytes(
  catalog: RuntimeCatalog,
  wearing?: ReadonlySet<string>,
  held = new Set<string>(),
  factions?: ReadonlySet<string> | null,
  scenery?: ReadonlySet<string> | null,
): number {
  const names = catalogLoadNames(catalog, wearing, factions, scenery);
  const skeletons = new Set(
    names
      .map((name) => catalog.appearances[name]?.skeleton)
      .filter((id): id is string => Boolean(id)),
  );
  const eager = [
    ...(catalog.templates ? [catalog.templates.library] : []),
    ...[...skeletons]
      .map((id) => catalog.skeletons?.[id])
      .filter((hash): hash is string => Boolean(hash)),
    ...names.map((name) => catalog.appearances[name].bundle),
  ];
  return wireBytes(catalog, eager, held);
}

/** What fetching the appearances fetched on request `names` (a kit with its
 *  full damage art, a regional look) downloads beyond the catalog load of a
 *  page whose units wear `wearing`: each once, with the textures of theirs
 *  the load did not fetch. Map selection, loader admission and the native CLI
 *  share this count. */
export function downloadBytes(
  catalog: RuntimeCatalog,
  names: Iterable<string>,
  wearing?: ReadonlySet<string>,
  factions?: ReadonlySet<string> | null,
  scenery?: ReadonlySet<string> | null,
): number {
  const held = new Set<string>();
  catalogLoadBytes(catalog, wearing, held, factions, scenery);
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

/** The original content `rawHash` from its gzip transport, cancelling as
 *  soon as it exceeds the declared length. Its lengths and gzip's own
 *  checksum refuse a damaged file; content hashes are proved once, when the
 *  files are baked and checked (`asset check`), not on every read. */
export async function unpackGzip(
  bytes: Uint8Array,
  record: GzipTransport,
  rawHash: string,
): Promise<Uint8Array> {
  if (bytes.length !== record.bytes)
    throw new Error(`content ${rawHash}: gzip length is ${bytes.length}, expected ${record.bytes}`);
  const raw = new Uint8Array(record.raw_bytes);
  const reader = new Blob([bytes as Uint8Array<ArrayBuffer>])
    .stream()
    .pipeThrough(new DecompressionStream("gzip"))
    .getReader();
  let at = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read().catch((error: unknown) => {
        throw new Error(`content ${rawHash}: gzip does not inflate (${String(error)})`, {
          cause: error,
        });
      });
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
