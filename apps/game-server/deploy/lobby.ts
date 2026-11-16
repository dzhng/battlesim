import { fileURLToPath } from "node:url";
import { startLobby } from "../src/lobby.ts";
import { sandboxBattleMachines } from "./sandboxes.ts";
import settings from "./settings.json";

const apiUrl = process.env.BENTO_API_URL;
const appKey = process.env.BENTO_APP_API_KEY;
if (!apiUrl || !appKey || !process.env.PORT) throw new Error("Lobby hosting environment missing");
try {
  const machines = await sandboxBattleMachines({
    apiUrl,
    appKey,
    capacity: settings.capacity,
    ttlSeconds: settings.ttlSeconds,
    archive: fileURLToPath(new URL("battle.tar.gz", import.meta.url)),
  });
  startLobby(
    machines,
    new URL(import.meta.url),
    "0.0.0.0",
    Number(process.env.PORT),
    settings.clientAddressHeader,
  );
} catch {
  console.error("Lobby machine provider could not start");
  process.exitCode = 1;
}
