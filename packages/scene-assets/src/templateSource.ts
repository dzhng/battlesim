// City template sets, as their sources say them (`blender/city/README.md`):
// reading a set's `templates.json`, holding it to its kit, to the physical
// contract and to the physical catalogue it dresses (the map generator's, or
// the authored maps'), and packing every set into the one template art
// library (`templateLibrary.ts`).
//
// Isomorphic and Blender-free: the caller supplies bytes, the baked kits and
// the physical contract. What a legal physical template is, and what the
// catalogue's hash is, is the Rust contract's own code (`PhysicalTemplates`,
// reached through the WebAssembly); nothing here re-derives either.

import { lfsPointerOid, lfsPullCommand } from "./glb.ts";
import { MODULE_ID, triangleCount } from "./build.ts";
import { positionsBounds } from "./pose.ts";
import {
  TIER_COUNT,
  type Bounds,
  type BuildingCollapse,
  type CitySetEntry,
  type Finding,
  type StaticBundle,
} from "./schema.ts";
import {
  ALL_TIERS,
  ROW_TRANSFORM_FLOATS,
  TEMPLATE_STATES,
  TEMPLATE_STATUSES,
  type RowRange,
  type TemplateArt,
  type TemplateArtLibrary,
  type TemplateState,
  type TemplateStatus,
} from "./templateLibrary.ts";

/** One oriented box of a physical template (`contract::templates::TemplatePart`). */
export interface TemplatePart {
  id: string;
  center: [number, number];
  yaw: number;
  half_extents: [number, number, number];
  base_z: number;
}

/** The contract's `BuildingTemplateDescriptor`, as far as art reads it: its
 *  identity and its parts. Every other field is the contract's to judge. */
export interface TemplateDescriptor {
  id: string;
  category: string;
  regional_family: string;
  parts: TemplatePart[];
  [field: string]: unknown;
}

/** Numbers in a source row: module, x, y, z, yaw, sx, sy, sz, tiers, r, g, b. */
export const SOURCE_ROW_NUMBERS = 12;

/** A set's `templates.json`. */
export interface TemplateSetSource {
  set: string;
  kit: string;
  /** How far art may reach past a part: `side_m` on its four sides and
   *  `top_m` above it; `ruin_top_m` (0 when absent) above the remains a
   *  collapsed part leaves, for the jagged tops of broken walls. */
  fit: { side_m: number; top_m: number; ruin_top_m?: number };
  source?: unknown;
  modules: string[];
  templates: {
    status: TemplateStatus;
    recipe?: unknown;
    descriptor: TemplateDescriptor;
    states: Partial<Record<TemplateState, number[][]>>;
  }[];
}

/** A canonical physical catalogue: its identity and its rows in id order. */
export interface AdmittedTemplates {
  hash: string;
  templates: TemplateDescriptor[];
}

/** The physical template contract, as its own code judges it. Each throws
 *  the contract's refusal. */
export interface PhysicalTemplates {
  /** The canonical catalogue of `descriptors`, each a valid physical template
   *  (`validate`): its floors, entrances and bays may be unresolved. */
  valid(descriptors: readonly unknown[]): AdmittedTemplates;
  /** The same, each a complete building as well (`require_complete`). */
  complete(descriptors: readonly unknown[]): AdmittedTemplates;
}

/** A physical catalogue the sets dress: every row of it has art in exactly
 *  one set that names it, and every descriptor of such a set is a row of it. */
export interface TemplateCatalogue {
  /** What a set calls it (`city_sets.<set>.catalogue`). */
  name: string;
  /** Its rows: the descriptors a map of it may place. */
  rows: readonly unknown[];
  /** Whether every row is a complete building. The map generator's are: it
   *  places them by their entrances and bays. An authored map's solid boxes
   *  are not. */
  complete: boolean;
}

/** Triangles a template is budgeted to draw at each tier, finest first. The
 *  bake reports what each template draws against it; it is not yet a gate. */
export const TEMPLATE_TIER_TRIANGLES = [150_000, 50_000, 12_000, 2_000] as const;

/** Slack on the fit for single-precision transforms, in metres. */
const FIT_ROUNDING_M = 1e-3;

