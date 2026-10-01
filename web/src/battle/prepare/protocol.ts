/** The wire contract of battle preparation: one request to a preparation
 *  worker, which generates the map with the simulation's own generator, has
 *  the simulation's planner place the encounter on it and hands back the
 *  scenario a battle runs. The worker is closed after its one answer, so
 *  everything generation allocated goes with it; closing it early cancels
 *  the request. */

export type MapType = "open" | "mixed" | "metro";
export type MapSize = "small" | "medium" | "large";
export const MAP_TYPES: readonly MapType[] = ["open", "mixed", "metro"];
export const MAP_SIZES: readonly MapSize[] = ["small", "medium", "large"];

/** Which map: the two composition controls and the seed, a canonical u64
 *  decimal string (a JS number cannot hold every seed). */
export interface MapChoice {
  type: MapType;
  size: MapSize;
  seed: string;
}

/** Mirrors `mapgen::CompileLimits`: the compiler's admission for the plan. */
export interface CompileLimits {
  max_authored_parts: number;
  max_bay_positions: number;
  max_ground_points: number;
}

export interface PrepareRequest {
  type: "prepare";
  map: MapChoice;
  /** `fixtures/map-presets.json`, as text. */
  presets: string;
  /** The physical template descriptors the request pins, as text. */
  templates: string;
  limits: CompileLimits;
  /** The rules the battle runs under, as JSON text (the village's, with the
   *  resolved catalog). */
  rules: string;
  /** One recipe of `fixtures/encounters.json` (`contract::encounter::
   *  EncounterRecipe`), as JSON text: who attacks from which edge, each
   *  side's roster and posts. It holds no coordinates. */
  recipe: string;
  /** The encounter seed, apart from the map's: a canonical u64 decimal
   *  string. It chooses among the buildings a garrison may take. */
  encounterSeed: string;
}

/** Mirrors `contract::identity::GenerationIdentity`. */
export interface GenerationIdentity {
  generator_version: string;
  preset_revision: string;
  seed: string;
  config_hash: string;
  template_catalog_hash: string;
  map_hash: string;
}

/** Why a request was refused: mirrors `mapgen::Diagnostic` and
 *  `contract::encounter::EncounterDiagnostic`, which share the shape. */
export interface PrepareDiagnostic {
  code: string;
  feature: string | null;
  location: string;
  message: string;
}

export type PrepareStage = "generating" | "placing";

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
  map: MapChoice;
  identity: GenerationIdentity;
  /** The playable area, metres. */
  size: [number, number];
  counts: { buildings: number; parts: number; surfaces: number; forests: number };
  /** The planned encounter's inputs: the recipe's content hash and the
   *  encounter seed. */
  encounter: { recipe_hash: string; encounter_seed: string };
  placement: EncounterPlacement;
  /** Where the encounter stands, for the camera: the head of blue's column
   *  and its heading, and the objective's centre. */
  anchors: { blue: [number, number]; blueYaw: number; town: [number, number] };
  /** Worker wall time per stage, milliseconds. */
  timings: Record<PrepareStage, number>;
  /** The preparation module's Wasm memory when it finished, bytes. */
  wasmBytes: number;
}

export type PrepareReply =
  | { type: "stage"; stage: PrepareStage }
  | { type: "prepared"; battle: PreparedBattle }
  /** The generator or compiler refused the map, or the planner the
   *  encounter (`stage` says which): never another seed. */
  | { type: "refused"; stage: PrepareStage; diagnostics: PrepareDiagnostic[] }
  | { type: "error"; message: string };
