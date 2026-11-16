import { inProcessBattleMachines } from "../../../../apps/game-server/src/machines/inProcess";
import type { BattleHost } from "@apps/game-server/src/battleHost/index";
import type { BattleSetup } from "@packages/multiplayer/src/wire";
import type { PresenceClock } from "@apps/game-server/src/presence";
import { pendingBattle } from "./pendingBattle";
import { createLobbyHttp } from "@apps/game-server/src/http/index";
import type { BuildIdentity } from "@packages/multiplayer/src/wire";

export const build: BuildIdentity = {
  data: "room-screen-test",
  wire: "room-screen-test",
  sim: "room-screen-sim",
};

/** Real room HTTP service; no battle or external service is started. */
export async function roomHttp(
  capacity = 8,
  createBattle: (setup: BattleSetup) => BattleHost = pendingBattle,
  clock?: PresenceClock,
  clientAddressHeader: string | null = null,
) {
  const server = createLobbyHttp(build, {
    machines: inProcessBattleMachines({
      capacity,
      createBattle: (spec) => createBattle(spec.candidates[0]),
      ordersPerSecond: 100,
    }),
    mapAttempts: 1,
    clock: clock ?? {
      now: () => Date.now(),
      schedule: (run, delay) => {
        const timer = setTimeout(run, delay);
        return () => clearTimeout(timer);
      },
    },
    random: Math.random,
    clientAddressHeader,
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  return {
    address,
    cut: () => server.closeAllConnections(),
    async post(path: string, who: string, value: unknown, headers: Record<string, string> = {}) {
      return (
        await fetch(`${address}${path}`, {
          method: "POST",
          headers: { "X-Player": who, "Content-Type": "application/json", ...headers },
          body: JSON.stringify(value),
        })
      ).json();
    },
    async close() {
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    },
  };
}
