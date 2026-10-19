import type { MapDefinition } from "@web/maps/resolve";
import type { GameRules } from "@web/battle/catalog/compose";
import { FACTIONS, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { SideName, Engagement } from "@web/battle/sim/protocol";
import type { Player } from "@web/battle/prepare/protocol";
import type matchRules from "@fixtures/skirmish.json";
import { entryCountForPlayers } from "../skirmishSetup";

export type BuilderController = "human" | "active_ai" | "passive_ai";
export type BuilderMode = "sandbox" | "economy";

export interface BuilderUnit {
  id: number;
  side: SideName;
  kind: string;
  position: [number, number];
  yaw: number;
  engagement?: Engagement;
  owner?: string;
}

export interface BuilderAsset {
  id: string;
  media_type: string;
  sha256: string;
  provenance: string;
  bytes_base64: string;
}

export interface ScenarioBundleDraft {
  format_version: "scenario-bundle-2";
  runtime_build: string;
  map: { map_hash: string; template_catalog_hash?: string | null };
  rules_hash: string;
  scenario: {
    skirmish?: BuilderSkirmish;
    map: MapDefinition;
    rules: GameRules;
    units: BuilderUnit[];
    players: Player[];
    events: unknown[];
    scripts: unknown[];
    opponent?: unknown;
    encounter?: unknown;
  };
  mode: BuilderMode;
  victory: { source: string };
  authored_scripts: { id: string; source: string; side: SideName; player?: string | null }[];
  assets: BuilderAsset[];
}

export const BUILDER_FACTIONS = FACTIONS;

export interface BuilderSkirmish {
  /** The match rules (`contract::skirmish::MatchRules`). */
  rules: typeof matchRules;
  sites: {
    entries: { side: "blue" | "red"; center: [number, number]; yaw: number }[];
    objectives: {
      id: string;
      center: [number, number];
      radius_m: number;
      kind: "junction" | "field";
      counterpart: string | null;
    }[];
    candidates: never[];
  };
  openings: Record<string, unknown>;
}

/** The simulation calls a skirmish draft's setup comes from. */
export interface SkirmishSimulation {
  synthetic_sites(size: string, entriesPerSide: number): string;
  PreparedWorld: new (
    map: string,
    rules: string,
  ) => { skirmish_fields(sites: string, players: string): string; free(): void };
}

/** The builder's authoring overrides to the starter rules: a short opening,
 *  to try a draft quickly. */
const BUILDER_RULES = { starting_minutes: 2, preparation_s: 20 } as const;

/** The skirmish `scenario` fields: the simulation's own setup (`setup_for`,
 *  through `skirmish_fields`) on its synthetic sites for the map's size and
 *  the players' entry count, with only the builder's rule overrides on top. */
export function builderEconomy(
  simulation: SkirmishSimulation,
  scenario: Pick<ScenarioBundleDraft["scenario"], "map" | "rules" | "players">,
): BuilderSkirmish {
  const skirmish: unknown = JSON.parse(
    simulation.synthetic_sites(
      JSON.stringify(scenario.map.size),
      entryCountForPlayers(scenario.players),
    ),
  );
  const world = new simulation.PreparedWorld(
    JSON.stringify(scenario.map),
    JSON.stringify(scenario.rules),
  );
  try {
    const fields = JSON.parse(
      world.skirmish_fields(
        JSON.stringify({ settlements: [], approaches: [], skirmish }),
        JSON.stringify(scenario.players),
      ),
    ) as { skirmish: BuilderSkirmish };
    return { ...fields.skirmish, rules: { ...fields.skirmish.rules, ...BUILDER_RULES } };
  } finally {
    world.free();
  }
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

export function nextPlayerId(players: readonly Player[], side: SideName): string {
  const prefix = `${side}-`;
  let n = 1;
  while (players.some((player) => player.id === `${prefix}${n}`)) n++;
  return `${prefix}${n}`;
}

export function createBuilderDraft(rules: GameRules, units: UnitCatalog): ScenarioBundleDraft {
  const blueKind = units.ids[0] ?? "";
  const redKind = blueKind;
  const authored: BuilderUnit[] = [
    { id: 0, side: "blue", kind: blueKind, position: [120, 240], yaw: 0, owner: "blue-1" },
    { id: 1, side: "red", kind: redKind, position: [520, 240], yaw: Math.PI, owner: "red-1" },
  ];
  return {
    format_version: "scenario-bundle-2",
    runtime_build: "builder-draft",
    map: { map_hash: "", template_catalog_hash: null },
    rules_hash: "",
    scenario: {
      map: emptyBuilderMap(),
      rules,
      units: authored,
      players: [
        {
          id: "blue-1",
          side: "blue",
          faction: "us",
          controller: "human",
          human_capable: true,
        },
        {
          id: "neutral-1",
          side: "neutral",
          controller: "passive_ai",
          human_capable: false,
        },
        {
          id: "red-1",
          side: "red",
          faction: "eastern",
          controller: "active_ai",
          human_capable: false,
        },
      ],
      events: [],
      scripts: [],
    },
    mode: "sandbox",
    victory: { source: "return false" },
    authored_scripts: [],
    assets: [],
  };
}

export function cloneDraft(draft: ScenarioBundleDraft): ScenarioBundleDraft {
  return structuredClone(draft);
}

/** Remove one editor unit and keep the portable unit indexes contiguous. */
export function removeBuilderUnit(
  draft: ScenarioBundleDraft,
  removedId: number,
): ScenarioBundleDraft {
  return {
    ...draft,
    scenario: {
      ...draft.scenario,
      units: draft.scenario.units
        .filter((unit) => unit.id !== removedId)
        .map((unit, index) => ({ ...unit, id: index })),
    },
  };
}

export function parseBuilderDraft(text: string): ScenarioBundleDraft {
  const parsed = JSON.parse(text) as ScenarioBundleDraft;
  if (
    !parsed ||
    parsed.format_version !== "scenario-bundle-2" ||
    !parsed.scenario ||
    !Array.isArray(parsed.scenario.units) ||
    !Array.isArray(parsed.scenario.players)
  )
    throw new Error("the uploaded package is not a scenario bundle draft");
  parsed.scenario.units = parsed.scenario.units.map((unit, index) => ({
    ...unit,
    id: Number.isInteger(unit.id) ? unit.id : index,
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
