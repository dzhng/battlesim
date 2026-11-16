import type { BattleHost } from "@apps/game-server/src/battleHost/index";
import type { BattleSetup } from "@packages/multiplayer/src/wire";
import skirmish from "@fixtures/skirmish.json";
import { pendingBattle } from "./pendingBattle";

/** A controllable authority edge; HTTP, placement, presence and the page stay real. */
export function frontlineHost(setup: BattleSetup): BattleHost {
  return {
    ...pendingBattle(),
    ready: Promise.resolve({
      mapIdentity: "frontline-map",
      tickHz: 30,
      layout: "test-layout",
      candidate: 0,
    }),
    referee: async () => ({
      tick: 0,
      scores: [0, 0],
      target: skirmish.victory_score,
      phase: "preparation",
      result: null,
      opening_credits: skirmish.credits_per_minute * skirmish.starting_minutes,
      seats: setup.seats.map((s) => ({
        ...s,
        living: 0,
        credits: skirmish.credits_per_minute * skirmish.starting_minutes,
      })),
    }),
    seatHuman: async () => {},
  };
}