const error = (code: Finding["code"], message: string, fix: string): Finding => ({
  code,
  severity: "error",
  message,
  fix,
});

const isRecord = (value: unknown): value is Record<string, unknown> =>
  !!value && typeof value === "object" && !Array.isArray(value);

/** Read a set's `templates.json`: its structure only. A file that cannot be
 *  read as a set at all comes back null. */
export function readTemplateSet(
  bytes: Uint8Array,
  path: string,
): { set: TemplateSetSource | null; findings: Finding[] } {
  const fix = "write the set as packages/scene-assets/blender/city/README.md describes";
  const refuse = (finding: Finding) => ({ set: null, findings: [finding] });
  if (lfsPointerOid(bytes) !== null)
    return refuse(
      error(
        "structure.lfs_pointer",
        `${path} is a Git LFS pointer, not its content`,
        `run: ${lfsPullCommand(path)}`,
      ),
    );
  let json: unknown;
  try {
    json = JSON.parse(new TextDecoder().decode(bytes));
  } catch (e) {
    return refuse(error("templates.source", `${path}: not JSON (${(e as Error).message})`, fix));
  }
  if (!isRecord(json) || !Array.isArray(json.templates) || !Array.isArray(json.modules))
    return refuse(
      error("templates.source", `${path}: a set needs "modules" and "templates" lists`, fix),
    );
  const findings: Finding[] = [];
  const add = (code: Finding["code"], message: string, repair = fix) =>
    findings.push(error(code, `${path}: ${message}`, repair));
  if (typeof json.set !== "string" || typeof json.kit !== "string")
    add("templates.source", `"set" and "kit" must be names`);
  const fit = json.fit;
  if (
    !isRecord(fit) ||
    ![fit.side_m, fit.top_m, fit.ruin_top_m ?? 0].every(
      (v) => typeof v === "number" && Number.isFinite(v) && v >= 0,
    )
  )
    add(
      "templates.source",
      `"fit" must be { side_m, top_m } and perhaps ruin_top_m, each zero or more metres`,
    );
  const modules = json.modules as unknown[];
  for (const [i, module] of modules.entries())
    if (typeof module !== "string" || !MODULE_ID.test(module) || modules.indexOf(module) !== i)
      add(
        "templates.module",
        `modules[${i}] (${JSON.stringify(module)}) is not a module id, or repeats one`,
        "list each kit module the rows use once, by its id",
      );
  const ids = new Set<string>();
  for (const [t, template] of (json.templates as unknown[]).entries()) {
    const descriptor = isRecord(template) ? template.descriptor : null;
    if (!isRecord(template) || !isRecord(descriptor) || typeof descriptor.id !== "string") {
      add("templates.source", `templates[${t}] needs a descriptor with an id`);
      continue;
    }
    const what = `template ${descriptor.id}`;
    if (ids.has(descriptor.id)) add("templates.source", `${what} appears twice`);
    ids.add(descriptor.id);
    if (!TEMPLATE_STATUSES.includes(template.status as TemplateStatus))
      add(
        "templates.source",
        `${what}: status ${JSON.stringify(template.status)} is not one of ${TEMPLATE_STATUSES.join(", ")}`,
        `mark accepted art "release" and a labelled stand-in "prototype"`,
      );
    const states = template.states;
    if (!isRecord(states)) {
      add("templates.state", `${what} has no states`, `give it "intact" rows`);
      continue;
    }
    for (const state of Object.keys(states))
      if (!TEMPLATE_STATES.includes(state as TemplateState))
        add(
          "templates.state",
          `${what}: "${state}" is not a state (${TEMPLATE_STATES.join(", ")})`,
          "name the states a side can know a building in",
        );
    const intact = states.intact;
    if (!Array.isArray(intact) || !intact.length)
      add(
        "templates.state",
        `${what} has no "intact" rows`,
        `every template is drawn intact: give it "intact" rows`,
      );
    for (const state of TEMPLATE_STATES) {
      const rows = states[state];
      if (rows === undefined) continue;
      if (!Array.isArray(rows)) {
        add("templates.state", `${what}: "${state}" is not a list of rows`);
        continue;
      }
      const bad = rows.findIndex((row) => rowProblem(row, modules.length) !== null);
      if (bad >= 0)
        add(
          rowNamesNoModule(rows[bad], modules.length) ? "templates.module" : "templates.row",
          `${what} ${state} row ${bad}: ${rowProblem(rows[bad], modules.length)}`,
          "a row is [module, x, y, z, yaw, sx, sy, sz, tiers, r, g, b]",
        );
    }
  }
  return { set: json as unknown as TemplateSetSource, findings };
}

