/** The contract of battle preparation: one request to a preparation worker,
 *  which resolves the map through the one map owner (`@web/maps/source`), has
 *  the encounter laid on it and hands back the scenario a battle runs. The
 *  worker is closed after its one answer, so everything preparation
 *  allocated goes with it; closing it early cancels the request. */
import type { MapIdentity } from "../../maps/resolve.ts";
import type { MapDiagnostic, MapSource } from "../../maps/source.ts";

/** Mirrors `contract::preparation::PrepareBattleRequest`, which checks it.
 *  Everything that decides the battle: a saved replay stores this. */
export interface PrepareBattleRequest {
  map_source: MapSource;
  /** Which encounter. On a generated map, a recipe of
   *  `fixtures/encounters.json`, placed by the simulation's planner; on a
   *  catalogue map, its saved encounter of that name. */
  recipe_id: string;
  /** The planner's seed, apart from the map's: canonical u64 decimal text.
   *  A saved encounter does not read it. */
  encounter_seed: string;
  /** The battle's own random input: a whole number a JS number holds
   *  exactly, as the battle's constructor takes it. */
  battle_seed: number;
}

/** The build's documents preparation reads, as text: none of them is part
 *  of a request's identity beyond what the request pins. */
export interface PrepareDocuments {
  /** The rules the battle runs under (the game's, with the resolved
   *  catalog). */
  rules: string;
  /** `fixtures/map-presets.json`. */
  presets: string;
  /** The physical template descriptors a generated map is built from. */
  templates: string;
  /** `fixtures/encounters.json`: the recipes, none holding a coordinate. */
  recipes: string;
}

/** What the page posts to the worker. */
export interface PrepareMessage {
  type: "prepare";
  request: PrepareBattleRequest;
  documents: PrepareDocuments;
}

/** Why a request was refused: the request check's, the map owner's or the
 *  encounter planner's diagnostics, which share the shape. */
export type PrepareDiagnostic = MapDiagnostic;

/** What preparation is doing: resolving the map, then laying the encounter. */
export type PrepareStage = "map" | "encounter";
/** Where a refusal came from: the request's own check, or a stage. */
export type RefusalStage = "request" | PrepareStage;

type Side = "blue" | "red";

/** Mirrors `contract::encounter::Placement`: where the planner put the
 *  encounter and why, for the camera, overlays and diagnostics. The battle
 *  reads none of it. */
export interface EncounterPlacement {
  objective: { settlement: string; center: [number, number]; radius_m: number };
  /** The attacker's column, then the defender's if it has one. */
  deployments: {
    side: Side;
    edge: "top" | "bottom";
    /** Unit ids, leader first. */
    units: number[];
    head: [number, number];
    yaw: number;
    /** How far the column was moved up its road to make the start fair. */
    advance_m: number;
    /** The pace unit's drive from `head` to the objective. */
    route_s: number;
    route_m: number;
    route: [number, number][];
  }[];
  /** A squad and the building (its owner prop id) it is ordered into. */
  garrisons: {
    unit: number;
    building: number;
    district: string;
    soldiers: number;
    seats: number;
  }[];
  overwatch: { unit: number; at: [number, number]; yaw: number; approach: number | null }[];
  /** Candidates tried, over every search. */
  attempts: number;
}

/** What preparation made: the scenario JSON a battle authority runs
 *  (`ScenarioDefinition`), and what it is. */
export interface PreparedBattle {
  scenario: string;
  report: PreparationReport;
}

export interface PreparationReport {
  /** The request, as the simulation's check wrote it back. */
  request: PrepareBattleRequest;
  /** What the resolved map is. */
  identity: MapIdentity;
  /** The playable area, metres. */
  size: [number, number];
  counts: { buildings: number; parts: number; surfaces: number; forests: number };
  /** A planned encounter's inputs (the recipe's content hash and the
   *  encounter seed) and where the planner put it; null for a saved one. */
  planned: { recipe_hash: string; encounter_seed: string; placement: EncounterPlacement } | null;
  /** The encounter's completion rule, if it has one: the zone to hold and
   *  for how long. */
  objective: { center: [number, number]; radius_m: number; hold_s: number } | null;
  /** Where blue starts, for the camera: its first unit (the head of its
   *  column) and its heading. */
  start: { at: [number, number]; yaw: number };
  /** Worker wall time per stage, milliseconds. */
  timings: Record<PrepareStage, number>;
  /** The preparation module's Wasm memory when it finished, bytes. */
  wasmBytes: number;
}

export type PrepareReply =
  | { type: "stage"; stage: PrepareStage }
  | { type: "prepared"; battle: PreparedBattle }
  /** The request's check, the map owner or the planner refused (`stage`
   *  says which): never another seed, never another map. */
  | { type: "refused"; stage: RefusalStage; diagnostics: PrepareDiagnostic[] }
  | { type: "error"; message: string };
