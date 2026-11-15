import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire.ts";
import { createBattleHost, type BattleHost } from "./battleHost/index.ts";
import { nodeSetupData } from "./serverData.ts";
import { battleRoute, createBattleHttp } from "./http/index.ts";
import { devBattle } from "./devBattle.ts";

async function main() {
  const root = fileURLToPath(new URL(".", import.meta.url));
  const build: BuildIdentity = JSON.parse(
    readFileSync(new URL("build.json", import.meta.url), "utf8"),
  );
  const data = await nodeSetupData(root);
  const { commands } = JSON.parse(data.rules) as { commands: { orders_per_s: number } };
  let host: BattleHost | undefined;
  let route: ReturnType<typeof battleRoute> | undefined;
  const dev = devBattle(async (setup) => {
    host = createBattleHost({ setup }, "thread", data);
    const ready = await host.ready;
    route = battleRoute(host, dev.seatOf, commands.orders_per_s);
    return ready;
  });
  const server = createBattleHttp(
    (id) => (id === "dev" ? route : undefined),
    build,
    process.argv.includes("--dev") ? dev.join : undefined,
  );
  const port = Number(process.env.PORT ?? 8787);
  server.listen(port, process.env.HOST ?? "127.0.0.1", () => {
    const address = server.address();
    if (address && typeof address !== "string")
      console.log(`Game server: http://127.0.0.1:${address.port}`);
  });
  function stop() {
    host?.dispose();
    server.closeAllConnections();
    server.close();
  }
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
}

await main();