const rowNamesNoModule = (row: unknown, modules: number) =>
  Array.isArray(row) && Number.isInteger(row[0]) && (row[0] < 0 || row[0] >= modules);

/** What is wrong with a source row, or null. */
function rowProblem(row: unknown, modules: number): string | null {
  if (!Array.isArray(row) || row.length !== SOURCE_ROW_NUMBERS)
    return `not ${SOURCE_ROW_NUMBERS} numbers`;
  if (!row.every((v) => typeof v === "number" && Number.isFinite(v)))
    return "holds a value that is not a finite number";
  const [module, , , , , sx, sy, sz, tiers, r, g, b] = row as number[];
  if (!Number.isInteger(module)) return `module ${module} is not an index`;
  if (module < 0 || module >= modules)
    return `module ${module} is not one of the set's ${modules} modules`;
  if (!(sx > 0 && sy > 0 && sz > 0))
    return `scale [${sx}, ${sy}, ${sz}] is not positive (a mirror is a module variant of its own)`;
  if (!Number.isInteger(tiers) || tiers < 1 || tiers > ALL_TIERS)
    return `tiers ${tiers} is not a mask of 1 to ${ALL_TIERS}`;
  if (![r, g, b].every((c) => Number.isInteger(c) && c >= 0 && c <= 255))
    return `tint [${r}, ${g}, ${b}] is not sRGB bytes`;
  return null;
}

// ---------------------------------------------------------------- fit

/** A part grown by the set's fit: what its art may fill. */
interface Allowed {
  id: string;
  cx: number;
  cy: number;
  cos: number;
  sin: number;
  hx: number;
  hy: number;
  low: number;
  high: number;
}

const allowedParts = (
  parts: readonly TemplatePart[],
  fit: { side_m: number; top_m: number },
  groundM: number,
): Allowed[] =>
  parts.map((p) => ({
    id: p.id,
    cx: p.center[0],
    cy: p.center[1],
    cos: Math.cos(p.yaw),
    sin: Math.sin(p.yaw),
    hx: p.half_extents[0] + fit.side_m + FIT_ROUNDING_M,
    hy: p.half_extents[1] + fit.side_m + FIT_ROUNDING_M,
    low: p.base_z - groundM - FIT_ROUNDING_M,
    high: p.base_z + 2 * p.half_extents[2] + fit.top_m + FIT_ROUNDING_M,
  }));

/** How far a point is outside a grown part: zero or less is inside. */
function outside(a: Allowed, x: number, y: number, z: number): number {
  const [dx, dy] = [x - a.cx, y - a.cy];
  const lx = a.cos * dx + a.sin * dy;
  const ly = -a.sin * dx + a.cos * dy;
  return Math.max(Math.abs(lx) - a.hx, Math.abs(ly) - a.hy, a.low - z, z - a.high);
}

/** A row that reaches out of its template: by how far, where (in the
 *  template's frame), and the part it is nearest. */
export interface FitExcess {
  row: number;
  excess_m: number;
  at: [number, number, number];
  part: string;
}

/**
 * One state's geometry against the template's parts: every vertex a row
 * draws, at every tier it draws at, lies inside some part grown by the set's
 * `fit` (`side_m` on the four sides, `top_m` above, the catalog's ground
 * tolerance below). Returns the rows that reach out, worst first.
 */
