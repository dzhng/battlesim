// The template art library: for every building template, in each state a side
// can know it in, the rows that place shared kit modules on it. One runtime
// file, content-addressed like a bundle and named in the runtime catalog. The
// one loader installs it whole with the catalog; the kits it places are
// fetched when something that draws from them asks.
//
// The simulation never sees it. A template's physical shape is the contract's
// descriptor, identified by its physical catalogue's hash; this library says
// which catalogues it was made to (`covers`: the map generator's and the
// authored maps') and has its own identity (`art_hash`). Art may change
// without moving any map, scenario or battle.
//
// Rows are columns over the whole library (34 bytes a row), and a template's
// state is a range of them, so resolving a placed building copies nothing but
// its transforms.

import { decodeContainer, encodeContainer } from "./codec.ts";
import { sha256Hex } from "./glb.ts";
import { TIER_COUNT, type StaticBundle } from "./schema.ts";

const MAGIC = 0x4c544742; // "BGTL" little-endian
export const TEMPLATE_LIBRARY_VERSION = 2;

/** What a side can know a building as. `intact` always has rows; `ruin` (a
 *  collapsed building) and `gutted` (a burnt shell that stands) come with the
 *  damage pass. */
export const TEMPLATE_STATES = ["intact", "ruin", "gutted"] as const;
export type TemplateState = (typeof TEMPLATE_STATES)[number];

/** A label a set gives each template, carried to the bake's report and the
 *  line-up lab: `release` is accepted art, `prototype` art not yet accepted.
 *  Nothing is drawn differently for it. */
export const TEMPLATE_STATUSES = ["prototype", "release"] as const;
export type TemplateStatus = (typeof TEMPLATE_STATUSES)[number];

/** A run of the library's rows. */
export interface RowRange {
  first: number;
  count: number;
}

export interface TemplateArt {
  /** The physical template's id (the descriptor's). */
  id: string;
  /** The source set that made it. */
  set: string;
  status: TemplateStatus;
  states: Partial<Record<TemplateState, RowRange>>;
}

/** Floats per row transform: x, y, z, yaw (radians about +Z), then the scale
 *  on the module's own x, y and z. A module's geometry is scaled, turned,
 *  then moved. */
export const ROW_TRANSFORM_FLOATS = 7;

export interface TemplateArtLibrary {
  /** This library's identity: the kits' bundles (modules, materials, tiers),
   *  every row and status, and `covers`. */
  art_hash: string;
  /** The hashes of the physical catalogues whose templates these rows
   *  dress, in the order of the names the bake has them under: every
   *  template of each has art here. */
  covers: string[];
  /** The kit appearances the rows place modules of, and the bundle each was
   *  packed against, in name order. */
  kits: { appearance: string; bundle: string }[];
  /** Every module a row may name: its kit (an index into `kits`) and its id
   *  there, the name of its state in the kit's bundle. */
  modules: { kit: number; module: string }[];
  /** In id order. */
  templates: TemplateArt[];
  rows: {
    /** Per row: an index into `modules`. */
    module: Uint16Array;
    /** Per row: `ROW_TRANSFORM_FLOATS` floats in the template's frame. */
    transform: Float32Array;
    /** Per row: a bit per detail tier it draws at (1 is tier 0). */
    tiers: Uint8Array;
    /** Per row: sRGB r, g, b on its tint-masked surfaces; 255 leaves them as authored. */
    tint: Uint8Array;
  };
}

/** Give a library its identity. The hash is over the library's own encoding
 *  without the hash, so anything that changes what is drawn changes it. */
export async function sealTemplateLibrary(
  content: Omit<TemplateArtLibrary, "art_hash">,
): Promise<TemplateArtLibrary> {
  const { covers, kits, modules, templates, rows } = content;
  const art_hash = await sha256Hex(
    encodeContainer(MAGIC, TEMPLATE_LIBRARY_VERSION, { covers, kits, modules, templates, rows }),
  );
  return { art_hash, covers, kits, modules, templates, rows };
}

