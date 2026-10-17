import type { MapDefinition } from "@web/maps/resolve";
import type { GameRules } from "@web/battle/catalog/compose";
import { FACTIONS, type Faction, type UnitCatalog } from "@packages/scene-assets/src/units";
import type { SideName, Engagement } from "@web/battle/sim/protocol";
import matchRules from "@fixtures/skirmish.json";

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

export interface BuilderPlayer {
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
  format_version: "scenario-bundle-2";
  runtime_build: string;
  map: { map_hash: string; template_catalog_hash?: string | null };
  rules_hash: string;
  scenario: {
    skirmish?: BuilderSkirmish;
    map: MapDefinition;
    rules: GameRules;
    units: BuilderUnit[];
    events: unknown[];
    scripts: unknown[];
    opponent?: unknown;
    encounter?: unknown;
  };
  players: BuilderPlayer[];
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
    entries: [
      { side: "blue"; center: [number, number]; yaw: number },
      { side: "red"; center: [number, number]; yaw: number },
    ];
    objectives: {
      id: string;
      center: [number, number];
      radius_m: number;
      kind: "junction" | "field";
      counterpart: string | null;
    }[];
    candidates: never[];
  };
}

export function builderEconomy(size: readonly [number, number]): BuilderSkirmish {
  const [width, depth] = size;
  return {
    rules: { ...matchRules, starting_minutes: 2, preparation_s: 20 },
    sites: {
      entries: [
        { side: "blue", center: [width / 2, 12], yaw: Math.PI / 2 },
        { side: "red", center: [width / 2, depth - 12], yaw: -Math.PI / 2 },
      ],
      objectives: [
        {
          id: "center",
          center: [width / 2, depth / 2],
          radius_m: 50,
          kind: "junction",
          counterpart: null,
        },
        {
          id: "west",
          center: [width * 0.25, depth / 2],
          radius_m: 50,
          kind: "field",
          counterpart: "east",
        },
        {
          id: "east",
          center: [width * 0.75, depth / 2],
          radius_m: 50,
          kind: "field",
          counterpart: "west",
        },
      ],
      candidates: [],
    },
  };
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

export function nextPlayerId(players: readonly BuilderPlayer[], side: SideName): string {
  const prefix = `${side}-`;
  let n = 1;
  while (players.some((player) => player.id === `${prefix}${n}`)) n++;
  return `${prefix}${n}`;
}

export function createBuilderDraft(rules: GameRules, units: UnitCatalog): ScenarioBundleDraft {
  const blueKind = units.ids[0] ?? "";
  const redKind = blueKind;
  const authored: BuilderUnit[] = [
    { id: 0, side: "blue", kind: blueKind, position: [120, 240], yaw: 0 },
    { id: 1, side: "red", kind: redKind, position: [520, 240], yaw: Math.PI },
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
      events: [],
      scripts: [],
    },
    players: [
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
    players: draft.players.map((player) => ({
      ...player,
      units: player.units
        .filter((id) => id !== removedId)
        .map((id) => (id > removedId ? id - 1 : id)),
    })),
  };
}

export function parseBuilderDraft(text: string): ScenarioBundleDraft {
  const parsed = JSON.parse(text) as ScenarioBundleDraft & {
    scenario: ScenarioBundleDraft["scenario"] & {
      players?: Omit<BuilderPlayer, "units">[];
    };
  };
  if (
    !parsed ||
    parsed.format_version !== "scenario-bundle-2" ||
    !parsed.scenario ||
    !Array.isArray(parsed.scenario.units) ||
    (!Array.isArray(parsed.players) && !Array.isArray(parsed.scenario.players))
  )
    throw new Error("the uploaded package is not a scenario bundle draft");
  parsed.scenario.units = parsed.scenario.units.map((unit, index) => ({
    ...unit,
    id: Number.isInteger(unit.id) ? unit.id : index,
  }));
  parsed.players = (parsed.players ?? parsed.scenario.players ?? []).map((player) => ({
    ...player,
    units: parsed.scenario.units
      .filter((unit) => unit.owner === player.id)
      .map((unit) => unit.id)
      .sort((a, b) => a - b),
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