export function fitExcess(
  rows: readonly number[][],
  moduleOf: (index: number) => StaticBundle["states"][number] | undefined,
  parts: readonly TemplatePart[],
  fit: { side_m: number; top_m: number },
  groundM: number,
): FitExcess[] {
  const allowed = allowedParts(parts, fit, groundM);
  const boxes = new Map<number, Bounds[]>();
  const out: FitExcess[] = [];
  for (const [r, row] of rows.entries()) {
    const [index, x, y, z, yaw, sx, sy, sz, tiers] = row;
    const module = moduleOf(index);
    if (!module) continue;
    let tierBoxes = boxes.get(index);
    if (!tierBoxes) {
      tierBoxes = module.tiers.map((mesh) => positionsBounds(mesh.positions));
      boxes.set(index, tierBoxes);
    }
    // The row's transform: scaled, turned about +Z, then moved.
    const [cos, sin] = [Math.cos(yaw), Math.sin(yaw)];
    const placedX = (px: number, py: number) => x + cos * px * sx - sin * py * sy;
    const placedY = (px: number, py: number) => y + sin * px * sx + cos * py * sy;
    const placedZ = (pz: number) => z + pz * sz;
    let worst = 0;
    let at: [number, number, number] = [0, 0, 0];
    let nearest = "";
    const judge = (px: number, py: number, pz: number) => {
      const [wx, wy, wz] = [placedX(px, py), placedY(px, py), placedZ(pz)];
      let least = Infinity;
      let part = "";
      for (const a of allowed) {
        const d = outside(a, wx, wy, wz);
        if (d < least) [least, part] = [d, a.id];
        if (d <= 0) return;
      }
      if (least > worst) [worst, at, nearest] = [least, [wx, wy, wz], part];
    };
    for (let t = 0; t < TIER_COUNT; t++) {
      if (!(tiers & (1 << t))) continue;
      // A module whose box sits whole inside one grown part is inside it.
      const { min, max } = tierBoxes[t];
      const inOne = allowed.some((a) => {
        for (let c = 0; c < 8; c++) {
          const [px, py, pz] = [
            c & 1 ? max[0] : min[0],
            c & 2 ? max[1] : min[1],
            c & 4 ? max[2] : min[2],
          ];
          if (outside(a, placedX(px, py), placedY(px, py), placedZ(pz)) > 0) return false;
        }
        return true;
      });
      if (inOne) continue;
      const positions = module.tiers[t].positions;
      for (let v = 0; v < positions.length; v += 3)
        judge(positions[v], positions[v + 1], positions[v + 2]);
    }
    if (worst > 0) out.push({ row: r, excess_m: worst, at, part: nearest });
  }
  return out.sort((a, b) => b.excess_m - a.excess_m || a.row - b.row);
}

// ---------------------------------------------------------------- damage states

/** How many floors a template has, as the simulation counts them. */
const floorCount = (descriptor: TemplateDescriptor): number =>
  Array.isArray(descriptor.floor_heights_m) ? descriptor.floor_heights_m.length : 1;

/** The state a destroyed template is known in: a building of the rule's
 *  `max_floors` or fewer collapses to a ruin; a taller one stands gutted. */
export const damageState = (
  descriptor: TemplateDescriptor,
  rule: BuildingCollapse,
): "ruin" | "gutted" => (floorCount(descriptor) <= rule.max_floors ? "ruin" : "gutted");

/** How tall the remains of a collapsed building `heightM` tall are: the
 *  rule's fraction of that height, between the rule's least and most. Every
 *  part falls to the one height. */
export const ruinHeight = (heightM: number, rule: BuildingCollapse): number =>
  Math.min(Math.max(heightM * rule.height_fraction, rule.min_height_m), rule.max_height_m);

/** A template's height: its highest part's top. */
const heightOf = (parts: readonly TemplatePart[]): number =>
  parts.reduce((top, p) => Math.max(top, p.base_z + 2 * p.half_extents[2]), 0);

/** The remains a collapsed template leaves: each part's plan, from its own
 *  base to the ruin height. A ruin state is held to these, as the standing
 *  states are to the parts. */
export function ruinParts(parts: readonly TemplatePart[], rule: BuildingCollapse): TemplatePart[] {
  const half = ruinHeight(heightOf(parts), rule) / 2;
  return parts.map((p) => ({ ...p, half_extents: [p.half_extents[0], p.half_extents[1], half] }));
}

// ---------------------------------------------------------------- packing

/** One catalog set, as the bake hands it over. */
export interface TemplateSetInput {
  /** The catalog's name for the set. */
  name: string;
  entry: CitySetEntry;
  /** The `templates.json` bytes. */
  bytes: Uint8Array;
  /** The set's kit as baked, and its bundle's content hash; null when the
   *  catalog has no such kit or it did not bake. */
  kit: { bundle: StaticBundle; hash: string } | null;
}

