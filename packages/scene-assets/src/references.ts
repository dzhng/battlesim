// A family's reference library: the photos, drawings and labelled generated
// views a modeller and a reviewer judge its art against, in
// `assets/references/<family>/` beside `references.json`. Review inputs only:
// no runtime, bake or test reads them; `asset check` holds the library to
// this contract and `asset sheet --references` sets them beside the model.
//
// references.json is `{ "entries": [ReferenceEntry...], "gaps": [string...] }`.

import { contentSha256, lfsPointerOid } from "./glb.ts";
import type { Severity } from "./schema.ts";

/** The views a reference shows, in the order a sheet lays them out. A soldier
 *  subject (a uniform, a carrier, a team weapon deployed or carried) is a
 *  variant of its own, photographed from the same views; `detail` is any
 *  close-up (running gear, a turret roof, a sight, a helmet). */
export const REFERENCE_VIEWS = [
  "three_quarter_front",
  "side",
  "three_quarter_rear",
  "front",
  "rear",
  "top",
  "detail",
] as const;
export type ReferenceView = (typeof REFERENCE_VIEWS)[number];

export const REFERENCE_SOURCES = ["photo", "drawing", "generated"] as const;
export type ReferenceSource = (typeof REFERENCE_SOURCES)[number];

/** Licences under which we may redistribute a photo or drawing, as SPDX ids
 *  (with `public-domain` for a work under none). A generated view is `ours`,
 *  and only a generated view is. */
export const REFERENCE_LICENCES = [
  "public-domain",
  "CC0-1.0",
  "CC-BY-2.0",
  "CC-BY-2.5",
  "CC-BY-3.0",
  "CC-BY-4.0",
  "CC-BY-SA-2.0",
  "CC-BY-SA-2.5",
  "CC-BY-SA-3.0",
  "CC-BY-SA-4.0",
] as const;
export const GENERATED_LICENCE = "ours";

/** Whether `licence` is allowed: one of `REFERENCE_LICENCES`, or a country port
 *  of an allowed Creative Commons licence (`CC-BY-SA-3.0-DE`), which grants
 *  the same redistribution. */
export const allowedLicence = (licence: string) =>
  (REFERENCE_LICENCES as readonly string[]).includes(licence) ||
  (/^CC-.*-[A-Z]{2,3}$/.test(licence) &&
    (REFERENCE_LICENCES as readonly string[]).includes(licence.replace(/-[A-Z]{2,3}$/, "")));

/** A reference's longest edge, in pixels: larger originals are resized. */
export const REFERENCE_MAX_EDGE_PX = 1600;

export interface ReferenceEntry {
  /** A file in the family's folder: `<variant>-<view>[-n][-generated].jpg|png`. */
  file: string;
  /** What it shows: an appearance name, or a soldier subject. */
  variant: string;
  view: ReferenceView;
  source: ReferenceSource;
  /** The file page (a Commons page, not a thumbnail URL); "" for a generated view. */
  page: string;
  author: string;
  licence: string;
  /** The sha256 of the file's bytes, lowercase hex. */
  sha256: string;
  note: string;
  /** A generated view's model id, full prompt, input reference files and seed. */
  model?: string;
  prompt?: string;
  inputs?: string[];
  seed?: number;
}

export interface ReferenceLibrary {
  entries: ReferenceEntry[];
  /** What the photos leave open, and what a generated view had to guess. */
  gaps: string[];
}

export const REFERENCE_CODES = [
  /** references.json is not JSON, or not `{ entries, gaps }`. */
  "references.json",
  /** An entry lacks a field, or a field is not of its kind. */
  "references.entry",
  /** An entry's file is not in the folder (or names another folder). */
  "references.file",
  /** A file's bytes do not hash to its entry's sha256. */
  "references.sha256",
  /** A licence outside the allowed list, or `ours` on a real image. */
  "references.licence",
  /** Not a JPEG or PNG, or a long edge over the limit. */
  "references.image",
  /** A generated view without its provenance or its label, or a real one with the label. */
  "references.generated",
  /** A variant's view whose only sources are generated. */
  "references.view",
  /** A file is an LFS pointer: its hash is checked, its size is not. */
  "references.unpulled",
] as const;
export type ReferenceCode = (typeof REFERENCE_CODES)[number];

