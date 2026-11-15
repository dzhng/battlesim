import { pendingBattle } from "./pendingBattle";
import { createBattleHttp } from "@apps/game-server/src/http/index";
import type { BuildIdentity } from "@packages/multiplayer/src/wire";

export const build: BuildIdentity = { commit: "room-screen-test", sim: "room-screen-sim" };

/** Real room HTTP service; no battle or external service is started. */
export async function roomHttp(capacity = 8, createBattle = pendingBattle) {
  const server = createBattleHttp(() => undefined, build, {
    createBattle,
    mapAttempts: 1,
    ordersPerSecond: 100,
    capacity,
    clock: {
      now: () => Date.now(),
      schedule: (run, delay) => {
        const timer = setTimeout(run, delay);
        return () => clearTimeout(timer);
      },
    },
    random: Math.random,
    clientAddressHeader: null,
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  return {
    address,
    cut: () => server.closeAllConnections(),
    async post(path: string, who: string, value: unknown) {
      return (
        await fetch(`${address}${path}`, {
          method: "POST",
          headers: { "X-Player": who, "Content-Type": "application/json" },
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