export interface TemplateLibraryStats {
  /** Per template: where it came from and what it draws intact. */
  templates: {
    id: string;
    set: string;
    status: TemplateStatus;
    rows: number;
    /** Triangles its intact rows draw at each tier, finest first. */
    triangles: number[];
    /** Its damage state, and what that draws; absent while it has none. */
    damage?: { state: TemplateState; rows: number; triangles: number[] };
  }[];
  rows: number;
  modules: number;
}

export interface PackedTemplates {
  findings: Finding[];
  /** The library without its identity (`sealTemplateLibrary`); null while
   *  any finding is an error. */
  library: Omit<TemplateArtLibrary, "art_hash"> | null;
  stats: TemplateLibraryStats | null;
}

const fmt = (n: number) => n.toFixed(3);

/**
 * Hold every set to its kit, the physical contract and its catalogue, and
 * pack them all into one library. Every row of every catalogue is dressed by
 * exactly one set, and every set's descriptor is a row of the catalogue the
 * set names. A template is known by its id alone, so no two catalogues share
 * one. `collapse` is the simulation's rule for a destroyed building: it names
 * the one damage state a template has, and the remains a ruin is held to.
 * Deterministic: sets in name order, templates in id order, rows as written.
 */
export function packTemplateSets(
  inputs: readonly TemplateSetInput[],
  catalogues: readonly TemplateCatalogue[],
  physical: PhysicalTemplates,
  groundM: number,
  collapse: BuildingCollapse,
): PackedTemplates {
  const findings: Finding[] = [];
  const admit = (catalogue: TemplateCatalogue, rows: readonly unknown[]) =>
    catalogue.complete ? physical.complete(rows) : physical.valid(rows);
  const kits = [...new Set(inputs.flatMap((i) => (i.kit ? [i.entry.kit] : [])))].sort();
  const kitHash = new Map(inputs.flatMap((i) => (i.kit ? [[i.entry.kit, i.kit.hash]] : [])));
  const modules: TemplateArtLibrary["modules"] = [];
  const moduleSlot = new Map<string, number>();
  const packed: { art: TemplateArt; rows: Partial<Record<TemplateState, number[][]>> }[] = [];
  const stats: TemplateLibraryStats["templates"] = [];
  /** Canonical descriptors by id, with the sets that dress each and the
   *  catalogue the first of them names. */
  const dressed = new Map<string, { sets: string[]; canonical: string; catalogue: string }>();
  let unread = false;

  for (const input of [...inputs].sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const path = input.entry.templates;
    const add = (code: Finding["code"], message: string, fix: string) =>
      findings.push(error(code, `${path}: ${message}`, fix));
    const read = readTemplateSet(input.bytes, path);
    findings.push(...read.findings);
    const set = read.set;
    if (!set || read.findings.length) unread = true;
    if (!set) continue;
    if (set.set !== input.name || set.kit !== input.entry.kit)
      add(
        "templates.kit",
        `the file says set "${set.set}" on kit "${set.kit}"; the catalog lists it as set "${input.name}" on kit "${input.entry.kit}"`,
        "make assets/catalog.json's city_sets entry and the set's own names agree",
      );
    if (!input.kit)
      add(
        "templates.kit",
        `kit "${input.entry.kit}" is not a baked kit appearance`,
        `add a { "unit": "kit" } appearance "${input.entry.kit}" for the set's kit.glb, and fix its findings`,
      );
    const kitStates = new Map(input.kit?.bundle.states.map((s) => [s.name, s]) ?? []);
    // The set's module indices, as the library's.
    const slots = set.modules.map((module) => {
      if (input.kit && !kitStates.has(module))
        add(
          "templates.module",
          `module "${module}" is not in kit "${input.entry.kit}"`,
          "export the module in the set's kit.glb, or drop it from the set",
        );
      const key = `${input.entry.kit}\n${module}`;
      let slot = moduleSlot.get(key);
      if (slot === undefined) {
        slot = modules.length;
        modules.push({ kit: kits.indexOf(input.entry.kit), module });
        moduleSlot.set(key, slot);
      }
      return slot;
    });
    const catalogue = catalogues.find((c) => c.name === input.entry.catalogue);
    if (!catalogue) {
      add(
        "templates.catalogue",
        `the catalog lists set "${input.name}" on catalogue "${input.entry.catalogue}", which is not one of ${catalogues.map((c) => c.name).join(", ")}`,
        "name the physical catalogue the set's templates are rows of in its city_sets entry",
      );
      unread = true;
    }
    // A set with a malformed row or template, or no catalogue, is judged no further.
    if (read.findings.length || !catalogue) continue;
    for (const template of set.templates) {
      const id = template.descriptor.id;
      let canonical: TemplateDescriptor | null = null;
      try {
        canonical = admit(catalogue, [template.descriptor]).templates[0];
      } catch (e) {
        add(
          "templates.physical",
          `template ${id} is not a physical template a map may place: ${(e as Error).message}`,
          `fix the descriptor the script writes; contract::templates::${catalogue.complete ? "require_complete" : "validate"} is the rule`,
        );
      }
      if (canonical) {
        const entry = dressed.get(id) ?? {
          sets: [],
          canonical: JSON.stringify(canonical),
          catalogue: catalogue.name,
        };
        entry.sets.push(input.name);
        dressed.set(id, entry);
      }
      const drawn = new Map<TemplateState, number[]>();
      if (canonical) {
        // Destroyed, a template is known in exactly one state: the one its floors call for.
        const ends = damageState(canonical, collapse);
        const other = ends === "ruin" ? "gutted" : "ruin";
        const what = `template ${id} (${floorCount(canonical)} floors) ${ends === "ruin" ? "collapses" : "stands gutted"}`;
        if (!template.states[ends]?.length)
          add(
            "templates.state",
            `${what}, and has no "${ends}" rows`,
            ends === "ruin"
              ? "give it the rows of its remains: the same plan, broken down to the ruin height"
              : "give it the rows of its burnt shell, standing at full height",
          );
        if (template.states[other])
          add(
            "templates.state",
            `${what}, and has "${other}" rows`,
            `drop its "${other}" state: a building of more than ${collapse.max_floors} floors stands gutted, and any other collapses`,
          );
      }
      for (const state of TEMPLATE_STATES) {
        const rows = template.states[state];
        if (!rows) continue;
        // A chunk of a town draws at one tier, and the whole map at the
        // coarsest: a state with no row at a tier is a building that vanishes.
        const undrawn = Array.from({ length: TIER_COUNT }, (_, t) => t).filter(
          (t) => !rows.some((row) => row[8] & (1 << t)),
        );
        if (undrawn.length)
          add(
            "templates.state",
            `template ${id} ${state} draws nothing at tier ${undrawn.join(", ")}`,
            "give every tier a row: fold what is left of a building into its shell at the coarse tiers",
          );
        if (!input.kit) continue;
        const moduleOf = (index: number) => kitStates.get(set.modules[index]);
        if (canonical) {
          // A ruin is held to the remains the simulation leaves; a standing state to the parts.
          const fallen = state === "ruin";
          const top = fallen ? (set.fit.ruin_top_m ?? 0) : set.fit.top_m;
          const parts = fallen ? ruinParts(canonical.parts, collapse) : canonical.parts;
          const [worst, ...more] = fitExcess(
            rows,
            moduleOf,
            parts,
            { side_m: set.fit.side_m, top_m: top },
            groundM,
          );
          const held = fallen
            ? `part "${worst?.part}" at its ruin height (${fmt(ruinHeight(heightOf(canonical.parts), collapse))} m), grown by the set's fit (side ${set.fit.side_m} m, ruin top ${top} m)`
            : `part "${worst?.part}" grown by the set's fit (side ${set.fit.side_m} m, top ${top} m)`;
          if (worst)
            add(
              "templates.fit",
              `template ${id} ${state}: row ${worst.row} (module "${set.modules[rows[worst.row][0]]}") reaches [${worst.at.map(fmt).join(", ")}], ${fmt(worst.excess_m)} m outside ${held}${more.length ? `; ${more.length} more row(s) reach out` : ""}`,
              fallen
                ? "keep a ruin inside the remains the simulation leaves: lower the row, or give the set's fit a ruin_top_m for the jagged tops of broken walls"
                : "keep the art on its parts: move the row, or change the descriptor it is made to; widen the set's fit only for what a real building overhangs",
            );
        }
        const triangles = Array.from({ length: TIER_COUNT }, () => 0);
        for (const row of rows) {
          const module = moduleOf(row[0]);
          for (let t = 0; module && t < TIER_COUNT; t++)
            if (row[8] & (1 << t)) triangles[t] += triangleCount(module.tiers[t]);
        }
        drawn.set(state, triangles);
      }
      const damaged = TEMPLATE_STATES.find((state) => state !== "intact" && drawn.has(state));
      packed.push({
        art: { id, set: input.name, status: template.status, states: {} },
        rows: Object.fromEntries(
          TEMPLATE_STATES.flatMap((state) => {
            const rows = template.states[state];
            return rows ? [[state, rows.map((row) => [slots[row[0]], ...row.slice(1)])]] : [];
          }),
        ),
      });
      stats.push({
        id,
        set: input.name,
        status: template.status,
        rows: template.states.intact?.length ?? 0,
        triangles: drawn.get("intact") ?? Array.from({ length: TIER_COUNT }, () => 0),
        ...(damaged
          ? {
              damage: {
                state: damaged,
                rows: template.states[damaged]!.length,
                triangles: drawn.get(damaged)!,
              },
            }
          : {}),
      });
    }
  }

  // Each catalogue and its sets, one against the other.
  const covers: string[] = [];
  /** Per template id: the catalogue that has it. */
  const home = new Map<string, string>();
  for (const catalogue of [...catalogues].sort((a, b) => (a.name < b.name ? -1 : 1))) {
    let admitted: AdmittedTemplates;
    try {
      admitted = admit(catalogue, catalogue.rows);
    } catch (e) {
      findings.push(
        error(
          "templates.catalogue",
          `the physical catalogue "${catalogue.name}" is refused: ${(e as Error).message}`,
          "fix the catalogue's rows; contract::templates is the rule",
        ),
      );
      continue;
    }
    covers.push(admitted.hash);
    const rows = new Map(admitted.templates.map((t) => [t.id, JSON.stringify(t)]));
    for (const id of rows.keys()) {
      const other = home.get(id);
      if (other !== undefined)
        findings.push(
          error(
            "templates.catalogue",
            `template ${id} is in the physical catalogues "${other}" and "${catalogue.name}"; art is found by a template's id alone`,
            "give the template of one catalogue another id",
          ),
        );
      home.set(id, catalogue.name);
    }
    for (const [id, entry] of dressed) {
      if (entry.catalogue !== catalogue.name) continue;
      const row = rows.get(id);
      if (row !== entry.canonical)
        findings.push(
          error(
            "templates.catalogue",
            `set ${entry.sets.join(", ")}: template ${id} ${row === undefined ? "is not in the physical catalogue" : "differs from its row in the physical catalogue"} "${catalogue.name}"`,
            catalogue.complete
              ? "the catalogue's rows are the sets' descriptors: run `bun run --cwd web asset -- catalogue`"
              : "write the set's descriptor as the catalogue's row is written",
          ),
        );
    }
    // Which templates lack art is only known once every set has been read.
    for (const id of unread ? [] : rows.keys())
      if (dressed.get(id)?.catalogue !== catalogue.name)
        findings.push(
          error(
            "templates.coverage",
            `template ${id} is in the physical catalogue "${catalogue.name}", but no set of it has art for it`,
            catalogue.complete
              ? "add it to a set, or give it stand-in rows: run `bun run --cwd web asset -- prototypes`"
              : "add it to a set that names this catalogue",
          ),
        );
  }
  for (const [id, entry] of dressed)
    if (entry.sets.length > 1)
      findings.push(
        error(
          "templates.coverage",
          `template ${id} is dressed by ${entry.sets.length} sets (${entry.sets.join(", ")}); exactly one set owns a template`,
          "drop the template from all but one set",
        ),
      );
  if (modules.length > 0xffff)
    findings.push(
      error(
        "templates.module",
        `the sets name ${modules.length} modules; a row indexes at most 65535`,
        "widen the library's module column",
      ),
    );

  if (findings.some((f) => f.severity === "error")) return { findings, library: null, stats: null };
  packed.sort((a, b) => (a.art.id < b.art.id ? -1 : 1));
  const count = packed.reduce(
    (n, t) => n + Object.values(t.rows).reduce((m, rows) => m + rows.length, 0),
    0,
  );
  const rows: TemplateArtLibrary["rows"] = {
    module: new Uint16Array(count),
    transform: new Float32Array(count * ROW_TRANSFORM_FLOATS),
    tiers: new Uint8Array(count),
    tint: new Uint8Array(count * 3),
  };
  let at = 0;
  for (const template of packed)
    for (const state of TEMPLATE_STATES) {
      const source = template.rows[state];
      if (!source) continue;
      const range: RowRange = { first: at, count: source.length };
      template.art.states[state] = range;
      for (const row of source) {
        rows.module[at] = row[0];
        rows.transform.set(row.slice(1, 1 + ROW_TRANSFORM_FLOATS), at * ROW_TRANSFORM_FLOATS);
        rows.tiers[at] = row[8];
        rows.tint.set(row.slice(9, 12), at * 3);
        at++;
      }
    }
  return {
    findings,
    library: {
      covers,
      kits: kits.map((appearance) => ({ appearance, bundle: kitHash.get(appearance)! })),
      modules,
      templates: packed.map((t) => t.art),
      rows,
    },
    stats: {
      templates: stats.sort((a, b) => (a.id < b.id ? -1 : 1)),
      rows: count,
      modules: modules.length,
    },
  };
}