export interface ReferenceFinding {
  code: ReferenceCode;
  severity: Severity;
  message: string;
}

/** The pixel size of a JPEG or PNG from its header, or null for anything else. */
export function imageSize(bytes: Uint8Array): { width: number; height: number } | null {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const pngSignature = [137, 80, 78, 71, 13, 10, 26, 10];
  if (bytes.byteLength >= 24 && pngSignature.every((b, i) => bytes[i] === b))
    return { width: view.getUint32(16), height: view.getUint32(20) };
  if (bytes.byteLength < 4 || bytes[0] !== 0xff || bytes[1] !== 0xd8) return null;
  // Walk the JPEG's segments to its frame header (SOF0..SOF15 bar DHT, JPG, DAC).
  let at = 2;
  while (at + 9 <= bytes.byteLength) {
    if (bytes[at] !== 0xff) return null;
    const marker = bytes[at + 1];
    if (marker === 0xff) {
      at += 1;
      continue;
    }
    if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker))
      return { height: view.getUint16(at + 5), width: view.getUint16(at + 7) };
    if (marker === 0xd9 || marker === 0xda) return null;
    at += 2 + view.getUint16(at + 2);
  }
  return null;
}

const isText = (v: unknown): v is string => typeof v === "string";
const isFilled = (v: unknown): v is string => typeof v === "string" && v.trim() !== "";
const stem = (file: string) => file.replace(/\.[^.]+$/, "");

/** Every way `text`, the references.json of `family`, breaks the library's
 *  contract. `read` gives a file of the family's folder by name, or null if
 *  it is not there. An LFS pointer is judged by its oid, which is the sha256
 *  of its content, and its size is left unjudged with a warning. */
