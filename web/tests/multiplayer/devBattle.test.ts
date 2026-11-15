// @vitest-environment node
import { expect, test } from "vitest";
import { devBattle } from "../../../apps/game-server/src/devBattle";
import { createBattleHttp } from "../../../apps/game-server/src/http/index";

test("one dev battle seats Blue then Red, refuses a third, and returns each browser's ticket", async () => {
  const battle = devBattle(async () => ({ mapIdentity: "generated-map" }));
  const [blue, red] = await Promise.all([battle.join("first"), battle.join("second")]);
  expect(blue).toMatchObject({ side: "blue", mapIdentity: "generated-map" });
  expect(red).toMatchObject({ side: "red", mapIdentity: "generated-map" });
  if ("refused" in blue || "refused" in red) throw new Error("seats refused");
  expect(red.battle).toBe(blue.battle);
  expect(red.player).not.toBe(blue.player);
  expect(blue.setup.seats.every((seat) => seat.controller === "human")).toBe(true);
  expect(await battle.join("third")).toEqual({ refused: "full" });
  expect(await battle.join("first")).toEqual(blue);
  expect(await battle.join("second")).toEqual(red);
  expect(battle.seatOf("first")).toEqual({ side: blue.side, player: blue.player });
  expect(battle.seatOf("second")).toEqual({ side: red.side, player: red.player });
  expect(battle.seatOf("third")).toBeUndefined();
});

test("HTTP dev entry is opt-in, checks the credential and returns tickets without credentials", async () => {
  for (const enabled of [false, true]) {
    const battle = devBattle(async () => ({ mapIdentity: "generated-map" }));
    const server = createBattleHttp(
      () => undefined,
      { commit: "test", sim: "test" },
      enabled ? battle.join : undefined,
    );
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    if (!address || typeof address === "string") throw new Error("no TCP address");
    const url = `http://127.0.0.1:${address.port}/dev/battle`;
    try {
      const invalid = await fetch(url, { method: "POST", headers: { "X-Player": "short" } });
      expect(invalid.status).toBe(enabled ? 403 : 404);
      const credential = "0123456789abcdef0123456789abcdef";
      const response = await fetch(url, { method: "POST", headers: { "X-Player": credential } });
      expect(response.status).toBe(enabled ? 200 : 404);
      expect(response.headers.get("Access-Control-Allow-Origin")).toBe("*");
      expect(response.headers.get("Cache-Control")).toContain("no-store");
      const result = await response.json();
      expect(JSON.stringify(result)).not.toContain(credential);
      if (enabled) expect(result).toMatchObject({ side: "blue" });
    } finally {
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  }
});