// ---------------------------------------------------------------- the catalogue

/**
 * The physical catalogue's rows from the sets' descriptors: every set's
 * descriptors and nothing else. A row the catalogue already has, unchanged
 * as the contract reads it, stays as it is written and where it is; a
 * changed one is replaced in place; new ones follow in set then template
 * order; a row no set has is dropped.
 */
export function catalogueRows(
  sets: readonly TemplateSetSource[],
  existing: readonly TemplateDescriptor[],
  physical: PhysicalTemplates,
): TemplateDescriptor[] {
  const canonical = (descriptor: unknown) =>
    JSON.stringify(physical.complete([descriptor]).templates[0]);
  const wanted = new Map<string, TemplateDescriptor>();
  for (const set of [...sets].sort((a, b) => (a.set < b.set ? -1 : 1)))
    for (const template of set.templates) {
      if (wanted.has(template.descriptor.id))
        throw new Error(`template ${template.descriptor.id} is in more than one set`);
      wanted.set(template.descriptor.id, template.descriptor);
    }
  const out: TemplateDescriptor[] = [];
  for (const row of existing) {
    const next = wanted.get(row.id);
    if (!next) continue;
    wanted.delete(row.id);
    out.push(canonical(row) === canonical(next) ? row : next);
  }
  return [...out, ...wanted.values()];
}

/** The catalogue file's text: a descriptor's fields a line each, each part,
 *  entrance, edge and join on one line, and every number written as the
 *  real it is (`3.0`). */
export function catalogueText(rows: readonly TemplateDescriptor[]): string {
  const inline = (value: unknown): string => {
    if (typeof value === "number") return Number.isInteger(value) ? value.toFixed(1) : `${value}`;
    if (Array.isArray(value)) return `[${value.map(inline).join(", ")}]`;
    if (isRecord(value))
      return `{${Object.entries(value)
        .map(([k, v]) => `${JSON.stringify(k)}: ${inline(v)}`)
        .join(", ")}}`;
    return JSON.stringify(value);
  };
  const field = (value: unknown): string =>
    Array.isArray(value) && value.length && value.every(isRecord)
      ? `[\n${value.map((v) => `      ${inline(v)}`).join(",\n")}\n    ]`
      : inline(value);
  const row = (descriptor: TemplateDescriptor) =>
    `  {\n${Object.entries(descriptor)
      .map(([k, v]) => `    ${JSON.stringify(k)}: ${field(v)}`)
      .join(",\n")}\n  }`;
  return `[\n${rows.map(row).join(",\n")}\n]\n`;
}
