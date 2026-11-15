import { createBattleHost, type BattleHost } from "./battleHost/index.ts";
import { nodeSetupData } from "./serverData.ts";
import { battleRoute, createBattleHttp } from "./http/index.ts";
import { devBattle } from "./devBattle.ts";

async function main() {
  const data = await nodeSetupData(process.cwd());
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
    process.argv.includes("--dev") ? dev.join : undefined,
  );
  const port = Number(process.env.PORT ?? 8787);
  server.listen(port, "127.0.0.1", () => {
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
