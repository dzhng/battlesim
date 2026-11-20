import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire.ts";
import type { BattleMachines } from "./machines/index.ts";
import { MAP_ATTEMPTS } from "./serverData.ts";
import { createLobbyHttp } from "./http/index.ts";

/** `clientAddressHeader` names the header a trusted proxy in front of this
 *  lobby sets to the client's address, or is null where clients connect
 *  directly: the header is believed only because the caller says a proxy
 *  overwrites it. */
export function startLobby(
  machines: BattleMachines,
  location: URL,
  host: string,
  port: number,
  clientAddressHeader: string | null,
) {
  const build: BuildIdentity = JSON.parse(readFileSync(new URL("build.json", location), "utf8"));
  const server = createLobbyHttp(build, {
    machines,
    // Beside the bundle, which neither a rebuild nor a deploy clears.
    archive: fileURLToPath(new URL("battles/", location)),
    mapAttempts: MAP_ATTEMPTS,
    clock: {
      now: () => performance.now(),
      schedule: (run, delay) => {
        const timer = setTimeout(run, delay);
        return () => clearTimeout(timer);
      },
    },
    random: Math.random,
    clientAddressHeader,
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
