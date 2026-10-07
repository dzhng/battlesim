/** The contract of battle preparation: one request to a preparation worker,
 *  which resolves the map through the one map owner (`@web/maps/source`),
 *  fields both factions on it and hands back the scenario a battle runs. The
 *  worker keeps the world it built and then runs that battle on it, so its
 *  later requests are the simulation's; closing it cancels a pending request
 *  and frees everything preparation allocated. */
import type { SimRequest } from "../sim/protocol";
import type { MapIdentity } from "../../maps/resolve.ts";
import type { MapDiagnostic, MapSource } from "../../maps/source.ts";
import type { MapExtents } from "@packages/battle-renderer/src/worldMesh";

/** Mirrors `contract::preparation::PrepareBattleRequest`, which checks it.
 *  Everything preparation needs to create a new battle; nothing is defaulted. */
export interface PrepareBattleRequest {
  map_source: MapSource;
  /** The player's faction, then the enemy's. */
  factions: [
    import("@packages/scene-assets/src/units").Faction,
    import("@packages/scene-assets/src/units").Faction,
  ];
  /** The battle's own random input: a whole number a JS number holds
   *  exactly, as the battle's constructor takes it. */
  battle_seed: number;
}

/** The build's documents preparation reads, as text: none of them is part
 *  of a request's identity beyond what the request pins. */
export interface PrepareDocuments {
  /** The rules the battle runs under (the game's, with the resolved
   *  catalog, which a generated map's street furniture is placed from). */
  rules: string;
  /** `fixtures/map-presets.json`. */
  presets: string;
  /** The physical template descriptors a generated map is built from. */
  templates: string;
}

/** What the page posts to the worker. */
export interface PrepareMessage {
  type: "prepare";
  request: PrepareBattleRequest;
  documents: PrepareDocuments;
  /** Lab-only synthetic contact workload; normal battles omit it. */
  stress?: StressPreparation;
}

/** Replay preparation reads only the saved scenario, never today's fixtures. */
export interface PrepareReplayMessage {
  type: "prepare-replay";
  battle: PreparedBattle;
}
export type PreparationMessage = PrepareMessage | PrepareReplayMessage;

export interface StressPreparation {
  kind: "city-arena-2";
  late: boolean;
}

/** Why a request was refused: the request check's, the map owner's or the
 *  skirmish sites' diagnostics, which share the shape. */
export type PrepareDiagnostic = MapDiagnostic;

/** What preparation is doing: resolving the map, then fielding the forces
 *  on it (`encounter`). */
export type PrepareStage = "map" | "encounter";
/** Where a refusal came from: the request's own check, or a stage. */
export type RefusalStage = "request" | PrepareStage;

/** What preparation made: the scenario JSON a battle authority runs
 *  (`ScenarioDefinition`), and what it is. */
export interface PreparedBattle {
  scenario: string;
  report: PreparationReport;
}

export interface PreparationReport {
  stress?: StressPreparation & { livingUnits: Record<"blue" | "red", number> };
  /** The request, as the simulation's check wrote it back. */
  request: PrepareBattleRequest;
  /** What the resolved map is. */
  identity: MapIdentity;
  /** The playable area, metres. */
  size: [number, number];
  /** Map-owned bounds, excluding the existing display environment beyond them. */
  extents: MapExtents;
  /** `props` are the map's own bodies that are no building's part: street
   *  furniture on a generated map. */
  counts: { buildings: number; parts: number; props: number; surfaces: number; forests: number };
  /** Where blue starts, for the camera: its admitted base (a stress
   *  scene's first unit) and its heading. */
  start: { at: [number, number]; yaw: number };
  /** Worker wall time per stage, milliseconds. */
  timings: Record<PrepareStage, number>;
  /** The preparation module's Wasm memory when it finished, bytes. */
  wasmBytes: number;
  /** Of `timings.encounter`: building the world the sites and the battle share. */
  worldBuildMs: number;
}

export type PrepareWorkerRequest = PreparationMessage | SimRequest;

export type PrepareReply =
  | { type: "stage"; stage: PrepareStage }
  | { type: "prepared"; battle: PreparedBattle }
  /** The request's check, the map owner or the skirmish sites refused (`stage`
   *  says which): never another seed, never another map. */
  | { type: "refused"; stage: RefusalStage; diagnostics: PrepareDiagnostic[] }
  | { type: "error"; message: string };
