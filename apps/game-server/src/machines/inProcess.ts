import type { BattleMachines } from "./index.ts";
import type { BattleHost, BattleSpec } from "../battleHost/index.ts";
import { createBattleServer } from "../http/control.ts";

/** Tests use the real control and player routes with their own authority clock. */
export function inProcessBattleMachines(options: {
  capacity: number;
  createBattle(spec: BattleSpec): BattleHost;
  ordersPerSecond: number;
  now?: () => number;
}): BattleMachines {
  let running = 0;
  return {
    capacity: options.capacity,
    async start(key) {
      if (running >= options.capacity) throw new Error("battle capacity exhausted");
      running++;
      let allocated = true;
      const release = () => {
        if (allocated) {
          allocated = false;
          running--;
        }
      };
      const battle = createBattleServer({ ...options, key });
      battle.server.once("close", release);
      await new Promise<void>((resolve, reject) => {
        battle.server.once("error", () => {
          release();
          battle.stop();
          reject(new Error("battle server startup failed"));
        });
        battle.server.listen(0, "127.0.0.1", resolve);
      });
      const address = battle.server.address();
      if (!address || typeof address === "string") throw new Error("battle server did not listen");
      const base = `http://127.0.0.1:${address.port}`;
      return {
        address: base,
        stop: () => {
          release();
          battle.stop();
        },
      };
    },
  };
}