export function encodeTemplateLibrary(library: TemplateArtLibrary): Uint8Array {
  return encodeContainer(MAGIC, TEMPLATE_LIBRARY_VERSION, library);
}

export function decodeTemplateLibrary(bytes: Uint8Array): TemplateArtLibrary {
  const library = decodeContainer(
    bytes,
    MAGIC,
    TEMPLATE_LIBRARY_VERSION,
    "template library",
  ) as TemplateArtLibrary;
  assertShape(library);
  return library;
}

function assertShape(library: TemplateArtLibrary) {
  const bad = (why: string) => new Error(`template library: ${why}`);
  const { rows } = library;
  if (
    typeof library.art_hash !== "string" ||
    !Array.isArray(library.covers) ||
    !Array.isArray(library.kits) ||
    !Array.isArray(library.modules) ||
    !Array.isArray(library.templates) ||
    !(rows?.module instanceof Uint16Array) ||
    !(rows.transform instanceof Float32Array) ||
    !(rows.tiers instanceof Uint8Array) ||
    !(rows.tint instanceof Uint8Array)
  )
    throw bad("malformed");
  const count = rows.module.length;
  if (
    rows.transform.length !== count * ROW_TRANSFORM_FLOATS ||
    rows.tiers.length !== count ||
    rows.tint.length !== count * 3
  )
    throw bad("row columns differ in length");
  if (library.modules.some((m) => !library.kits[m.kit])) throw bad("a module names no kit");
  for (const module of rows.module)
    if (module >= library.modules.length) throw bad("a row names no module");
  for (const template of library.templates)
    for (const range of Object.values(template.states))
      if (!(range.first >= 0 && range.count >= 0 && range.first + range.count <= count))
        throw bad(`template ${template.id} has rows past the end`);
}

// ---------------------------------------------------------------- resolving

export type TemplateArtErrorCode =
  /** The library has no art for the template. */
  | "template.missing"
  /** The template has no rows for the state. */
  | "state.missing"
  /** A kit the library places is not in the catalog, is not the bundle it was
   *  packed against, or is not installed where a map draws from it. */
  | "kit.missing"
  /** A module the library places is not in its kit. */
  | "module.missing";

/** Why template art could not be resolved. Nothing stands in for missing
 *  art: no other template, state, family or module. */
export class TemplateArtError extends Error {
  readonly code: TemplateArtErrorCode;
  constructor(code: TemplateArtErrorCode, message: string) {
    super(`${code}: ${message}`);
    this.code = code;
  }
}

const indexes = new WeakMap<TemplateArtLibrary, Map<string, TemplateArt>>();
function templateIndex(library: TemplateArtLibrary): Map<string, TemplateArt> {
  let index = indexes.get(library);
  if (!index) {
    index = new Map(library.templates.map((t) => [t.id, t]));
    indexes.set(library, index);
  }
  return index;
}

/** A template's art, or the refusal naming it. */
export function templateArt(library: TemplateArtLibrary, templateId: string): TemplateArt {
  const template = templateIndex(library).get(templateId);
  if (!template)
    throw new TemplateArtError(
      "template.missing",
      `template "${templateId}" has no art in library ${library.art_hash.slice(0, 12)}`,
    );
  return template;
}

/** The rows a template draws in a state: a range of the library's row
 *  columns, shared, never copied. */
export function templateRows(
  library: TemplateArtLibrary,
  templateId: string,
  state: TemplateState,
): RowRange {
  const template = templateArt(library, templateId);
  const rows = template.states[state];
  if (!rows)
    throw new TemplateArtError(
      "state.missing",
      `template "${templateId}" (set ${template.set}) has no "${state}" rows`,
    );
  return rows;
}

/** The kit appearances whose modules the rows of `templateIds` place, in
 *  every state: what drawing those templates needs installed. */
export function templateKits(
  library: TemplateArtLibrary,
  templateIds: Iterable<string>,
): Set<string> {
  const kits = new Set<string>();
  for (const id of templateIds)
    for (const range of Object.values(templateArt(library, id).states))
      for (let r = range.first; r < range.first + range.count; r++)
        kits.add(library.kits[library.modules[library.rows.module[r]].kit].appearance);
  return kits;
}

