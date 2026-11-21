// @vitest-environment node
import { afterEach, beforeAll, expect, test } from "vitest";
import { archiveFolder } from "./support/archive";
import { randomBytes } from "node:crypto";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { childBattleMachines } from "../../../apps/game-server/src/machines/child";
import { createRemoteBattleHost } from "../../../apps/game-server/src/battleHost/remote";
import { duel, loadBattleSupport, FRONTLINE_DRAWS } from "./support/battle";

const root = join(import.meta.dirname, "../../..");
const cleanups: (() => void)[] = [];
beforeAll(async () => {
  execFileSync("bun", ["apps/game-server/build.mjs"], { cwd: root, stdio: "pipe" });
  await loadBattleSupport();
}, 60_000);
afterEach(() => {
  for (const close of cleanups.splice(0).reverse()) close();
});

async function machine() {
  const key = randomBytes(16).toString("hex");
  const machines = childBattleMachines(new URL(`file://${root}/throwaway/game-server/battle.mjs`));
  const m = await machines.start(key);
  cleanups.push(m.stop);
  return { m, key };
}

test("a battle refuses an unauthenticated controller and an unseated browser without disclosing credentials", async () => {
  const { m, key } = await machine();
  const who = randomBytes(16).toString("hex");
  const denied = await fetch(`${m.address}/control/referee`, { method: "POST" });
  expect(denied.status).toBe(403);
  expect((await denied.text()).includes(key), "credential appeared in readable output").toBe(false);
  const host = createRemoteBattleHost(
    { candidates: [duel] },
    "duel",
    Promise.resolve(m),
    key,
    () => {},
  );
  cleanups.push(host.dispose);
  await host.ready;
  const reply = await fetch(`${m.address}/battles/duel/up`, {
    method: "POST",
    headers: { "X-Player": who, "Content-Type": "application/json" },
    body: JSON.stringify({ seq: 1, items: [{ id: 1, order: { kind: "ready" }, queued: false }] }),
  });
  expect(reply.status).toBe(403);
  const text = await reply.text();
  expect(JSON.parse(text)).toEqual({ seq: 1, refused: "not_seated" });
  expect(text.includes(key), "credential appeared in readable output").toBe(false);
  expect(text.includes(who), "credential appeared in readable output").toBe(false);
  expect(JSON.parse(await host.replay()).accepted).toEqual([]);
}, 60_000);

// The lobby runs in this test; every battle below runs the actual Node bundle.
import { vi } from "vitest";
import { createLobbyHttp } from "../../../apps/game-server/src/http/index";
import type { Machine, BattleMachines } from "../../../apps/game-server/src/machines/index";
import { httpRecords } from "../../../packages/multiplayer/src/stream/http";
import { readJson } from "../../../packages/multiplayer/src/stream/index";
import type { BattleTicket, RoomView, StreamRecord } from "../../../packages/multiplayer/src/wire";
import { CONTROL_HEADER, LOBBY_LEASE_MS } from "../../../apps/game-server/src/http/controlProtocol";
import { PageClock } from "./pageClock";
import { PRESENCE_GRACE_MS } from "../../../apps/game-server/src/presence";
import { fork, spawn } from "node:child_process";
import { once } from "node:events";

