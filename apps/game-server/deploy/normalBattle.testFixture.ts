// A host-end fixture at the authority boundary; HTTP and the process are real.
import { fileURLToPath } from "node:url";
import { createBattleHost } from "../src/battleHost/index.ts";
import { createBattleServer } from "../src/http/control.ts";
import { nodeSetupData } from "../src/serverData.ts";

const data = await nodeSetupData(fileURLToPath(new URL(".", import.meta.url)));
const battle = createBattleServer({
  key: process.env.BATTLE_KEY!,
  ordersPerSecond: JSON.parse(data.rules).commands.orders_per_s,
  createBattle(spec) {
    const host = createBattleHost(spec, "thread", data);
    return {
      ...host,
      onEnd(handle) {
        host.onEnd(handle);
        process.once("SIGUSR2", () => handle({ kind: "finished" }));
      },
    };
  },
});
process.once("SIGTERM", battle.stop);
battle.server.listen(Number(process.env.PORT), process.env.HOST);
