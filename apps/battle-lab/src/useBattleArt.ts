// One battle owns the wanted kinds, their complete art and canonical fielding.
import { useMemo } from "react";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import type { ObservationView } from "@web/battle/sim/observationView";
import type { Player } from "@web/battle/prepare/protocol";
import type { SideName } from "@web/battle/sim/protocol";
import type { ScenarioRules } from "./battleSessionOptions";
import type { StaticWorld, MapBuildings } from "./useStaticWorld";
import { gameRuntime, useBattleAppearances, useBattleKinds } from "./gameAppearances";
import { useBattleFielding } from "./useBattleFielding";
import type { fieldedBattle } from "./presentedFrame";
import { gameSun } from "./gameLight";
import { gameXray } from "./gameOverlay";
import { gameHud } from "@web/battle/present/hudTheme";

const PURCHASE_BLOCKED = [...gameHud.bad, gameXray.selected[3]] as const;

export function useBattleArt(input: {
  scenario: string;
  seed: number;
  side: SideName;
  presentation: ScenePresentation | null;
  world: StaticWorld | null;
  buildings: MapBuildings | null;
  units: UnitCatalog;
  fielded: readonly string[];
  rules: ScenarioRules;
  rulesText: string;
  skirmish: boolean;
  players: readonly Player[];
  /** The factions whose looks this battle's art is loaded in; none outside a skirmish. */
  factions: ReadonlySet<string> | null;
  viewer: boolean;
  observation: ObservationView | null;
  purchasing: string | null;
  scenery: ReadonlySet<string> | null | undefined;
}) {
  const {
    scenario,
    seed,
    side,
    presentation,
    world,
    buildings,
    units,
    fielded,
    rules,
    rulesText,
    skirmish,
    players,
    factions,
    viewer,
    observation,
    purchasing,
    scenery,
  } = input;
  const blueFaction = players.find((player) => player.side === "blue")?.faction ?? null;
  const redFaction = players.find((player) => player.side === "red")?.faction ?? null;
  const faction = side === "blue" ? blueFaction : redFaction;
  const kinds = useBattleKinds({
    scope: presentation,
    side,
    units,
    fielded,
    faction: skirmish && viewer ? faction : null,
    observation,
    purchasing,
  });
  // The kits this map's buildings and stand-in boxes draw
  // from and its region's looks, fetched once the map is known: the loading
  // cover stays up for them.
  const completeArt = useBattleAppearances({
    world,
    buildings,
    units,
    kinds,
    skirmish,
    factions,
    scenery,
  });
  // A battle/observer/map owns one sequence of complete topology/art generations.
  const fielding = useMemo<Parameters<typeof fieldedBattle>[0]>(
    () => ({
      rules: rulesText,
      catalog: units.view,
      kinds,
      factions: skirmish ? [blueFaction!, redFaction!] : undefined,
      runtime: gameRuntime,
      side,
      tickHz: rules.tick_hz,
      sun: gameSun(),
      soldier: rules.physics,
      xray: gameXray,
      blocked: PURCHASE_BLOCKED,
    }),
    [rulesText, units, kinds, skirmish, blueFaction, redFaction, rules, side],
  );
  const scope = useMemo(
    () => ({ scenario, seed, side, presentation, world }),
    [scenario, seed, side, presentation, world],
  );
  return useBattleFielding(scope, fielding, completeArt);
}
