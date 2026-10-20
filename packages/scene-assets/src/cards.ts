// Prepared static cards share the runtime container and gzip transport with art.
import type { Vec3 } from "math";
import { encodeContainer, decodeContainer } from "./codec.ts";
import { sha256Hex } from "./glb.ts";
import {
  publishGzip,
  readGzip,
  gzipTransport,
  type Publication,
  type ReadRuntime,
} from "./gzip.ts";
import type { RuntimeCatalog } from "./schema.ts";

export const CARD_SPEC = { yaws: 8, pitches: [0.85, 0.5], cell: 64, supersample: 2 } as const;
const MAGIC = 0x43494742; // BGIC
const VERSION = 2;
export const CARD_BYTES = 512 * 128 * 4;
export const cardPath = (hash: string) => `${hash}/card.bin`;
export interface PreparedCard {
  appearance: string;
  bundle: string;
  state: string;
  spec: { yaws: number; pitches: readonly number[]; cell: number; supersample: number };
  width: number;
  height: number;
  center: Vec3;
  radius: number;
  /** Display albedo / coverage and model-space normal / tint, RGBA8. */
  albedo: Uint8Array;
  normal: Uint8Array;
  /** RG8, coverage-weighted reverse-Z UNORM16 depth in the bake frame. */
  depth: Uint8Array;
}

export function validateCard(card: PreparedCard, appearance: string, bundle: string): void {
  const fail = (why: string) => {
    throw new Error(`card ${appearance}: ${why}; re-bake`);
  };
  if (!card || typeof card !== "object") fail("invalid metadata");
  const keys = [
    "appearance",
    "bundle",
    "state",
    "spec",
    "width",
    "height",
    "center",
    "radius",
    "albedo",
    "normal",
    "depth",
  ];
  if (Object.keys(card).some((k) => !keys.includes(k))) fail("unknown channel or metadata");
  if (card.appearance !== appearance || card.bundle !== bundle || !/^[a-f0-9]{64}$/.test(bundle))
    fail("source bundle or appearance differs");
  if (
    card.state !== "default" ||
    card.width !== 512 ||
    card.height !== 128 ||
    card.spec?.yaws !== CARD_SPEC.yaws ||
    card.spec?.cell !== CARD_SPEC.cell ||
    card.spec?.supersample !== CARD_SPEC.supersample ||
    card.spec?.pitches?.length !== 2 ||
    card.spec.pitches.some((p, i) => Math.fround(p) !== Math.fround(CARD_SPEC.pitches[i]))
  )
    fail("unsupported views or state");
  if (
    !Array.isArray(card.center) ||
    card.center.length !== 3 ||
    !card.center.every(Number.isFinite) ||
    !Number.isFinite(card.radius) ||
    card.radius <= 0
  )
    fail("invalid frame");
  if (
    !(card.albedo instanceof Uint8Array) ||
    card.albedo.length !== CARD_BYTES ||
    !(card.normal instanceof Uint8Array) ||
    card.normal.length !== CARD_BYTES ||
    !(card.depth instanceof Uint8Array) ||
    card.depth.length !== CARD_BYTES / 2
  )
    fail("invalid card pixels");
}

export function encodeCard(card: PreparedCard): Uint8Array {
  const { appearance, ...content } = card;
  validateCard(card, appearance, card.bundle);
  return encodeContainer(MAGIC, VERSION, content);
}

export function decodeCard(bytes: Uint8Array, appearance: string, bundle: string): PreparedCard {
  const content = decodeContainer(bytes, MAGIC, VERSION, "card") as Omit<
    PreparedCard,
    "appearance"
  >;
  const card = { ...content, appearance };
  validateCard(card, appearance, bundle);
  return card;
}

export async function publishCard(out: Publication, card: PreparedCard) {
  const entry = out.runtime.appearances[card.appearance];
  if (!entry || !(entry.far_card_px && entry.far_card_px > 0))
    throw new Error(`card ${card.appearance}: no asset opt-in`);
  validateCard(card, card.appearance, entry.bundle);
  const raw = encodeCard(card);
  const hash = await sha256Hex(raw);
  const previous = out.runtime.cards?.[entry.bundle];
  await publishGzip(out, hash, raw, cardPath);
  (out.runtime.cards ??= {})[entry.bundle] = hash;
  if (previous && previous !== hash && !Object.values(out.runtime.cards).includes(previous)) {
    const transport = out.runtime.gzip?.[previous];
    delete out.runtime.gzip?.[previous];
    if (transport && /^[a-f0-9]{64}$/.test(transport.hash)) {
      const removed = cardPath(transport.hash);
      out.files.delete(removed);
      return removed;
    }
  }
  return undefined;
}

export async function readCard(
  catalog: RuntimeCatalog,
  appearance: string,
  read: ReadRuntime,
): Promise<PreparedCard | undefined> {
  const bundle = catalog.appearances[appearance].bundle;
  const hash = catalog.cards?.[bundle];
  if (hash === undefined) return undefined;
  if (typeof hash !== "string" || !/^[a-f0-9]{64}$/.test(hash))
    throw new Error(`card ${appearance}: invalid content address`);
  gzipTransport(catalog, hash, CARD_BYTES * 2.5 + 4096);
  const bytes = await readGzip(catalog, "card", hash, cardPath, read);
  if ((await sha256Hex(bytes)) !== hash)
    throw new Error(`card ${appearance}: content hash differs`);
  return decodeCard(bytes, appearance, bundle);
}

/** Keep only cards whose source identity survives a mesh rebake. Validate the
 * retained files without a GPU; changed art must get a new explicit card bake. */
export async function retainCards(out: Publication, previous: RuntimeCatalog, read: ReadRuntime) {
  for (const [name, entry] of Object.entries(out.runtime.appearances)) {
    if (previous.appearances[name]?.bundle !== entry.bundle || !entry.far_card_px) continue;
    const card = await readCard(previous, name, read);
    if (card) await publishCard(out, card);
  }
}

/** The offline baker reads only its source mesh. Broken old cards must never
 * prevent repairing them, and baking one prop needs no unrelated art. */
export function cardSourceCatalog(catalog: RuntimeCatalog, appearance: string): RuntimeCatalog {
  const entry = catalog.appearances[appearance];
  if (!entry?.far_card_px) throw new Error(`card ${appearance}: no asset opt-in`);
  return {
    sides: catalog.sides,
    appearances: { [appearance]: entry },
    skeletons: {},
    gzip: catalog.gzip,
    textures: catalog.textures,
  };
}
