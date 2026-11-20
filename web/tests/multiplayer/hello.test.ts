import { inProcessBattleMachines } from "../../../apps/game-server/src/machines/inProcess";
import { archiveFolder } from "./support/archive";
import { pendingBattle } from "./support/pendingBattle";
// @vitest-environment node
import { timerClock } from "./timerClock";
import { afterEach, expect, test, vi } from "vitest";
import { createLobbyHttp } from "../../../apps/game-server/src/http/index";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join, dirname } from "node:path";
import { buildInputs, sourceBuildIdentity } from "../../../packages/multiplayer/buildIdentity.ts";
import type { BuildIdentity } from "../../../packages/multiplayer/src/wire";

const build: BuildIdentity = { sim: "b".repeat(64), data: "a".repeat(64), wire: "e".repeat(64) };
const cleanup: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const close of cleanup.splice(0)) await close();
  vi.restoreAllMocks();
});
async function serving(serverBuild = build) {
  const server = createLobbyHttp(serverBuild, {
    archive: archiveFolder(),
    machines: inProcessBattleMachines({
      capacity: 0,
      createBattle: pendingBattle,
      ordersPerSecond: 100,
    }),
    mapAttempts: 1,
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
  const hello = (pageBuild: BuildIdentity, callsign = "SCOUT", id = "1".repeat(32)) =>
    fetch(`http://127.0.0.1:${address.port}/hello`, {
      method: "POST",
      headers: { "X-Player": id, "Content-Type": "application/json" },
      body: JSON.stringify({ build: pageBuild, callsign }),
    });
  return Object.assign(hello, { address: `http://127.0.0.1:${address.port}` });
}

test("hello welcomes the matching build and refuses any different shared part", async () => {
  const hello = await serving();
  for (const pageBuild of [
    { ...build, data: "c".repeat(64) },
    { ...build, wire: "f".repeat(64) },
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
    await hello({ ...build, data: "wrong" }, credential, credential),
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

test("hello admits equal shared inputs from different page commits", async () => {
  const hello = await serving();
  const page = { ...build, commit: "a different page commit" };
  const response = await hello(page);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual({ callsign: "SCOUT" });
});

test("file bytes alone decide admission through real hello", async () => {
  const root = join(import.meta.dirname, "../../..");
  const folder = await mkdtemp(join(root, "throwaway/multiplayer/identity-"));
  try {
    const inputs = buildInputs(root);
    for (const path of [inputs.sim, ...inputs.data, ...inputs.wire]) {
      await mkdir(dirname(join(folder, path)), { recursive: true });
      await cp(join(root, path), join(folder, path));
    }
    const serverBuild = sourceBuildIdentity(folder);
    const hello = await serving(serverBuild);
    await writeFile(join(folder, "README.md"), "Only this page's prose changed.");
    const page = { ...sourceBuildIdentity(folder), commit: "different-provenance" };
    expect(page).toMatchObject(serverBuild);
    expect(await (await hello(page)).json()).toEqual({ callsign: "SCOUT" });
    for (const path of [inputs.sim, inputs.data[0], inputs.wire[0]]) {
      const original = await readFile(join(folder, path));
      await writeFile(join(folder, path), Buffer.concat([original, Buffer.from("\n")]));
      const response = await hello(sourceBuildIdentity(folder));
      expect(response.status).toBe(409);
      expect(await response.json()).toEqual({ refused: "build" });
      await writeFile(join(folder, path), original);
    }
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});

test("the public build route states only shared hashes and never reserves a callsign", async () => {
  const server = await serving({ ...build, commit: "private-provenance" } as BuildIdentity);
  const response = await fetch(`${server.address}/build`);
  expect(response.status).toBe(200);
  expect(await response.json()).toEqual(build);
  expect(response.headers.get("cache-control")).toBe("no-store, no-transform");
  expect(await (await server(build)).json()).toEqual({ callsign: "SCOUT" });
});
