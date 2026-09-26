// Where the workbench's models come from, both through the one loader
// (`AppearanceLibrary`):
// - a catalog appearance: the baked runtime catalog served at the site root;
// - a dropped GLB: validated in the page with the CLI's own `validateLoose`,
//   its preview bundle encoded and served from memory, so it installs exactly
//   as a baked bundle would, findings and all.

import village from "@fixtures/village.json";
import catalogJson from "../../../../assets/catalog.json";
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
  UnitKind,
} from "@packages/scene-assets/src/schema";
import type { Stats } from "@packages/scene-assets/src/validate";
import type { MountRole, UnitKindName } from "@packages/battle-renderer/src/models/poseDriver";
import { AUTHORITY } from "./benchWorld";

export const CATALOG = catalogJson as unknown as Catalog;
const PROVENANCE = (manifest as { third_party: ProvenanceEntry[] }).third_party;

export interface LoadedModel {
  /** Appearance name in `installed`. */
  name: string;
  unit: UnitKind;
  installed: InstalledAppearances;
  /** Where it came from: "catalog" or the dropped file's name. */
  source: string;
  findings: { from: string; findings: Finding[] }[];
  stats: Stats | null;
  clipStats: Stats | null;
  /** Milliseconds from bytes to installed. */
  loadMs: number;
}

/** Every appearance in the baked runtime catalog, installed at once. */
export async function loadCatalog(library: AppearanceLibrary): Promise<InstalledAppearances> {
  return library.load("/");
}

export function catalogModel(installed: InstalledAppearances, name: string): LoadedModel | null {
  const entry = installed.appearances.get(name);
  if (!entry) return null;
  return {
    name,
    unit: entry.unit,
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
                bundle: preview,
                clips: result.clips?.preview ?? undefined,
              },
            ]
          : [],
      ),
      MEMORY,
    ),
  );
  const installed = await library.load(MEMORY);
  return {
    name: file,
    unit: result.unit,
    installed,
    source: file,
    findings,
    stats: result.appearance?.stats ?? null,
    clipStats: result.clips?.stats ?? null,
    loadMs: performance.now() - started,
  };
}

/** Mount roles per unit kind from the rules' mount lists: the first turret
 *  mount is the gun, an HMG turret mount the HMG, anything else in hand. */
export function mountRoles(): Partial<Record<UnitKindName, MountRole[]>> {
  const out: Partial<Record<UnitKindName, MountRole[]>> = {};
  for (const [kind, mounts] of Object.entries(village.mounts) as [
    UnitKindName,
    { name: string; turret?: boolean }[],
  ][]) {
    let gun = false;
    out[kind] = mounts.map((m) => {
      if (!m.turret) return "hand";
      if (/hmg/i.test(m.name)) return "hmg";
      if (!gun) {
        gun = true;
        return "gun";
      }
      return "hand";
    });
  }
  return out;
}

/** Half the gauge of each vehicle kind's running gear, from its hit box. */
export const HALF_TRACK: Partial<Record<UnitKindName, number>> = {
  tank: village.physics.tank_half_extents_m[1] * 0.8,
  supply: village.physics.supply_half_extents_m[1] * 0.75,
};
