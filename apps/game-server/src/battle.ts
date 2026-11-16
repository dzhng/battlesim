import { fileURLToPath } from "node:url";
import { createBattleHost } from "./battleHost/index.ts";
import { createBattleServer } from "./http/control.ts";
import { nodeSetupData } from "./serverData.ts";

const key = process.env.BATTLE_KEY;
if (!key || !/^[a-f0-9]{32}$/.test(key)) throw new Error("battle credential missing");
delete process.env.BATTLE_KEY;
const data = await nodeSetupData(fileURLToPath(new URL(".", import.meta.url)));
const { commands } = JSON.parse(data.rules) as { commands: { orders_per_s: number } };
const battle = createBattleServer({
  key,
  createBattle: (spec) => createBattleHost(spec, "thread", data),
  ordersPerSecond: commands.orders_per_s,
});
// Losing the parent is immediate locally. The control lease also covers a
// lobby on another machine, or a connection that no longer makes progress.
process.once("disconnect", battle.stop);
process.once("SIGTERM", battle.stop);
process.once("SIGINT", battle.stop);
battle.server.listen(0, "127.0.0.1", () => {
  const address = battle.server.address();
  if (address && typeof address !== "string")
    process.send?.({ address: `http://127.0.0.1:${address.port}` });
});
battle.server.once("close", () => {
  if (process.connected) process.disconnect?.();
});
