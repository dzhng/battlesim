import type { MapDefinition } from "@web/maps/resolve";
import type { GameRules } from "@web/battle/catalog/compose";
import type { Faction, UnitCatalog } from "@packages/scene-assets/src/units";
import type { SideName, Engagement } from "@web/battle/sim/protocol";

export type BuilderController = "human" | "active_ai" | "passive_ai";
export type BuilderMode = "sandbox" | "economy";

export interface BuilderUnit {
  id: number;
  side: SideName;
  kind: string;
  position: [number, number];
  yaw: number;
  engagement?: Engagement;
}

export interface BuilderSlot {
  id: string;
  side: SideName;
  faction?: Faction;
  controller: BuilderController;
  human_capable: boolean;
  units: number[];
}

export interface BuilderAsset {
  id: string;
  media_type: string;
  sha256: string;
  provenance: string;
  bytes_base64: string;
}

export interface ScenarioBundleDraft {
  format_version: "scenario-bundle-1";
  runtime_build: string;
  map: { map_hash: string; template_catalog_hash?: string | null };
  rules_hash: string;
  scenario: {
    skirmish?: unknown;
    map: MapDefinition;
    rules: GameRules;
    units: BuilderUnit[];
    events: unknown[];
    scripts: unknown[];
    opponent?: unknown;
    encounter?: unknown;
  };
  slots: BuilderSlot[];
  mode: BuilderMode;
  victory: { source: string };
  authored_scripts: { id: string; source: string }[];
  assets: BuilderAsset[];
}

/** A compact resolved-map snapshot for the first authoring checkpoint. */
export function emptyBuilderMap(): MapDefinition {
  return {
    size: [640, 480],
    fog_cell_m: 8,
    height_grid_m: 4,
    slope_cutoff_deg: 35,
    relief: [],
    surfaces: [],
    bridges: [],
    forests: [],
    props: [],
  };
}

export function createBuilderDraft(rules: GameRules, units: UnitCatalog): ScenarioBundleDraft {
  const blueKind = units.ids[0] ?? "";
  const redKind = units.ids.find((kind) => kind !== blueKind) ?? blueKind;
  const authored: BuilderUnit[] = [
    { id: 0, side: "blue", kind: blueKind, position: [120, 240], yaw: 0 },
    { id: 1, side: "red", kind: redKind, position: [520, 240], yaw: Math.PI },
  ];
  return {
    format_version: "scenario-bundle-1",
    runtime_build: "builder-draft",
    map: { map_hash: "", template_catalog_hash: null },
    rules_hash: "",
    scenario: {
      map: emptyBuilderMap(),
      rules,
      units: authored,
      events: [],
      scripts: [],
    },
    slots: [
      {
        id: "blue-1",
        side: "blue",
        faction: "us",
        controller: "human",
        human_capable: true,
        units: [0],
      },
      {
        id: "neutral-1",
        side: "neutral",
        controller: "passive_ai",
        human_capable: false,
        units: [],
      },
      {
        id: "red-1",
        side: "red",
        faction: "eastern",
        controller: "active_ai",
        human_capable: false,
        units: [1],
      },
    ],
    mode: "sandbox",
    victory: { source: "return false" },
    authored_scripts: [],
    assets: [],
  };
}

export function cloneDraft(draft: ScenarioBundleDraft): ScenarioBundleDraft {
  return structuredClone(draft);
}

export function parseBuilderDraft(text: string): ScenarioBundleDraft {
  const parsed = JSON.parse(text) as ScenarioBundleDraft;
  if (
    !parsed ||
    parsed.format_version !== "scenario-bundle-1" ||
    !parsed.scenario ||
    !Array.isArray(parsed.scenario.units) ||
    !Array.isArray(parsed.slots)
  )
    throw new Error("the uploaded package is not a scenario bundle draft");
  parsed.scenario.units = parsed.scenario.units.map((unit, index) => ({
    ...unit,
    id: Number.isInteger(unit.id) ? unit.id : index,
  }));
  parsed.slots = parsed.slots.map((slot) => ({
    ...slot,
    units: [...slot.units].sort((a, b) => a - b),
  }));
  return parsed;
}

export function unitName(units: UnitCatalog, kind: string): string {
  try {
    return units.type(kind).name;
  } catch {
    return kind;
  }
}
