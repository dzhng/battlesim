// Material transport: how a source material's glTF alpha mode and extras
// become the bundle's coverage, interior and role metadata (`Coverage`,
// `schema.ts`), and which combinations a bundle may carry. Nothing here draws.

import type { GltfJson } from "./glb.ts";
import {
  GLASS_MAX_LUMINANCE,
  GLASS_MAX_ROUGHNESS,
  INTERIOR_SHEETS,
  MATERIAL_ROLES,
  RUBBER_MAX_LUMINANCE,
  type Bundle,
  type Coverage,
  type Finding,
  type InteriorSheet,
  type Material,
  type MaterialRole,
  type SkeletonClips,
  type Texture,
} from "./schema.ts";
import { bundleMeshes, textureMean } from "./texture.ts";

type AddFinding = (code: Finding["code"], message: string, fix: string) => void;

/** glTF's `alphaCutoff` when a `MASK` material names none. */
const DEFAULT_CUTOFF = 0.5;

const isSheet = (value: unknown): value is InteriorSheet =>
  (INTERIOR_SHEETS as readonly unknown[]).includes(value);
const isRole = (value: unknown): value is MaterialRole =>
  (MATERIAL_ROLES as readonly unknown[]).includes(value);

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

/** A glTF material's role (extras `role`), when it names one. */
export function sourceRole(m: GltfJson, name: string, add: AddFinding): MaterialRole | undefined {
  const role = m.extras?.role;
  if (role === undefined || isRole(role)) return role;
  add(
    "material.role",
    `material "${name}" has role ${JSON.stringify(role)}`,
    `name one of the material roles in the material's extras: ${MATERIAL_ROLES.join(", ")}`,
  );
  return undefined;
}

/** Linear luminance (Rec. 709). */
const luminance = ([r, g, b]: readonly number[]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

/** What each material draws on average, over every vertex any mesh draws it
 *  at, as the model shader multiplies them: its albedo texture's mean times its
 *  mean vertex colour times `colour_scale` times its base colour; and its
 *  roughness, the factor times the ORM texture's mean green. */
function drawnSurfaces(bundle: Exclude<Bundle, SkeletonClips>) {
  const sums = bundle.materials.map(() => ({ rgb: [0, 0, 0], count: 0 }));
  for (const [, mesh] of bundleMeshes(bundle))
    for (const draw of mesh.draws) {
      const seen = new Set<number>();
      for (let i = draw.first; i < draw.first + draw.count; i++) seen.add(mesh.indices[i]);
      const sum = sums[draw.material];
      for (const v of seen) for (let c = 0; c < 3; c++) sum.rgb[c] += mesh.colors[v * 4 + c] / 255;
      sum.count += seen.size;
    }
  return bundle.materials.map((m, i) => {
    const { rgb, count } = sums[i];
    const albedo = m.textures?.albedo;
    const orm = m.textures?.orm;
    const mean = albedo === undefined ? [1, 1, 1] : textureMean(bundle.textures[albedo]);
    const scale = m.colour_scale ?? 1;
    return {
      albedo: [0, 1, 2].map(
        (c) => (count ? rgb[c] / count : 1) * scale * m.base_color[c] * mean[c],
      ),
      roughness: m.roughness * (orm === undefined ? 1 : textureMean(bundle.textures[orm])[1]),
    };
  });
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
 *   own (textures, wear), or is on a body that moves;
 * - `material.role_rubber`: rubber that draws lighter than black rubber;
 * - `material.role_glass`: glass that draws lighter or rougher than sight glass.
 */
export function materialFindings(label: string, bundle: Exclude<Bundle, SkeletonClips>): Finding[] {
  const out: Finding[] = [];
  const drawn = drawnSurfaces(bundle);
  bundle.materials.forEach((m, i) => {
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
    const { albedo, roughness } = drawn[i];
    const lit = luminance(albedo);
    const tone = `draws at luminance ${lit.toFixed(3)} (albedo ${albedo.map((x) => x.toFixed(3)).join(", ")})`;
    if (m.role === "rubber" && lit > RUBBER_MAX_LUMINANCE)
      add(
        "material.role_rubber",
        `is rubber and ${tone}, over ${RUBBER_MAX_LUMINANCE}: it reads grey, not black`,
        "keep dust and paint off the rubber (`parts.tyre`); a dusty tread is dust low down, not a film",
      );
    if (m.role === "glass" && (lit > GLASS_MAX_LUMINANCE || roughness > GLASS_MAX_ROUGHNESS))
      add(
        "material.role_glass",
        `is glass and ${tone} at roughness ${roughness.toFixed(3)}, over ${GLASS_MAX_LUMINANCE} or ${GLASS_MAX_ROUGHNESS}: it reads as a coloured block, not glass`,
        "draw glass near black and smooth (`parts.glass`): what it shows is what it reflects",
      );
    if (m.interior === undefined) return;
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
  });
  return out;
}

/** A decoded material carries a coverage, an interior sheet the atlas has and
 *  a role the contract has. */
export function assertMaterial(m: Material) {
  const c: Partial<{ kind: string; cutoff: unknown }> | undefined = m.coverage;
  const covered =
    c?.kind === "opaque" ||
    c?.kind === "blended" ||
    (c?.kind === "cutout" && typeof c.cutoff === "number");
  if (!covered) throw new Error(`material ${m.name}: no coverage`);
  if (m.interior !== undefined && !isSheet(m.interior))
    throw new Error(`material ${m.name}: unknown interior sheet ${String(m.interior)}`);
  if (m.role !== undefined && !isRole(m.role))
    throw new Error(`material ${m.name}: unknown role ${String(m.role)}`);
}

/** One line for a person: `glass: blended (coverage 0.350 to 0.350); role glass`. */
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
    ...(material.role ? [`role ${material.role}`] : []),
    ...(material.wear ? ["wears"] : []),
    ...(channels.length ? [`textures ${channels.join(", ")}`] : []),
  ].join("; ");
}
