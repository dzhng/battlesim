import { readFileSync } from "node:fs";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire.ts";
import type { BattleMachines } from "./machines/index.ts";
import { MAP_ATTEMPTS } from "./serverData.ts";
import { createLobbyHttp } from "./http/index.ts";

export function startLobby(machines: BattleMachines, location: URL, host: string, port: number) {
  const build: BuildIdentity = JSON.parse(readFileSync(new URL("build.json", location), "utf8"));
  const server = createLobbyHttp(build, {
    machines,
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
  server.listen(port, host, () => {
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
  return server;
}
