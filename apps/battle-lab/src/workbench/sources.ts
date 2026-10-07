// Where the workbench's models come from, both through the one loader
// (`AppearanceLibrary`):
// - a catalog appearance: the baked runtime catalog served at the site root,
//   the page's one load of it (`gameAppearances`), and a kit fetched when it
//   is the one shown;
// - a dropped GLB: validated in the page with the CLI's own `validateLoose`,
//   its preview bundle encoded and served from memory, so it installs exactly
//   as a baked bundle would, findings and all.

import type { GameRules } from "@web/battle/catalog/compose";
import catalogJson from "../../../../assets/catalog.json";
import type { Vec3 } from "math";
import { previewRuntime } from "@packages/scene-assets/src/bake";
import {
  AppearanceLibrary,
  memoryFetch,
  type InstalledAppearances,
} from "@packages/scene-assets/src/loader";
import { validateLoose, type LooseOptions } from "@packages/scene-assets/src/loose";
import { INFANTRY_CLIPS } from "@packages/scene-assets/src/schema";
import { reloadGameAppearances, gameAppearances } from "../gameAppearances";
import type { AppearanceUnit, Catalog, Finding, Side } from "@packages/scene-assets/src/schema";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { Stats } from "@packages/scene-assets/src/validate";
import { benchAuthority, footprint, type Footprint, type PropClasses } from "./benchWorld";
import { loadWasm } from "@web/battle/sim/module";
import { loadMap } from "@web/maps/browser";

export const CATALOG = catalogJson as unknown as Catalog;

/** The side's tint on a model's tint-masked surfaces, as the battle tints a
 *  unit's (`AppearanceCatalog`); none for things drawn per placement. */
export function sideTint(model: LoadedModel, side: Side): Vec3 | undefined {
  return model.unit === "soldier" || model.unit === "vehicle"
    ? model.installed.sides[side]
    : undefined;
}

/** The unit type a catalog appearance shows on the bench: the first hull
 *  type it draws, or the first squad type whose soldier or equipment set wears it. */
function typeDrawing(units: UnitCatalog, name: string): string | null {
  return (
    units.ids.find((id) => units.type(id).appearance === name) ??
    units.ids.find(
      (id) =>
        units.slots(id).some((kind) => units.soldier(kind).appearance.includes(name)) ||
        units
          .type(id)
          .mounts.some((mount) =>
            [
              ...(mount.operator_appearance?.active ?? []),
              ...(mount.operator_appearance?.carried ?? []),
            ].includes(name),
          ),
    ) ??
    null
  );
}

export interface LoadedModel {
  /** Appearance name in `installed`. */
  name: string;
  unit: AppearanceUnit;
  /** The unit type it is shown and replayed as: the one it was fitted to. */
  type: string | null;
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

/** The simulation's prop classes (what blocks whom, what hides sight) under
 *  the page's `rules`, read once from `world_layout()`, with the boxes the
 *  street test map places. */
export async function loadPropClasses(rules: GameRules): Promise<PropClasses> {
  if (!propClasses) {
    const [wasm, street] = await Promise.all([loadWasm(), loadMap("street")]);
    propClasses = {
      ...(JSON.parse(wasm.world_layout(JSON.stringify(rules))) as PropClasses),
      placed: street.definition.props,
    };
  }
  return propClasses;
}

/** The baked runtime catalog as the page loads it for `units` (the
 *  scenery and their art installed at once): the page's load, or `fresh`
 *  after a re-bake. */
export async function loadCatalog(
  { units, rules }: { units: UnitCatalog; rules: GameRules },
  fresh = false,
): Promise<InstalledAppearances> {
  await loadPropClasses(rules);
  return fresh ? reloadGameAppearances(units) : gameAppearances(units);
}

/** Every appearance the catalog names: those installed, and those fetched on request. */
export const catalogNames = (installed: InstalledAppearances): string[] =>
  [...new Set([...installed.appearances.keys(), ...installed.onRequest.keys()])].sort();

/** The clip roles that loop, for a dropped GLB's clips: every one but the fall. */
export const INFANTRY_LOOPS: string[] = INFANTRY_CLIPS.filter((clip) => clip !== "death");

/** The catalog's appearance `name` as a bench model, or null when it names
 *  none. One fetched on request (a kit, a regional look) is fetched to be shown. */
export async function catalogModel(
  units: UnitCatalog,
  catalog: InstalledAppearances,
  name: string,
): Promise<LoadedModel | null> {
  const installed = catalog.onRequest.has(name) ? await gameAppearances(units, [name]) : catalog;
  const entry = installed.appearances.get(name);
  if (!entry) return null;
  const type = typeDrawing(units, name);
  return {
    name,
    unit: entry.unit,
    type,
    scenery: entry.scenery,
    body: footprint(units, entry.unit, entry.scenery, propClasses, entry.footprint, type),
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
  { units, rules }: { units: UnitCatalog; rules: GameRules },
  file: string,
  bytes: Uint8Array,
  options: LooseOptions,
): Promise<LoadedModel> {
  const started = performance.now();
  const result = await validateLoose(
    file,
    bytes,
    CATALOG,
    { authority: benchAuthority(units), tolerances: CATALOG.tolerances },
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
  const drawn =
    preview && preview.kind !== "clips"
      ? [
          {
            name: file,
            unit: result.unit,
            scenery: options.scenery,
            mounts: result.mounts ?? undefined,
            bundle: preview,
            clips: result.clips?.preview ?? undefined,
          },
        ]
      : [];
  const library = new AppearanceLibrary(
    memoryFetch(await previewRuntime(drawn, CATALOG.sides), MEMORY),
  );
  await library.load(MEMORY);
  // A dropped kit is asked for, like any kit that is to be drawn.
  const installed = await library.withAppearances(drawn.map((entry) => entry.name));
  const type = options.type ?? (result.entryName ? typeDrawing(units, result.entryName) : null);
  return {
    name: file,
    unit: result.unit,
    type,
    scenery: options.scenery ?? null,
    body: footprint(
      units,
      result.unit,
      options.scenery ?? null,
      await loadPropClasses(rules),
      null,
      type,
    ),
    installed,
    source: file,
    findings,
    stats: result.appearance?.stats ?? null,
    clipStats: result.clips?.stats ?? null,
    loadMs: performance.now() - started,
  };
}
