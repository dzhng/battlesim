// Where the workbench's models come from, both through the one loader
// (`AppearanceLibrary`):
// - a catalog appearance: the baked runtime catalog served at the site root;
// - a dropped GLB: validated in the page with the CLI's own `validateLoose`,
//   its preview bundle encoded and served from memory, so it installs exactly
//   as a baked bundle would, findings and all.

import catalogJson from "../../../../assets/catalog.json";
import village from "@fixtures/village.json";
import type { Vec3 } from "math";
import { AppearanceCatalog } from "@packages/scene-assets/src/appearanceCatalog";
import manifest from "../../../../specs/battle-look/assets/reuse-manifest.json";
import { previewRuntime } from "@packages/scene-assets/src/bake";
import {
  AppearanceLibrary,
  memoryFetch,
  type InstalledAppearances,
} from "@packages/scene-assets/src/loader";
import { validateLoose, type LooseOptions } from "@packages/scene-assets/src/loose";
import type {
  Catalog,
  Finding,
  ProvenanceEntry,
  Side,
  UnitKind,
} from "@packages/scene-assets/src/schema";
import type { Stats } from "@packages/scene-assets/src/validate";
import { AUTHORITY, footprint, type Footprint, type PropClasses } from "./benchWorld";
import { loadWasm } from "@web/battle/sim/module";

export const CATALOG = catalogJson as unknown as Catalog;

/** The side's tint on a model's tint-masked surfaces, as the battle resolves it
 *  (`AppearanceCatalog`); none for units drawn per placement. */
export function sideTint(model: LoadedModel, side: Side): Vec3 | undefined {
  return new AppearanceCatalog(model.installed).resolve(model.unit, side)?.tint;
}
const PROVENANCE = (manifest as { third_party: ProvenanceEntry[] }).third_party;

export interface LoadedModel {
  /** Appearance name in `installed`. */
  name: string;
  unit: UnitKind;
  /** For scenery: the kind (`SCENERY_KINDS`). */
  scenery: string | null;
  /** What the simulation knows of it, drawn beside it. */
  body: Footprint;
  installed: InstalledAppearances;
  /** Where it came from: "catalog" or the dropped file's name. */
  source: string;
  findings: { from: string; findings: Finding[] }[];
  stats: Stats | null;
  clipStats: Stats | null;
  /** Milliseconds from bytes to installed. */
  loadMs: number;
}

let propClasses: PropClasses | null = null;

/** The simulation's prop classes (what blocks whom, what hides sight), read
 *  once from `world_layout()`. */
export async function loadPropClasses(): Promise<PropClasses> {
  if (!propClasses) {
    const wasm = await loadWasm();
    propClasses = JSON.parse(wasm.world_layout(JSON.stringify(village.props))) as PropClasses;
  }
  return propClasses;
}

/** Every appearance in the baked runtime catalog, installed at once. */
export async function loadCatalog(library: AppearanceLibrary): Promise<InstalledAppearances> {
  await loadPropClasses();
  return library.load("/");
}

export function catalogModel(installed: InstalledAppearances, name: string): LoadedModel | null {
  const entry = installed.appearances.get(name);
  if (!entry) return null;
  return {
    name,
    unit: entry.unit,
    scenery: entry.scenery,
    body: footprint(entry.unit, entry.scenery, propClasses, entry.footprint),
    installed,
    source: "catalog",
    findings: [],
    stats: null,
    clipStats: null,
    loadMs: 0,
  };
}

const MEMORY = "memory:/";

/** Validate a dropped GLB and install its preview through the loader. */
export async function loadDropped(
  file: string,
  bytes: Uint8Array,
  options: LooseOptions,
): Promise<LoadedModel> {
  const started = performance.now();
  const result = await validateLoose(
    file,
    bytes,
    CATALOG,
    { authority: AUTHORITY, tolerances: CATALOG.tolerances, provenance: PROVENANCE },
    options,
  );
  const findings: LoadedModel["findings"] = [];
  if (result.clips)
    findings.push({ from: `${result.clips.path} (clips)`, findings: result.clips.findings });
  if (result.appearance) {
    // A body that carries its own clips is read twice; say each thing once.
    const said = new Set(findings.flatMap((g) => g.findings.map((f) => `${f.code}|${f.message}`)));
    findings.push({
      from: file,
      findings: result.appearance.findings.filter((f) => !said.has(`${f.code}|${f.message}`)),
    });
  }
  const preview = result.appearance?.preview;
  const library = new AppearanceLibrary(
    memoryFetch(
      await previewRuntime(
        preview && preview.kind !== "clips"
          ? [
              {
                name: file,
                unit: result.unit,
                scenery: options.scenery,
                bundle: preview,
                clips: result.clips?.preview ?? undefined,
              },
            ]
          : [],
        CATALOG.sides,
      ),
      MEMORY,
    ),
  );
  const installed = await library.load(MEMORY);
  return {
    name: file,
    unit: result.unit,
    scenery: options.scenery ?? null,
    body: footprint(result.unit, options.scenery ?? null, await loadPropClasses()),
    installed,
    source: file,
    findings,
    stats: result.appearance?.stats ?? null,
    clipStats: result.clips?.stats ?? null,
    loadMs: performance.now() - started,
  };
}
