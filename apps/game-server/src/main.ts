import { readFileSync } from "node:fs";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire.ts";
import { childBattleMachines } from "./machines/child.ts";
import { MAP_ATTEMPTS } from "./serverData.ts";
import { createLobbyHttp } from "./http/index.ts";

async function main() {
  const build: BuildIdentity = JSON.parse(
    readFileSync(new URL("build.json", import.meta.url), "utf8"),
  );
  const server = createLobbyHttp(build, {
    machines: childBattleMachines(),
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
