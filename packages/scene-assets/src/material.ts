// Material transport: how a source material's glTF alpha mode and extras
// become the bundle's coverage and interior metadata (`Coverage`, `schema.ts`),
// and which combinations a bundle may carry. Nothing here draws.

import type { GltfJson } from "./glb.ts";
import {
  INTERIOR_SHEETS,
  type Bundle,
  type Coverage,
  type Finding,
  type InteriorSheet,
  type Material,
  type SkeletonClips,
  type Texture,
} from "./schema.ts";

type AddFinding = (code: Finding["code"], message: string, fix: string) => void;

/** glTF's `alphaCutoff` when a `MASK` material names none. */
const DEFAULT_CUTOFF = 0.5;

const isSheet = (value: unknown): value is InteriorSheet =>
  (INTERIOR_SHEETS as readonly unknown[]).includes(value);

/** A glTF material's coverage: `alphaMode` (`OPAQUE` when absent, `MASK`,
 *  `BLEND`) and, for a mask, `alphaCutoff`. What it cannot read is a finding,
 *  and the material is built opaque. */
export function sourceCoverage(m: GltfJson, name: string, add: AddFinding): Coverage {
  const mode = m.alphaMode ?? "OPAQUE";
  if (mode === "OPAQUE") return { kind: "opaque" };
  if (mode === "BLEND") return { kind: "blended" };
  const cutoff = m.alphaCutoff ?? DEFAULT_CUTOFF;
  if (mode === "MASK" && typeof cutoff === "number" && Number.isFinite(cutoff))
    return { kind: "cutout", cutoff };
  add(
    "material.coverage",
    mode === "MASK"
      ? `material "${name}" has alphaCutoff ${JSON.stringify(cutoff)}, which is not a number`
      : `material "${name}" has alphaMode ${JSON.stringify(mode)}`,
    "export alphaMode OPAQUE, MASK (with an alphaCutoff from 0 to 1) or BLEND",
  );
  return { kind: "opaque" };
}

/** A glTF material's interior sheet (extras `interior`), when it names one. */
export function sourceInterior(
  m: GltfJson,
  name: string,
  add: AddFinding,
): InteriorSheet | undefined {
  const sheet = m.extras?.interior;
  if (sheet === undefined || isSheet(sheet)) return sheet;
  add(
    "material.interior",
    `material "${name}" names interior sheet ${JSON.stringify(sheet)}`,
    `name one of the interior atlas sheets in the material's extras: ${INTERIOR_SHEETS.join(", ")}`,
  );
  return undefined;
}

/** The least and the most a material's coverage value is, over its surface:
 *  the base colour's alpha times its normal texture's alpha. */
function coverageRange(material: Material, textures: Texture[]): [number, number] {
  const index = material.textures?.normal;
  const texels = index === undefined ? undefined : textures[index]?.levels[0];
  let least = 255;
  let most = texels ? 0 : 255;
  for (let i = 3; i < (texels?.length ?? 0); i += 4) {
    least = Math.min(least, texels![i]);
    most = Math.max(most, texels![i]);
  }
  const alpha = material.base_color[3];
  return [(alpha * least) / 255, (alpha * most) / 255];
}

/**
 * The material findings of a built bundle:
 * - `material.coverage`: a cutout's cutoff is outside 0..1;
 * - `material.coverage_source`: a cutout that cuts nothing or everything, or
 *   a blended surface that is whole everywhere, so its coverage was lost on
 *   the way here or was never authored;
 * - `material.wear`: a blended surface that wears;
 * - `material.interior`: a room surface that is not opaque, has a look of its
 *   own (textures, wear), or is on a body that moves.
 */
export function materialFindings(label: string, bundle: Exclude<Bundle, SkeletonClips>): Finding[] {
  const out: Finding[] = [];
  for (const m of bundle.materials) {
    const add: AddFinding = (code, message, fix) =>
      out.push({
        code,
        severity: "error",
        message: `${label}: material "${m.name}" ${message}`,
        fix,
      });
    const { coverage } = m;
    const [least, most] = coverageRange(m, bundle.textures);
    const source =
      "its coverage value is the base colour's alpha times the normal texture's alpha: author it there";
    if (coverage.kind === "cutout") {
      if (!(coverage.cutoff >= 0 && coverage.cutoff <= 1))
        add(
          "material.coverage",
          `is a cutout at ${coverage.cutoff}`,
          "set alphaCutoff from 0 to 1",
        );
      else if (!(least < coverage.cutoff && most >= coverage.cutoff))
        add(
          "material.coverage_source",
          `is a cutout at ${coverage.cutoff}, but its coverage runs from ${least.toFixed(3)} to ${most.toFixed(3)}: ${most < coverage.cutoff ? "none of it is drawn" : "nothing is cut"}`,
          source,
        );
    }
    if (coverage.kind === "blended") {
      if (!(least < 1)) add("material.coverage_source", "is blended, but whole everywhere", source);
      if (m.wear)
        add(
          "material.wear",
          "is blended and wears: a worn patch has no coverage of its own",
          "make it a cutout or opaque, or export it without wear",
        );
    }
    if (m.interior === undefined) continue;
    const fix =
      "an interior material is opaque and untextured, on a static appearance: it shows its atlas cell and nothing else";
    if (bundle.kind !== "static")
      add("material.interior", `is a room, on a ${bundle.kind} body`, fix);
    if (coverage.kind !== "opaque")
      add("material.interior", `is a room, and ${coverage.kind}`, fix);
    if (m.textures || m.wear)
      add(
        "material.interior",
        `is a room, and has ${m.textures ? "textures" : "wear"} of its own`,
        fix,
      );
  }
  return out;
}

/** A decoded material carries a coverage, and an interior sheet the atlas has. */
export function assertMaterial(m: Material) {
  const c: Partial<{ kind: string; cutoff: unknown }> | undefined = m.coverage;
  const covered =
    c?.kind === "opaque" ||
    c?.kind === "blended" ||
    (c?.kind === "cutout" && typeof c.cutoff === "number");
  if (!covered) throw new Error(`material ${m.name}: no coverage`);
  if (m.interior !== undefined && !isSheet(m.interior))
    throw new Error(`material ${m.name}: unknown interior sheet ${String(m.interior)}`);
}

/** One line for a person: `glass: blended (coverage 0.350 to 0.350)`. */
export function describeMaterial(material: Material, textures: Texture[]): string {
  const { coverage } = material;
  const range = coverageRange(material, textures)
    .map((x) => x.toFixed(3))
    .join(" to ");
  const covered =
    coverage.kind === "opaque"
      ? "opaque"
      : coverage.kind === "cutout"
        ? `cutout at ${coverage.cutoff} (coverage ${range})`
        : `blended (coverage ${range})`;
  const channels = Object.keys(material.textures ?? {}).sort();
  return [
    `${material.name}: ${covered}`,
    ...(material.interior ? [`interior ${material.interior}`] : []),
    ...(material.wear ? ["wears"] : []),
    ...(channels.length ? [`textures ${channels.join(", ")}`] : []),
  ].join("; ");
}
