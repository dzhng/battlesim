import type { BattleHost } from "../../../../apps/game-server/src/battleHost/index";

/** Hold preparation at the service seam; lobby tests never run a simulation. */
export function pendingBattle(): BattleHost {
  return {
    ready: new Promise(() => {}),
    attach() {
      throw new Error("battle still preparing");
    },
    setController() {},
    onEnd() {},
    dispose() {},
    replay() {
      return Promise.reject(new Error("battle still preparing"));
    },
  };
}
