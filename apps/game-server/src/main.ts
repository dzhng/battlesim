import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire.ts";
import { createBattleHost } from "./battleHost/index.ts";
import { MAP_ATTEMPTS, nodeSetupData } from "./serverData.ts";
import { createBattleHttp } from "./http/index.ts";
import { battleCapacity } from "./online.ts";

async function main() {
  const root = fileURLToPath(new URL(".", import.meta.url));
  const build: BuildIdentity = JSON.parse(
    readFileSync(new URL("build.json", import.meta.url), "utf8"),
  );
  const data = await nodeSetupData(root);
  const { commands } = JSON.parse(data.rules) as { commands: { orders_per_s: number } };
  const server = createBattleHttp(() => undefined, build, {
    capacity: battleCapacity(),
    createBattle(setup) {
      const host = createBattleHost({ setup }, "thread", data);
      // A crashed battle is lost; the log is the only record of why.
      host.onEnd((end) => {
        if (end.kind === "crashed") console.error(`A battle crashed: ${end.message}`);
      });
      return host;
    },
    ordersPerSecond: commands.orders_per_s,
    mapAttempts: MAP_ATTEMPTS,
    clock: {
      now: () => performance.now(),
      schedule: (run, delay) => {
        const timer = setTimeout(run, delay);
        return () => clearTimeout(timer);
      },
    },
    random: Math.random,
    clientAddressHeader: process.env.CLIENT_ADDRESS_HEADER ?? null,
  });
  const port = Number(process.env.PORT ?? 8787);
  server.listen(port, process.env.HOST ?? "127.0.0.1", () => {
    const address = server.address();
    if (address && typeof address !== "string")
      console.log(`Game server: http://127.0.0.1:${address.port}`);
  });
  function stop() {
    server.closeAllConnections();
    server.close();
  }
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
}

await main();
