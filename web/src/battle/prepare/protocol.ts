/** The wire contract of battle preparation: one request to a preparation
 *  worker, which generates the map with the simulation's own generator, lays
 *  the developer encounter on it and hands back the scenario a battle runs.
 *  The worker is closed after its one answer, so everything generation
 *  allocated goes with it; closing it early cancels the request. */

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

/** The developer encounter laid on a generated map (`developerEncounter`). */
export interface EncounterRecipe {
  /** Blue's force, a column on the country road that enters the map nearest
   *  the main settlement, heading for it: its tail `edge_inset_m` in from the
   *  edge, each unit `spacing_m` ahead of the next, the first row leading. */
  blue: { column: string[]; edge_inset_m: number; spacing_m: number };
  /** Red's force, one row per district of the main settlement in plan order:
   *  each stands `standoff_m` outside an entrance of the building nearest
   *  the district's anchor, and a `garrison` row is ordered into it. */
  red: { kind: string; garrison: boolean; standoff_m: number }[];
  /** The hold zone's radius, about the main settlement's centre. */
  zone_radius_m: number;
}

export interface PrepareRequest {
  type: "prepare";
  map: MapChoice;
  /** `fixtures/map-presets.json`, as text. */
  presets: string;
  /** The physical template descriptors the request pins, as text. */
  templates: string;
  limits: CompileLimits;
  /** The rules the battle runs under, as JSON text (the game's, with the
   *  resolved catalog). */
  rules: string;
  encounter: EncounterRecipe;
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

/** Mirrors `mapgen::Diagnostic`: why a request was refused. */
export interface MapDiagnostic {
  code: string;
  feature: string | null;
  location: string;
  message: string;
}

export type PrepareStage = "generating" | "placing";

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
  /** Where the encounter stands, for the camera: the head of blue's column
   *  and its heading, and the main settlement's centre. */
  anchors: { blue: [number, number]; blueYaw: number; town: [number, number] };
  /** Worker wall time per stage, milliseconds. */
  timings: Record<PrepareStage, number>;
  /** The preparation module's Wasm memory when it finished, bytes. */
  wasmBytes: number;
}

export type PrepareReply =
  | { type: "stage"; stage: PrepareStage }
  | { type: "prepared"; battle: PreparedBattle }
  /** The generator or compiler refused the request: never another seed. */
  | { type: "refused"; diagnostics: MapDiagnostic[] }
  | { type: "error"; message: string };
