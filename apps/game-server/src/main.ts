// Temporary bootstrap until rooms own battle creation. Credentials are supplied
// by the local operator, never printed or made part of the battle setup.
import { preset, setOccupant } from "../../../packages/skirmish-setup/src/lobby.ts";
import type { Seat } from "../../../packages/multiplayer/src/link.ts";
import { toBattleSetup } from "../../../packages/multiplayer/src/setup.ts";
import { createBattleHost } from "./battleHost/index.ts";
import { nodeSetupData } from "./serverData.ts";
import { battleRoute, createBattleHttp } from "./http/index.ts";

const blueId = process.env.BATTLE_BLUE_ID;
const redId = process.env.BATTLE_RED_ID;
if (
  !blueId ||
  !redId ||
  !/^[a-f0-9]{32}$/.test(blueId) ||
  !/^[a-f0-9]{32}$/.test(redId) ||
  blueId === redId
)
  throw new Error(
    "Set distinct BATTLE_BLUE_ID and BATTLE_RED_ID to 128-bit lowercase hex credentials",
  );
const root = process.cwd();
const data = await nodeSetupData(root);
const setup = toBattleSetup(
  setOccupant(preset("1v1"), "seat-2", { kind: "human", who: "dev-red" }),
  "skirmish",
);
const host = createBattleHost({ setup }, "thread", data);
await host.ready;
const { commands } = JSON.parse(data.rules) as { commands: { orders_per_s: number } };
const seats = new Map<string, Seat>([
  [blueId, { side: "blue", player: "blue-1" }],
  [redId, { side: "red", player: "red-1" }],
]);
const route = battleRoute(host, (browserId) => seats.get(browserId), commands.orders_per_s);
const server = createBattleHttp((id) => (id === "dev" ? route : undefined));
const port = Number(process.env.PORT ?? 8787);
server.listen(port, "127.0.0.1", () =>
  console.log(`Game server: http://127.0.0.1:${port}/battles/dev`),
);
function stop() {
  host.dispose();
  server.closeAllConnections();
  server.close();
}
process.once("SIGINT", stop);
process.once("SIGTERM", stop);
