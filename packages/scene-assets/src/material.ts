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

/** What material `i` draws on average over the area every mesh draws it on,
 *  as the model shader does: its albedo texture's mean times the vertex
 *  colour times `colour_scale` times its base colour, except where the vertex's
 *  wear (its colour's alpha) passes the albedo texture's wear threshold (its
 *  alpha; a half without one), where the wear colour shows; and its roughness,
 *  the factor times the ORM texture's mean green. */
function drawnSurface(bundle: Exclude<Bundle, SkeletonClips>, i: number) {
  const m = bundle.materials[i];
  const albedo = m.textures?.albedo === undefined ? undefined : bundle.textures[m.textures.albedo];
  const orm = m.textures?.orm;
  const mean = albedo ? textureMean(albedo) : [1, 1, 1];
  const scale = m.colour_scale ?? 1;
  // How much of the surface is worn at each wear alpha byte: the share of
  // texels whose threshold is under it.
  const worn = new Float64Array(256);
  if (m.wear) {
    const texels = albedo?.levels[0];
    const below = new Float64Array(256);
    if (texels) for (let t = 3; t < texels.length; t += 4) below[texels[t]]++;
    else below[128] = 1;
    const total = below.reduce((a, b) => a + b, 0);
    for (let a = 1, sum = 0; a < 256; a++) worn[a] = (sum += below[a - 1]) / total;
  }
  // Each triangle weighs by its area, its colour the mean of its corners'.
  const rgb = [0, 0, 0];
  let area = 0;
  for (const [, mesh] of bundleMeshes(bundle))
    for (const draw of mesh.draws) {
      if (draw.material !== i) continue;
      const { positions: p, colors, indices } = mesh;
      const drawn = (v: number, c: number) => {
        const own = (colors[v * 4 + c] / 255) * scale * m.base_color[c] * mean[c];
        return own + worn[colors[v * 4 + 3]] * ((m.wear?.[c] ?? 0) - own);
      };
      for (let k = draw.first; k + 2 < draw.first + draw.count; k += 3) {
        const [a, b, c] = [indices[k], indices[k + 1], indices[k + 2]];
        const e = [0, 1, 2].map((j) => p[b * 3 + j] - p[a * 3 + j]);
        const f = [0, 1, 2].map((j) => p[c * 3 + j] - p[a * 3 + j]);
        const size = Math.hypot(
          e[1] * f[2] - e[2] * f[1],
          e[2] * f[0] - e[0] * f[2],
          e[0] * f[1] - e[1] * f[0],
        );
        for (let ch = 0; ch < 3; ch++)
          rgb[ch] += (size * (drawn(a, ch) + drawn(b, ch) + drawn(c, ch))) / 3;
        area += size;
      }
    }
  return {
    albedo:
      area > 0 ? rgb.map((x) => x / area) : [0, 1, 2].map((c) => scale * m.base_color[c] * mean[c]),
    roughness: m.roughness * (orm === undefined ? 1 : textureMean(bundle.textures[orm])[1]),
  };
}

/** Rubber and glass are held to what they draw. */
const judgedDark = (m: Material) => m.role === "rubber" || m.role === "glass";

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
    const { albedo, roughness } = judgedDark(m)
      ? drawnSurface(bundle, i)
      : { albedo: [0, 0, 0], roughness: 0 };
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

/** One line per material for a person, `glass: opaque; role glass, draws
 *  luminance 0.012 at roughness 0.080`: rubber and glass say what they draw. */
export function describeMaterials(bundle: Exclude<Bundle, SkeletonClips>): string[] {
  return bundle.materials.map((m, i) => {
    if (!judgedDark(m)) return describeMaterial(m, bundle.textures);
    const { albedo, roughness } = drawnSurface(bundle, i);
    const drawn = `draws luminance ${luminance(albedo).toFixed(3)} at roughness ${roughness.toFixed(3)}`;
    return describeMaterial(m, bundle.textures, drawn);
  });
}

function describeMaterial(material: Material, textures: Texture[], drawn?: string): string {
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
    ...(material.role ? [`role ${material.role}${drawn ? `, ${drawn}` : ""}`] : []),
    ...(material.wear ? ["wears"] : []),
    ...(channels.length ? [`textures ${channels.join(", ")}`] : []),
  ].join("; ");
}
