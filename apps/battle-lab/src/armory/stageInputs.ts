// What a `ShowcaseStage` is built from on the lab's pages: the game's look,
// the session's fielding and sound setup, and the simulation's squad
// scatter. The Armory and the showcase lab share them, so both show a unit
// as the battle would.
import { useEffect, useMemo, useState } from "react";
import type { Showground } from "@wasm/game_wasm.js";
import { gameSounds } from "@packages/battle-audio/src/shippedSounds";
import type { SessionCatalog } from "@web/battle/catalog/compose";
import type { StageFielding, StageLook } from "@web/battle/render/showcaseStage";
import { loadWasm } from "@web/battle/sim/module";
import { audioMix } from "@web/audioMix";
import { presentationSettings } from "@web/presentationSettings";
import { gameLight } from "../gameLight";
import { gameFogStyle } from "../gameFog";
import { gameOverlayGlow, gamePaint, gameXrayMinHiddenFragmentFraction } from "../gameOverlay";
import { gameModelDetail } from "../gameModels";

/** The battle seed a squad on show stands scattered as. */
export const SQUAD_SEED = 7;

/** The game's light, fog, paint, model detail and overlay glow. */
export const STAGE_LOOK: StageLook = {
  settings: {
    light: gameLight,
    fog: gameFogStyle,
    paint: gamePaint,
    xrayMinHiddenFraction: gameXrayMinHiddenFragmentFraction,
  },
  detail: JSON.parse(gameModelDetail),
  overlayGlow: gameOverlayGlow,
};

/** What `catalog`'s units are fielded and heard from. The sound setup takes
 *  the battle's weapon rules (`rules.weapons`), never the published rows,
 *  whose `derived` the sound admission refuses. */
export function useStageFielding(catalog: SessionCatalog): StageFielding {
  return useMemo(
    () => ({
      rules: JSON.stringify(catalog.rules),
      catalog: catalog.units.view,
      sound: {
        tickHz: catalog.rules.tick_hz,
        presentation: audioMix,
        catalog: gameSounds,
        weapons: catalog.rules.weapons,
        unitCatalog: catalog.units.view,
        looks: presentationSettings,
      },
    }),
    [catalog],
  );
}

/** The simulation's squad scatter under `rules` (JSON), freed with the
 *  page; null until the module loads. A load that fails goes to `failed`. */
export function useShowground(rules: string, failed: (error: unknown) => void): Showground | null {
  const [showground, setShowground] = useState<Showground | null>(null);
  useEffect(() => {
    let live = true;
    let made: Showground | null = null;
    void loadWasm().then((wasm) => {
      if (!live) return;
      made = new wasm.Showground(rules);
      setShowground(made);
    }, failed);
    return () => {
      live = false;
      made?.free();
      setShowground(null);
    };
  }, [rules, failed]);
  return showground;
}
