import { useEffect, useState } from "react";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import type { BattleSetup } from "@packages/multiplayer/src/wire";
import { fromBattleSetup } from "@packages/multiplayer/src/setup";
import { battleRequest } from "@packages/skirmish-setup/src/lobby";
import { prepareBattle } from "@web/battle/prepare/client";
import type { PreparedBattle, PrepareBattleRequest } from "@web/battle/prepare/protocol";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { useBuiltScenario, buildFailed } from "../useBuiltScenario";
import { setupData } from "../skirmishData";

/** Plain admitted data can cross navigation; the worker and its connection cannot. */
export type OnlineMap = { request: PrepareBattleRequest; battle: PreparedBattle };
type Preparation =
  | { status: "preparing"; stage: string }
  | { status: "failed"; error: string }
  | { status: "ready"; prepared: OnlineMap };

/** Rooms and Frontline build the same request, worker and identity check. The
 * briefing owns its progress, so map work does not raise the menu shell's cover. */
export function useOnlineMap(setup: BattleSetup, mapIdentity: string): Preparation {
  const { rules } = useSessionCatalog();
  const request = useBuiltScenario(
    setup,
    (wasm, value) => battleRequest(fromBattleSetup(value), setupData(wasm)),
    false,
  );
  const key = JSON.stringify([request, mapIdentity]);
  const [state, setState] = useState<{ key: string; value: Preparation } | null>(null);
  useEffect(() => {
    if (!request || buildFailed(request)) return;
    let live = true;
    const publish = (value: Preparation) => {
      if (live) setState({ key, value });
    };
    const preparation = prepareBattle(
      {
        type: "prepare",
        request,
        documents: { rules: JSON.stringify(rules), presets, templates },
      },
      (stage) => publish({ status: "preparing", stage }),
    );
    void preparation.battle
      .then((battle) => {
        if (
          battle.report.identity.kind !== "generated" ||
          battle.report.identity.generation.map_hash !== mapIdentity
        )
          return publish({
            status: "failed",
            error: "The local map differs from the server's map.",
          });
        publish({
          status: "ready",
          prepared: {
            request,
            battle: { scenario: battle.scenario, report: battle.report },
          },
        });
      })
      .catch(() =>
        publish({
          status: "failed",
          error:
            "The battlefield could not be prepared or did not match the server. Return to Multiplayer and try again.",
        }),
      )
      .finally(() => preparation.cancel());
    return () => {
      live = false;
      preparation.cancel();
    };
    // The request's serialized identity, rather than an incidental object, owns this work.
  }, [key, request, mapIdentity, rules]);
  if (buildFailed(request))
    return {
      status: "failed",
      error: "The battlefield could not be prepared. Return to Multiplayer and try again.",
    };
  return state?.key === key ? state.value : { status: "preparing", stage: "map" };
}
