import type { BattleHost } from "../../../../apps/game-server/src/battleHost/index";

/** Hold preparation at the service seam; lobby tests never run a simulation. */
export function pendingBattle(): BattleHost {
  return {
    ready: new Promise(() => {}),
    attach() {
      throw new Error("battle still preparing");
    },
    setController() {
      return Promise.resolve();
    },
    referee() {
      return Promise.reject(new Error("battle still preparing"));
    },
    seatHuman() {
      return Promise.reject(new Error("battle still preparing"));
    },
    onOrder() {},
    endSeat() {
      return Promise.resolve();
    },
    onEnd() {},
    dispose() {},
    replay() {
      return Promise.reject(new Error("battle still preparing"));
    },
  };
}
