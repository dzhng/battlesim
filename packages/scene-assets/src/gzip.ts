// Lossless transport around the existing codecs. Encoded files and decoded
// art have separate content addresses; only decoded art names modules/rows.
import { sha256Hex } from "./glb.ts";
import {
  fetchedOnRequest,
  KIT_BUNDLE_MAX_BYTES,
  type GzipTransport,
  type RuntimeCatalog,
} from "./schema.ts";

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

/** The library plus each selected appearance fetched on request (a kit with
 * its full damage art, a regional look) once, as its gzip object. Map
 * selection, loader admission and the native CLI share this count. */
export function downloadBytes(catalog: RuntimeCatalog, names: Iterable<string>): number {
  let bytes = catalog.templates ? gzipTransport(catalog, catalog.templates.library).bytes : 0;
  const hashes = new Set<string>();
  for (const name of names) {
    const entry = catalog.appearances[name];
    const what = `${onRequestLabel(entry)} "${name}"`;
    if (!entry || !fetchedOnRequest(entry))
      throw new Error(`${what} is not in the appearance catalog`);
    if (hashes.has(entry.bundle)) continue;
    try {
      bytes += gzipTransport(catalog, entry.bundle, KIT_BUNDLE_MAX_BYTES).bytes;
    } catch (error) {
      const why = error instanceof Error ? error.message : String(error);
      throw new Error(`${what}: ${why}`, { cause: error });
    }
    hashes.add(entry.bundle);
  }
  return bytes;
}

/** What an appearance fetched on request is called in an error: a kit, or a
 *  regional look. One the catalog lacks was asked for as a kit. */
export const onRequestLabel = (entry: RuntimeCatalog["appearances"][string] | undefined) =>
  entry?.regional_family !== undefined ? "regional look" : "kit";

/** Verify the transport, cancel inflation as soon as it exceeds the declared
 * length, then verify exactly the original content before any codec reads it. */
export async function unpackGzip(
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
  if ((await sha256Hex(raw)) !== rawHash)
    throw new Error(`content ${rawHash}: decoded content hash differs`);
  return raw;
}
