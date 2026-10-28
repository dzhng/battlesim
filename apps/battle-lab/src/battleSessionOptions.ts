// What a battle session (`useBattleSession`) is handed, and the rules its
// views read.
import type { PreparedConnectionSession } from "@web/battle/prepare/client";
import type { SoldierBody } from "@packages/battle-renderer/src/picking";
import type { PanelRules } from "@web/battle/present/panelRows";
import type { RulerRules } from "@web/battle/present/rangeRuler";
import type { ObservationView } from "@web/battle/sim/observationView";
import type { SideName } from "@web/battle/sim/protocol";
import type { ScriptedSim } from "./useSimSession";
import type { AirLayers } from "./unitGhosts";

export interface BattleSessionOptions {
  /** The scenario JSON the authority runs. */
  scenario: string;
  seed: number;
  prepared?: PreparedConnectionSession;
  /** Called for every decoded frame, before its credit returns. */
  onDecoded?: (o: ObservationView, digest: string) => void;
  /** A recorded battle to replay: input is off. */
  replay?: string;
  /** Blue is played by a script (the benchmark): input is off. */
  scripted?: ScriptedSim;
  /** Whose units and identified enemies are drawn (a lab's diagnostic side switch). */
  side?: SideName;
  /** Player issuing orders in a player-owned scenario; null when the session
   *  seats no human (`commandingPlayer.ts`). */
  player?: string | null;
  /** Play the battle's sound (heard from the last frame's camera). */
  sound?: boolean;
  /** Menus and loading covers suspend command input without losing selection. */
  inputEnabled?: boolean;
  /** Own units hidden by the world are drawn through it (the player's x-ray);
   *  off, the picture shows only what the camera sees. */
  xray?: boolean;
  /** Uploaded resolved sandbox bundles may narrow art to their fielded map. */
  scenarioArt?: boolean;
}

/** The rule values the scenario runs under (only what views read). Its
 *  units are its session catalog's: the scenario's rules carry that
 *  catalog's documents. */
export interface ScenarioRules extends PanelRules, RulerRules {
  tick_hz: number;
  weapons: PanelRules["weapons"] & RulerRules["weapons"];
  physics: SoldierBody & RulerRules["physics"];
  service: { radius_m: number };
  /** The sensor numbers fog of war shares with the simulation (the
   *  presentation reads them: `presentation::fog::sight::Sensors`). */
  sensors: object;
  /** Each air layer's cruise height. */
  air: AirLayers;
}