export async function checkReferences(
  family: string,
  text: string,
  read: (file: string) => Uint8Array | null,
): Promise<ReferenceFinding[]> {
  const findings: ReferenceFinding[] = [];
  const at = `assets/references/${family}`;
  const error = (code: ReferenceCode, message: string) =>
    findings.push({ code, severity: "error", message: `${at}: ${message}` });
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e) {
    error("references.json", `references.json does not parse: ${(e as Error).message}`);
    return findings;
  }
  const library = parsed as Partial<ReferenceLibrary> | null;
  if (
    !library ||
    typeof library !== "object" ||
    !Array.isArray(library.entries) ||
    !Array.isArray(library.gaps) ||
    !library.gaps.every(isText)
  ) {
    error("references.json", `references.json is not { "entries": [...], "gaps": [strings] }`);
    return findings;
  }

  const files = new Set(library.entries.map((e) => e?.file));
  const generatedFiles = new Set(
    library.entries.filter((e) => e?.source === "generated").map((e) => e.file),
  );
  /** Per variant and view, whether a real image shows it. */
  const shown = new Map<string, boolean>();
  const seen = new Set<string>();

  for (const [i, entry] of library.entries.entries()) {
    const label = isFilled(entry?.file) ? entry.file : `entry ${i}`;
    if (!entry || typeof entry !== "object") {
      error("references.entry", `${label} is not an object`);
      continue;
    }
    const required = ["file", "variant", "sha256"] as const;
    const missing = required.filter((k) => !isFilled(entry[k]));
    if (!isText(entry.note)) missing.push("note" as never);
    if (missing.length) {
      error("references.entry", `${label} lacks ${missing.join(", ")}`);
      continue;
    }
    if (seen.has(entry.file)) error("references.entry", `${label} is listed twice`);
    seen.add(entry.file);
    if (!REFERENCE_VIEWS.includes(entry.view))
      error(
        "references.entry",
        `${label} view ${JSON.stringify(entry.view)} is none of ${REFERENCE_VIEWS.join(", ")}`,
      );
    if (!REFERENCE_SOURCES.includes(entry.source)) {
      error(
        "references.entry",
        `${label} source ${JSON.stringify(entry.source)} is none of ${REFERENCE_SOURCES.join(", ")}`,
      );
      continue;
    }
    const isGenerated = entry.source === "generated";

    // Provenance: a real image names its page and author under a licence we
    // may redistribute; a generated one is ours, labelled, and says how it was made.
    if (isGenerated) {
      if (entry.licence !== GENERATED_LICENCE)
        error(
          "references.licence",
          `${label} is generated, so its licence is "${GENERATED_LICENCE}", not ${JSON.stringify(entry.licence)}`,
        );
      const lacks = [
        !isFilled(entry.model) && "model",
        !isFilled(entry.prompt) && "prompt",
        !(Array.isArray(entry.inputs) && entry.inputs.length && entry.inputs.every(isFilled)) &&
          "inputs",
        !Number.isInteger(entry.seed) && "seed",
      ].filter(Boolean);
      if (lacks.length)
        error("references.generated", `${label} is generated but lacks ${lacks.join(", ")}`);
      for (const input of Array.isArray(entry.inputs) ? entry.inputs : [])
        if (isFilled(input) && (!files.has(input) || generatedFiles.has(input)))
          error(
            "references.generated",
            `${label} is generated from ${input}, which is not a real reference in this library`,
          );
      if (!stem(entry.file).endsWith("-generated"))
        error(
          "references.generated",
          `${label} is generated but its name lacks the -generated suffix`,
        );
    } else {
      if (!allowedLicence(entry.licence))
        error(
          "references.licence",
          `${label} licence ${JSON.stringify(entry.licence)} is none of ${REFERENCE_LICENCES.join(", ")} or a country port of one`,
        );
      const lacks = (["page", "author"] as const).filter((k) => !isFilled(entry[k]));
      if (lacks.length) error("references.entry", `${label} lacks ${lacks.join(", ")}`);
      if (stem(entry.file).endsWith("-generated"))
        error("references.generated", `${label} is a ${entry.source} named as generated`);
    }
    if (REFERENCE_VIEWS.includes(entry.view)) {
      const key = `${entry.variant}\u0000${entry.view}`;
      shown.set(key, (shown.get(key) ?? false) || !isGenerated);
    }

    // The file itself.
    const bytes = /[/\\]/.test(entry.file) ? null : read(entry.file);
    if (!bytes) {
      error("references.file", `${label} is not in the folder`);
      continue;
    }
    const hash = await contentSha256(bytes);
    if (hash !== entry.sha256.toLowerCase())
      error("references.sha256", `${label} hashes to ${hash}, not ${entry.sha256}`);
    if (lfsPointerOid(bytes) !== null) {
      findings.push({
        code: "references.unpulled",
        severity: "warning",
        message: `${at}: ${label} is an LFS pointer: its hash is checked, its size is not (git lfs pull --include="${at}/${entry.file}")`,
      });
      continue;
    }
    const size = imageSize(bytes);
    if (!size) error("references.image", `${label} is not a JPEG or PNG`);
    else if (Math.max(size.width, size.height) > REFERENCE_MAX_EDGE_PX)
      error(
        "references.image",
        `${label} is ${size.width} × ${size.height} px; its long edge is over ${REFERENCE_MAX_EDGE_PX} px`,
      );
  }

  for (const [key, real] of shown)
    if (!real) {
      const [variant, view] = key.split("\u0000");
      error(
        "references.view",
        `${variant} ${view} has only generated references: find a photo or drawing of it`,
      );
    }
  return findings;
}

/** What a review sheet sets beside one of the model's views: the reference
 *  that shows it best (a photo or drawing before a generated view, else the
 *  library's order), how many show it, or null where none does. */
export interface SheetReference {
  view: ReferenceView;
  entry: ReferenceEntry | null;
  count: number;
}

/** `variant`'s reference for each view, in sheet order. */
export function sheetReferences(library: ReferenceLibrary, variant: string): SheetReference[] {
  return REFERENCE_VIEWS.map((view) => {
    const showing = library.entries.filter((e) => e.variant === variant && e.view === view);
    const entry = showing.find((e) => e.source !== "generated") ?? showing[0] ?? null;
    return { view, entry, count: showing.length };
  });
}