const build = { data: "process-test", wire: "process-test", sim: "process-test" };
const ids = ["1".repeat(32), "2".repeat(32)];
async function lobby() {
  const clock = new PageClock();
  let draw = 0;
  let remote!: Machine;
  let key = "";
  let battlePid = 0;
  const provider = childBattleMachines(new URL(`file://${root}/throwaway/game-server/battle.mjs`));
  const machines: BattleMachines = {
    capacity: 1,
    async start(credential) {
      key = credential;
      remote = await provider.start(credential);
      cleanups.push(remote.stop);
      const rows = execFileSync("ps", ["-axo", "pid=,ppid=,command="], { encoding: "utf8" }).split(
        "\n",
      );
      battlePid = Math.max(
        ...rows.flatMap((row) => {
          const match = /^\s*(\d+)\s+(\d+)\s+(.+)$/.exec(row);
          return match && Number(match[2]) === process.pid && match[3].includes("battle.mjs")
            ? [Number(match[1])]
            : [];
        }),
      );
      return remote;
    },
  };
  const server = createLobbyHttp(build, {
    archive: archiveFolder(),
    machines,
    clock,
    random: () => FRONTLINE_DRAWS[draw++ % FRONTLINE_DRAWS.length],
    mapAttempts: 5,
    clientAddressHeader: null,
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  const stop = () => {
    roomAbort?.abort();
    server.closeAllConnections();
    server.close();
  };
  cleanups.push(stop);
  const replies: string[] = [];
  async function post(path: string, value: unknown, who = ids[0], base = address) {
    const response = await fetch(`${base}${path}`, {
      method: "POST",
      headers: { "X-Player": who, "Content-Type": "application/json" },
      body: JSON.stringify(value),
    });
    const text = await response.text();
    replies.push(text);
    return JSON.parse(text);
  }
  for (const [i, who] of ids.entries())
    await post("/hello", { build, callsign: `SCOUT-${i}` }, who);
  const records: StreamRecord[] = [];
  let roomPath = "";
  let roomAbort: AbortController | undefined;
  async function room() {
    const made = await post("/rooms", { visibility: "private" });
    roomPath = `/rooms/${made.code}`;
    roomAbort = new AbortController();
    cleanups.push(() => roomAbort?.abort());
    void (async () => {
      try {
        for await (const record of httpRecords(`${address}${roomPath}`, ids[0], roomAbort!.signal))
          records.push(record);
      } catch (error) {
        if (!roomAbort!.signal.aborted) throw error;
      }
    })();
    await vi.waitFor(() => expect(records.some((r) => r.kind === "state")).toBe(true));
    for (const act of [
      { kind: "set_seat", seat: "seat-2", to: "ai" },
      { kind: "settings", patch: { map: { type: "open", size: "small", seed: "1" } } },
      { kind: "ready", ready: true },
      { kind: "start" },
    ])
      expect(await post(`${roomPath}/act`, act)).toEqual({ ok: true });
    let ticket!: BattleTicket;
    await vi.waitFor(
      () => {
        const view = readJson<RoomView>(records.filter((r) => r.kind === "state").at(-1)!);
        expect(view.phase.kind).toBe("battle");
        if (view.phase.kind === "battle") ticket = view.phase.ticket;
      },
      { timeout: 30_000 },
    );
    return ticket;
  }
  async function control(action: string) {
    const response = await fetch(`${remote.address}/control/${action}`, {
      method: "POST",
      headers: { [CONTROL_HEADER]: key },
    });
    const text = await response.text();
    replies.push(text);
    return JSON.parse(text);
  }
  return {
    clock,
    address,
    post,
    records,
    room,
    control,
    stop,
    replies,
    remote: () => remote,
    key: () => key,
    killBattle: () => process.kill(battlePid, "SIGKILL"),
    closeRoom: () => roomAbort?.abort(),
  };
}

async function unreachable(address: string) {
  await vi.waitFor(
    async () => {
      await expect(fetch(address, { signal: AbortSignal.timeout(500) })).rejects.toThrow();
    },
    { timeout: 15_000 },
  );
}

test("losing a battle server fails its room, frees admission, and leaves lobby routes alive", async () => {
  const l = await lobby();
  const ticket = await l.room();
  expect(ticket.address).not.toBe(l.address);
  expect((await fetch(`${l.address}/battles/${ticket.battle}/stream`)).status).toBe(404);
  expect(await l.post("/rooms", { visibility: "private" }, ids[1])).toEqual({ refused: "full" });
  l.killBattle();
  await vi.waitFor(
    () =>
      expect(l.records.filter((r) => r.kind === "end").map((r) => readJson(r))).toEqual([
        { reason: "battle_failed" },
      ]),
    { timeout: 15_000 },
  );
  expect(await l.post("/rooms", { visibility: "private" }, ids[1])).toEqual({
    code: expect.any(String),
  });
  expect(l.replies.join("\n").includes(l.key()), "credential appeared in readable output").toBe(
    false,
  );
}, 60_000);

test("a dropped battle stream keeps its seat through presence grace and reclaims the same army after AI takeover", async () => {
  const l = await lobby();
  const ticket = await l.room();
  const path = `/battles/${ticket.battle}`;
  async function open() {
    const response = await fetch(`${ticket.address}${path}/stream`, {
      headers: { "X-Player": ids[0] },
    });
    expect(response.status).toBe(200);
    return response.body!.getReader();
  }
  let reader = await open();
  await reader.read();
  l.closeRoom();
  // A referee round trip lets the stream-open event reach the lobby first.
  await l.control("referee");
  await vi.waitFor(async () =>
    expect(
      (await l.control("referee")).seats.find((s: { player: string }) => s.player === ticket.player)
        .controller,
    ).toBe("human"),
  );
  await reader.cancel();
  await new Promise((resolve) => setTimeout(resolve, 100));
  await l.clock.advance(PRESENCE_GRACE_MS - 1);
  expect(
    (await l.control("referee")).seats.find((s: { player: string }) => s.player === ticket.player)
      .controller,
  ).toBe("human");
  // Return inside the grace; its own orders still work.
  reader = await open();
  await reader.read();
  expect(
    await l.post(
      `${path}/up`,
      { seq: 1, items: [{ id: 1, order: { kind: "ready" }, queued: false }] },
      ids[0],
      ticket.address,
    ),
  ).toMatchObject({ results: [{ ack: { error: null } }] });
  await reader.cancel();
  await new Promise((resolve) => setTimeout(resolve, 100));
  await l.clock.advance(PRESENCE_GRACE_MS);
  await vi.waitFor(async () =>
    expect(
      (await l.control("referee")).seats.find((s: { player: string }) => s.player === ticket.player)
        .controller,
    ).toBe("active_ai"),
  );
  const before = (await l.control("referee")).seats.find(
    (s: { player: string }) => s.player === ticket.player,
  );
  reader = await open();
  await reader.read();
  cleanups.push(() => {
    void reader.cancel();
  });
  await vi.waitFor(async () =>
    expect(
      (await l.control("referee")).seats.find((s: { player: string }) => s.player === ticket.player)
        .controller,
    ).toBe("human"),
  );
  const after = (await l.control("referee")).seats.find(
    (s: { player: string }) => s.player === ticket.player,
  );
  expect(after.living).toBeGreaterThanOrEqual(before.living);
  expect(
    await l.post(
      `${path}/up`,
      { seq: 2, items: [{ id: 2, order: { kind: "ready" }, queued: false }] },
      ids[0],
      ticket.address,
    ),
  ).toMatchObject({ results: [{ ack: { error: null } }] });
  expect(JSON.parse(await l.control("replay")).seat_events).toBeUndefined();
  for (const credential of [...ids, l.key()])
    expect(
      l.replies.join("\n").includes(credential),
      "credential appeared in readable output",
    ).toBe(false);
}, 60_000);

test("stopping the lobby stops its running battle", async () => {
  const l = await lobby();
  const ticket = await l.room();
  l.stop();
  await unreachable(ticket.address!);
}, 60_000);

test("a battle stops itself when its authenticated lobby stops renewing the lease", async () => {
  const { m, key } = await machine();
  const response = await fetch(`${m.address}/control/start`, {
    method: "POST",
    headers: { [CONTROL_HEADER]: key, "Content-Type": "application/json" },
    body: JSON.stringify({ id: "lease", candidates: [duel] }),
  });
  expect(response.status).toBe(200);
  await unreachableAfterLease(m.address);
}, 60_000);
async function unreachableAfterLease(address: string) {
  await vi.waitFor(
    async () => {
      await expect(fetch(address)).rejects.toThrow();
    },
    { timeout: LOBBY_LEASE_MS * 2 },
  );
}

test("even a lobby killed without cleanup leaves no battle process behind", async () => {
  const child = spawn("node", [join(root, "throwaway/game-server/main.mjs")], {
    env: { ...process.env, PORT: "0" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  cleanups.push(() => {
    child.kill("SIGKILL");
  });
  let log = "";
  child.stdout.on("data", (b) => {
    log += b;
  });
  child.stderr.on("data", (b) => {
    log += b;
  });
  await vi.waitFor(() => expect(log).toMatch(/Game server: http/));
  const address = /Game server: (http:\/\/127\.0\.0\.1:\d+)/.exec(log)![1];
  // Use the bundle's identity, exactly as the browser does.
  const { readFileSync } = await import("node:fs");
  const identity = JSON.parse(readFileSync(join(root, "throwaway/game-server/build.json"), "utf8"));
  async function post(path: string, value: unknown) {
    return (
      await fetch(`${address}${path}`, {
        method: "POST",
        headers: { "X-Player": ids[0], "Content-Type": "application/json" },
        body: JSON.stringify(value),
      })
    ).json();
  }
  await post("/hello", { build: identity, callsign: "SCOUT" });
  const made = await post("/rooms", { visibility: "private" });
  await post(`/rooms/${made.code}/act`, { kind: "set_seat", seat: "seat-2", to: "ai" });
  await post(`/rooms/${made.code}/act`, {
    kind: "settings",
    patch: { map: { type: "open", size: "small", seed: "1" } },
  });
  await post(`/rooms/${made.code}/act`, { kind: "ready", ready: true });
  await post(`/rooms/${made.code}/act`, { kind: "start" });
  const roomAbort = new AbortController();
  cleanups.push(() => roomAbort.abort());
  for await (const record of httpRecords(
    `${address}/rooms/${made.code}`,
    ids[0],
    roomAbort.signal,
  )) {
    if (record.kind === "state" && readJson<RoomView>(record).phase.kind === "battle") break;
    if (record.kind === "end") throw new Error("battle failed before the lobby was killed");
  }
  roomAbort.abort();
  let battlePid = 0;
  await vi.waitFor(() => {
    const children = execFileSync("ps", ["-axo", "pid=,ppid=,command="], { encoding: "utf8" });
    const row = children.split("\n").find((row) => {
      const match = /^\s*(\d+)\s+(\d+)\s+(.+)$/.exec(row);
      return match && Number(match[2]) === child.pid && match[3].includes("battle.mjs");
    });
    expect(row).toBeDefined();
    battlePid = Number(row!.trim().split(/\s+/)[0]);
  });
  cleanups.push(() => {
    try {
      process.kill(battlePid, "SIGKILL");
    } catch {
      /* Already stopped. */
    }
  });
  const exited = once(child, "exit");
  child.kill("SIGKILL");
  await exited;
  await vi.waitFor(() => expect(() => process.kill(battlePid, 0)).toThrow(), { timeout: 15_000 });
  for (const credential of ids)
    expect(log.includes(credential), "credential appeared in readable output").toBe(false);
}, 60_000);

test("a battle key and seated browser credential never appear in readable replies, errors or process logs", async () => {
  const key = randomBytes(16).toString("hex");
  const who = randomBytes(16).toString("hex");
  let output = "";
  const child = fork(join(root, "throwaway/game-server/battle.mjs"), [], {
    env: { ...process.env, BATTLE_KEY: key },
    stdio: ["ignore", "pipe", "pipe", "ipc"],
  });
  cleanups.push(() => {
    child.kill();
  });
  child.stdout!.on("data", (b) => {
    output += b;
  });
  child.stderr!.on("data", (b) => {
    output += b;
  });
  const [message] = (await once(child, "message")) as [{ address: string }];
  const responses: string[] = [];
  async function post(action: string, value: unknown, credential: string | null = key) {
    const response = await fetch(`${message.address}/control/${action}`, {
      method: "POST",
      headers: {
        ...(credential && { [CONTROL_HEADER]: credential }),
        "Content-Type": "application/json",
      },
      body: typeof value === "string" ? value : JSON.stringify(value),
    });
    responses.push(await response.text());
    return response.status;
  }
  expect(await post("start", { id: "credentials", candidates: [duel] })).toBe(200);
  expect(await post("seating", [[who, { side: "blue", player: "blue-1" }]])).toBe(200);
  expect(await post("referee", {}, null)).toBe(403);
  expect(await post("controller", "{")).toBe(400);
  // Make a genuine authority error quote the key. The control event and the
  // diagnostic must both remove it rather than trusting exception text.
  expect(await post("seating", [])).toBe(200);
  expect(await post("controller", { player: "blue-1", kind: `${key}:${who}` })).toBe(200);
  await vi.waitFor(() => expect(output).toContain("A battle crashed:"));
  await post("replay", {});
  for (const credential of [key, who])
    expect(
      (responses.join("\n") + output).includes(credential),
      "credential appeared in readable output",
    ).toBe(false);
}, 60_000);

test("an unreachable control surface is a failed room and its machine is stopped", async () => {
  const l = await lobby();
  const ticket = await l.room();
  l.remote().address += "/unreachable";
  await vi.waitFor(
    () =>
      expect(l.records.filter((r) => r.kind === "end").map((r) => readJson(r))).toEqual([
        { reason: "battle_failed" },
      ]),
    { timeout: 15_000 },
  );
  await unreachable(ticket.address!);
  expect(await l.post("/rooms", { visibility: "private" }, ids[1])).toEqual({
    code: expect.any(String),
  });
}, 60_000);

test("a dead Frontline battle is removed and releases the shared battle place", async () => {
  const l = await lobby();
  const briefing = await l.post("/frontline/join", {});
  expect(briefing).toHaveProperty("battle");
  const ticket = await l.post(`/frontline/${briefing.battle}/deploy`, {});
  expect(ticket).toMatchObject({ battle: briefing.battle, address: expect.any(String) });
  l.killBattle();
  await vi.waitFor(
    async () =>
      expect(await l.post(`/frontline/${briefing.battle}/deploy`, {})).toEqual({
        refused: "expired",
      }),
    { timeout: 15_000 },
  );
  const abort = new AbortController();
  for await (const record of httpRecords(`${l.address}/rooms`, ids[0], abort.signal)) {
    if (record.kind !== "state") continue;
    expect(
      readJson<{ frontline: { battles: number; commanders: number } }>(record).frontline,
    ).toEqual({ battles: 0, commanders: 0 });
    break;
  }
  abort.abort();
  expect(await l.post("/rooms", { visibility: "private" }, ids[1])).toEqual({
    code: expect.any(String),
  });
}, 120_000);

test("the lobby's record stream resumes after a cut without losing or repeating player activity", async () => {
  const { m, key } = await machine();
  async function post(path: string, value: unknown, control = true) {
    const response = await fetch(`${m.address}${path}`, {
      method: "POST",
      headers: {
        ...(control ? { [CONTROL_HEADER]: key } : { "X-Player": ids[0] }),
        "Content-Type": "application/json",
      },
      body: JSON.stringify(value),
    });
    expect(response.status).toBe(200);
    return response.json();
  }
  await post("/control/start", { id: "resume", candidates: [duel] });
  await post("/control/seating", [[ids[0], { side: "blue", player: "blue-1" }]]);
  const abort = new AbortController();
  const reader = httpRecords(
    `${m.address}/control/events`,
    { [CONTROL_HEADER]: key },
    abort.signal,
  )[Symbol.asyncIterator]();
  expect(readJson((await reader.next()).value!)).toEqual({ resume: "keyframe" });
  expect(readJson((await reader.next()).value!)).toEqual({
    kind: "snapshot",
    streams: [],
    orders: {},
  });
  const order = async (seq: number) =>
    post(
      "/battles/resume/up",
      { seq, items: [{ id: seq, order: { kind: "ready" }, queued: false }] },
      false,
    );
  await order(1);
  const first = (await reader.next()).value!;
  expect(readJson(first)).toEqual({ kind: "order", player: "blue-1" });
  abort.abort();
  await reader.return?.(undefined);
  await order(2);
  const back = new AbortController();
  const resumed = httpRecords(
    `${m.address}/control/events?after=${first.id}`,
    { [CONTROL_HEADER]: key },
    back.signal,
  )[Symbol.asyncIterator]();
  expect(readJson((await resumed.next()).value!)).toEqual({ resume: "continued" });
  const next = (await resumed.next()).value!;
  expect(next.id).toBe(first.id + 1);
  expect(readJson(next)).toEqual({ kind: "order", player: "blue-1" });
  back.abort();
  await resumed.return?.(undefined);
}, 60_000);

test("a refused map releases its machine before the next battle is admitted", async () => {
  const { inProcessBattleMachines } =
    await import("../../../apps/game-server/src/machines/inProcess");
  const { createBattleHost } = await import("../../../apps/game-server/src/battleHost/index");
  const { data, generator } = await import("./support/battle");
  const clock = new PageClock();
  const provider = inProcessBattleMachines({
    capacity: 1,
    createBattle: (spec) => createBattleHost(spec, { clock, generator }, data),
    ordersPerSecond: JSON.parse(data.rules).commands.orders_per_s,
  });
  const key = randomBytes(16).toString("hex");
  const refused = createRemoteBattleHost(
    { candidates: [{ ...duel, map: { ...duel.map, type: "unadmitted" as never } }] },
    "refused",
    provider.start(key),
    key,
    () => {},
  );
  cleanups.push(refused.dispose);
  await expect(refused.ready).rejects.toThrow();
  const next = createRemoteBattleHost(
    { candidates: [duel] },
    "admitted",
    provider.start(key),
    key,
    () => {},
  );
  cleanups.push(next.dispose);
  expect(await next.ready).toMatchObject({
    layout: expect.any(String),
    tickHz: expect.any(Number),
    mapIdentity: expect.any(String),
  });
}, 60_000);

test("a malformed request target is refused without killing the battle server", async () => {
  const { m, key } = await machine();
  const { request } = await import("node:http");
  const reply = await new Promise<{ status: number; body: string }>((resolve, reject) => {
    const pending = request(m.address, { path: "http://invalid:port/" }, (response) => {
      let body = "";
      response.on("data", (chunk) => {
        body += chunk;
      });
      response.on("end", () => resolve({ status: response.statusCode!, body }));
    });
    pending.on("error", () => reject(new Error("malformed target terminated the battle listener")));
    pending.end();
  });
  expect(reply).toEqual({ status: 410, body: JSON.stringify({ refused: "unavailable" }) });
  const ping = await fetch(`${m.address}/control/ping`, {
    method: "POST",
    headers: { [CONTROL_HEADER]: key },
  });
  expect(ping.status).toBe(200);
}, 60_000);

test("a machine that cannot start releases its allocation for a later battle", async () => {
  const warnings: string[] = [];
  const warned = (warning: Error) => {
    warnings.push(warning.message);
  };
  process.on("warning", warned);
  cleanups.push(() => {
    process.off("warning", warned);
  });
  const key = randomBytes(16).toString("hex");
  const provider = childBattleMachines(new URL(`file://${root}/throwaway/game-server/battle.mjs`));
  const executable = process.execPath;
  try {
    process.execPath = join(root, "throwaway/multiplayer/missing-node");
    for (let i = 0; i < provider.capacity; i++) await expect(provider.start(key)).rejects.toThrow();
  } finally {
    process.execPath = executable;
  }
  const m = await provider.start(key);
  cleanups.push(m.stop);
  const ping = await fetch(`${m.address}/control/ping`, {
    method: "POST",
    headers: { [CONTROL_HEADER]: key },
  });
  expect(ping.status).toBe(200);
  expect(warnings, "failed starts must leave no process warnings").toEqual([]);
}, 60_000);
