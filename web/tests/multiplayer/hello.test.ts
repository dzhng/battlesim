// @vitest-environment node
import { timerClock } from "./timerClock";
import { afterEach, expect, test, vi } from "vitest";
import { createBattleHttp } from "../../../apps/game-server/src/http/index";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire";

const build: BuildIdentity = { commit: "a".repeat(40), sim: "b".repeat(64) };
const cleanup: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const close of cleanup.splice(0)) await close();
  vi.restoreAllMocks();
});
async function serving() {
  const server = createBattleHttp(() => undefined, build, {
    capacity: 0,
    clock: timerClock(),
    random: Math.random,
    clientAddressHeader: null,
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("no TCP address");
  cleanup.push(async () => {
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  });
  return (pageBuild: BuildIdentity, callsign = "SCOUT", id = "1".repeat(32)) =>
    fetch(`http://127.0.0.1:${address.port}/hello`, {
      method: "POST",
      headers: { "X-Player": id, "Content-Type": "application/json" },
      body: JSON.stringify({ build: pageBuild, callsign }),
    });
}

test("hello welcomes the matching build and refuses either different commit or simulation", async () => {
  const hello = await serving();
  for (const pageBuild of [
    { ...build, commit: "c".repeat(40) },
    { ...build, sim: "d".repeat(64) },
  ]) {
    const response = await hello(pageBuild);
    expect(response.status).toBe(409);
    expect(await response.json()).toEqual({ refused: "build" });
  }
  const welcomed = await hello(build);
  expect(welcomed.status).toBe(200);
  expect(await welcomed.json()).toEqual({ callsign: "SCOUT" });
});

test("clashing browsers get numbered names, and returning browsers keep their own", async () => {
  const hello = await serving();
  const ask = async (id: string, name = "SCOUT") => (await hello(build, name, id)).json();
  expect(await ask("1".repeat(32))).toEqual({ callsign: "SCOUT" });
  expect(await ask("2".repeat(32))).toEqual({ callsign: "SCOUT#2" });
  expect(await ask("3".repeat(32))).toEqual({ callsign: "SCOUT#3" });
  expect(await ask("2".repeat(32))).toEqual({ callsign: "SCOUT#2" });
  expect(await ask("2".repeat(32), "SCOUT#2")).toEqual({ callsign: "SCOUT#2" });
  expect(await ask("1".repeat(32), "RAVEN")).toEqual({ callsign: "RAVEN" });
  expect(await ask("4".repeat(32))).toEqual({ callsign: "SCOUT" });
});

test("the page and hello clean overlong or unsupported callsigns identically", async () => {
  const { setCallsign } = await import("../../../apps/battle-lab/src/online/identity");
  const { CALLSIGN_LIMIT } = await import("../../../packages/multiplayer/src/callsign");
  const hello = await serving();
  for (const requested of ["X".repeat(CALLSIGN_LIMIT + 10), "  SCOUT☃/#_- 42  "]) {
    const pageName = setCallsign(requested);
    expect(pageName.length).toBeLessThanOrEqual(CALLSIGN_LIMIT);
    expect(/^[A-Za-z0-9 _-]+$/.test(pageName)).toBe(true);
    expect(await (await hello(build, requested)).json()).toEqual({ callsign: pageName });
  }
  expect(await (await hello(build, "☃")).json()).toEqual({ refused: "callsign" });
});

test("hello responses and logs never expose credentials, including malformed input", async () => {
  const hello = await serving();
  const spies = (["log", "info", "warn", "error", "debug"] as const).map((method) =>
    vi.spyOn(console, method).mockImplementation(() => {}),
  );
  const credential = "1234567890abcdef1234567890abcdef";
  const responses = [
    await hello(build, "SCOUT", credential),
    await hello({ ...build, commit: "wrong" }, credential, credential),
    await hello(build, "☃", credential),
    await hello(build, "SCOUT", "invalid-credential"),
    await hello(null as unknown as BuildIdentity, "SCOUT", credential),
  ];
  for (const response of responses) {
    expect(response.headers.get("cache-control")).toBe("no-store, no-transform");
    expect(response.headers.get("access-control-allow-origin")).toBe("*");
    expect(response.headers.get("access-control-allow-headers")).toBe("X-Player, Content-Type");
    expect(response.headers.get("access-control-max-age")).toBe("7200");
    expect((await response.text()).includes(credential)).toBe(false);
  }
  expect(JSON.stringify(spies.flatMap((spy) => spy.mock.calls)).includes(credential)).toBe(false);
  expect(spies.every((spy) => spy.mock.calls.length === 0)).toBe(true);
});