/** Where a template stands on a map: the contract's `PlacementFrame`. */
export interface PlacementFrame {
  translation: readonly [number, number, number];
  /** Radians about +Z. */
  yaw: number;
}

/**
 * Write `rows` in world space: `ROW_TRANSFORM_FLOATS` floats per row into
 * `out` from `offset`, each row's position turned and moved by the frame and
 * its yaw turned with it. Returns the offset past the last float written.
 * Allocates nothing.
 */
export function placeRows(
  library: TemplateArtLibrary,
  rows: RowRange,
  frame: PlacementFrame,
  out: Float32Array,
  offset = 0,
): number {
  const source = library.rows.transform;
  const [cos, sin] = [Math.cos(frame.yaw), Math.sin(frame.yaw)];
  const [tx, ty, tz] = frame.translation;
  let from = rows.first * ROW_TRANSFORM_FLOATS;
  let to = offset;
  for (let i = 0; i < rows.count; i++) {
    const [x, y] = [source[from], source[from + 1]];
    out[to] = tx + cos * x - sin * y;
    out[to + 1] = ty + sin * x + cos * y;
    out[to + 2] = tz + source[from + 2];
    out[to + 3] = frame.yaw + source[from + 3];
    out[to + 4] = source[from + 4];
    out[to + 5] = source[from + 5];
    out[to + 6] = source[from + 6];
    from += ROW_TRANSFORM_FLOATS;
    to += ROW_TRANSFORM_FLOATS;
  }
  return to;
}

/** A placed building's module placements. Row `i`'s module, tiers and tint
 *  are the library's columns at `rows.first + i`; its world transform is
 *  `transforms` at `offset + i * ROW_TRANSFORM_FLOATS`. */
export interface Placements {
  rows: RowRange;
  transforms: Float32Array;
  offset: number;
}

/**
 * The module placements of template `templateId`, standing at `frame`, as a
 * side knows it (`state`). Pure. Missing art is refused by name
 * (`TemplateArtError`). Pass `out` to write the transforms into a buffer of
 * the caller's, from `offset`; a caller that keeps its own counts uses
 * `templateRows` and `placeRows` and allocates nothing.
 */
export function resolve(
  templateId: string,
  frame: PlacementFrame,
  state: TemplateState,
  library: TemplateArtLibrary,
  out?: Float32Array,
  offset = 0,
): Placements {
  const rows = templateRows(library, templateId, state);
  const transforms = out ?? new Float32Array(rows.count * ROW_TRANSFORM_FLOATS);
  placeRows(library, rows, frame, transforms, offset);
  return { rows, transforms, offset };
}

/** A library module where it is drawn from: its kit appearance, and the index
 *  of its state in that kit's bundle. `state` is null while the kit is not
 *  installed: the module is known, and nothing can draw it yet. */
export interface BoundModule {
  kit: string;
  state: number | null;
}

/**
 * Bind every module of a library to the kits that are installed, in the
 * library's module order. `kitOf` answers a kit appearance's bundle, or
 * nothing for a kit that is not installed: its modules are bound to no state.
 * A module its installed kit lacks is refused by name.
 */
export function bindModules(
  library: TemplateArtLibrary,
  kitOf: (appearance: string) => StaticBundle | undefined,
): BoundModule[] {
  const kits = library.kits.map((kit) => {
    const bundle = kitOf(kit.appearance);
    return bundle && new Map(bundle.states.map((state, i) => [state.name, i]));
  });
  return library.modules.map((module) => {
    const kit = library.kits[module.kit].appearance;
    const states = kits[module.kit];
    if (!states) return { kit, state: null };
    const state = states.get(module.module);
    if (state === undefined)
      throw new TemplateArtError("module.missing", `kit "${kit}" has no module "${module.module}"`);
    return { kit, state };
  });
}

/** Every tier's bit of a row's tier mask. */
export const ALL_TIERS = (1 << TIER_COUNT) - 1;
