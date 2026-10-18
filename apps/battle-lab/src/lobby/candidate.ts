// The request a lobby previews and deploys, at a candidate map seed: the
// lobby model's request (`toRequest`) pinned to this build's generator,
// presets and template catalogue, which the model cannot read (it has no
// simulation module) and the battle route pins the same way.
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import type { PrepareBattleRequest } from "@web/battle/prepare/protocol";
import type { Wasm } from "@web/battle/sim/module";
import { generationRequest } from "@web/maps/source";
import { reroll, toRequest, type LobbyState } from "./model";

/** `state`'s request at each candidate seed the preview's admission draws. */
export function lobbyCandidate(wasm: Wasm, state: LobbyState) {
  return (seed: string): PrepareBattleRequest => {
    const request = toRequest(reroll(state, seed));
    return {
      ...request,
      map_source: {
        kind: "generated",
        request: generationRequest(
          wasm,
          request.map_source.request,
          { presets, templates },
          config.limits,
        ),
      },
    };
  };
}
