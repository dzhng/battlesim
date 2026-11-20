// @vitest-environment node
vi.mock("node:child_process", async (original) => {
  const actual = await original<typeof import("node:child_process")>();
  const permitted = (command: unknown) => {
    if (!["bun", "node", "git", "tar", "/bin/sh"].includes(String(command)))
      throw new Error("External process execution forbidden in deploy tests");
  };
  return {
    ...actual,
    execFileSync: (...args: Parameters<typeof actual.execFileSync>) => {
      permitted(args[0]);
      return actual.execFileSync(...args);
    },
    spawn: (...args: Parameters<typeof actual.spawn>) => {
      permitted(args[0]);
      return actual.spawn(...args);
    },
    exec: () => {
      throw new Error("Shell execution forbidden in deploy tests");
    },
    execSync: () => {
      throw new Error("Shell execution forbidden in deploy tests");
    },
    execFile: () => {
      throw new Error("Process execution forbidden in deploy tests");
    },
    fork: () => {
      throw new Error("Process execution forbidden in deploy tests");
    },
  };
});
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { join } from "node:path";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { serviceStandIn } from "../../../apps/game-server/deploy/sandbox.testSupport";
import { createRemoteBattleHost } from "../../../apps/game-server/src/battleHost/remote";
import { CONTROL_HEADER } from "../../../apps/game-server/src/http/controlProtocol";
import { httpRecords } from "../../../packages/multiplayer/src/stream/http";
import { duel, loadBattleSupport } from "./support/battle";
const root = join(import.meta.dirname, "../../..");
const archive = join(root, "throwaway/multiplayer/test-battle.tar.gz");
const cleanup: (() => Promise<unknown> | void)[] = [];
beforeAll(async () => {
  execFileSync("bun", ["apps/game-server/build.mjs"], { cwd: root });
  execFileSync("tar", [
    "-czf",
    archive,
    "-C",
    join(root, "throwaway/game-server"),
    "battle.mjs",
    "worker.mjs",
    "build.json",
    "fixtures",
    "web",
  ]);
  execFileSync(
    "bun",
    [
      "build",
      "--target=node",
      "--format=esm",
      "--outfile",
      join(root, "throwaway/multiplayer/finish/battle.mjs"),
      "apps/game-server/deploy/normalBattle.testFixture.ts",
    ],
    { cwd: root },
  );
  execFileSync("tar", [
    "-czf",
    join(root, "throwaway/multiplayer/finish-battle.tar.gz"),
    "-C",
    join(root, "throwaway/multiplayer/finish"),
    "battle.mjs",
    "-C",
    join(root, "throwaway/game-server"),
    "worker.mjs",
    "build.json",
    "fixtures",
    "web",
  ]);
  await loadBattleSupport();
}, 60_000);
afterEach(async () => {
  for (const close of cleanup.splice(0).reverse()) await close();
});
async function service() {
  const s = await serviceStandIn(archive);
  cleanup.push(s.close);
  return s;
}
const key = () => randomBytes(16).toString("hex");

test("an allocated remote battle seats a player, publishes and accepts a real order, then stops and deletes", async () => {
  const s = await service();
  const provider = await s.provider();
  const credential = key();
  const m = await provider.start(credential);
  const host = createRemoteBattleHost(
    { candidates: [duel] },
    "playable",
    Promise.resolve(m),
    credential,
    () => {},
  );
  cleanup.push(host.dispose);
  await host.ready;
  const who = key();
  await host.setSeating([[who, { side: "blue", player: "blue-1" }]]);
  const abort = new AbortController();
  cleanup.push(() => abort.abort());
  const reader = httpRecords(`${m.address}/battles/playable/stream`, who, abort.signal)[
    Symbol.asyncIterator
  ]();
  expect((await reader.next()).value?.kind).toBe("open");
  const reply = await fetch(`${m.address}/battles/playable/up`, {
    method: "POST",
    headers: { "X-Player": who, "Content-Type": "application/json" },
    body: JSON.stringify({ seq: 1, items: [{ id: 1, order: { kind: "ready" }, queued: false }] }),
  });
  expect(await reply.json()).toMatchObject({ results: [{ ack: { error: null } }] });
  let publication = false;
  for (let i = 0; i < 10 && !publication; i++)
    publication = (await reader.next()).value?.kind === "publication";
  expect(publication).toBe(true);
  abort.abort();
  await reader.return?.(undefined);
  host.dispose();
  await vi.waitFor(() => expect([...s.machines.values()].map((m) => m.state)).toEqual(["deleted"]));
  expect(s.appCredentialEscaped()).toBe(false);
  for (const secret of [credential, who])
    expect(JSON.stringify([s.seen, s.output])).not.toContain(secret);
}, 60_000);

test.each([402, 403, 409, 429, 503])(
  "a refused allocation (%s) rejects and leaves nothing running",
  async (status) => {
    const s = await service();
    s.refuse(status);
    const provider = await s.provider();
    await expect(provider.start(key())).rejects.toThrow();
    expect([...s.machines.values()]).toEqual([]);
  },
);

test("a default-tier capacity refusal falls back to a small battle machine", async () => {
  const s = await service();
  s.refuse(503, "default");
  const provider = await s.provider();
  const machine = await provider.start(key());
  machine.stop();
  await vi.waitFor(() => expect([...s.machines.values()].map((m) => m.state)).toEqual(["deleted"]));
});

test("an allocation that never becomes ready rejects and is stopped and deleted", async () => {
  const s = await service();
  s.stall();
  const provider = await s.provider({ startupMs: 100 });
  await expect(provider.start(key())).rejects.toThrow();
  await vi.waitFor(() => expect([...s.machines.values()].map((m) => m.state)).toEqual(["deleted"]));
});

test("startup removes the game's leftovers and leaves other machines alone", async () => {
  const s = await service();
  const ours = await s.leftover(true);
  const other = await s.leftover(false);
  await s.provider();
  await vi.waitFor(() => expect(ours.state).toBe("deleted"));
  expect(other.state).toBe("ready");
});

test("a leftover the service has erased but still lists does not keep a lobby from starting, and is not asked about again", async () => {
  const s = await service();
  const erased = await s.leftover(true);
  erased.state = "deleted";
  await s.provider();
  const asked = () => s.calls.filter((call) => call.includes(erased.id)).length;
  const before = asked();
  await new Promise((resolve) => setTimeout(resolve, 200));
  expect(asked()).toBe(before);
});

test("a leftover that kept its disk is deleted once the service has archived it", async () => {
  const s = await service();
  const kept = await s.leftover(true, true);
  await s.provider();
  await vi.waitFor(() => expect(kept.state).toBe("deleted"));
  expect(s.calls).toContain(`DELETE /sandboxes/${kept.id}`);
});

test("a lost create reply replays the allocation without leaking a credential or renting twice", async () => {
  const s = await service();
  s.loseCreateReply();
  const provider = await s.provider();
  const credential = key();
  const m = await provider.start(credential);
  m.stop();
  await vi.waitFor(() => expect([...s.machines.values()].map((m) => m.state)).toEqual(["deleted"]));
  expect(JSON.stringify([s.seen, s.output])).not.toContain(credential);
});

test("a crashed battle's diagnostics reach lobby output, scrubbed, before its machine stops", async () => {
  const s = await service();
  const provider = await s.provider();
  const credential = key();
  const m = await provider.start(credential);
  const host = createRemoteBattleHost(
    { candidates: [duel] },
    "crash",
    Promise.resolve(m),
    credential,
    () => {},
  );
  cleanup.push(host.dispose);
  await host.ready;
  const who = key();
  s.diagnostic(`${credential}:${who}`);
  const response = await fetch(`${m.address}/control/controller`, {
    method: "POST",
    headers: { [CONTROL_HEADER]: credential, "Content-Type": "application/json" },
    body: JSON.stringify({ player: "blue-1", kind: `${credential}:${who}` }),
  });
  expect(response.status).toBe(200);
  await vi.waitFor(() => expect(s.output.join("")).toContain("A battle crashed:"));
  await vi.waitFor(() => expect([...s.machines.values()].map((m) => m.state)).toEqual(["deleted"]));
  const diagnostic = s.calls.findIndex((c) => c.startsWith("GET") && c.includes("/commands/"));
  const stopped = s.calls.findIndex((c) => c.endsWith("/stop"));
  expect(diagnostic).toBeGreaterThanOrEqual(0);
  expect(diagnostic).toBeLessThan(stopped);
  expect(s.appCredentialEscaped()).toBe(false);
  for (const secret of [credential, who])
    expect(JSON.stringify([s.seen, s.output])).not.toContain(secret);
}, 60_000);

test("a normally ended battle automatically stops and deletes its machine", async () => {
  const s = await service();
  const provider = await s.provider({
    archive: join(root, "throwaway/multiplayer/finish-battle.tar.gz"),
  });
  const credential = key();
  const host = createRemoteBattleHost(
    { candidates: [duel] },
    "finish",
    provider.start(credential),
    credential,
    () => {},
  );
  cleanup.push(host.dispose);
  const ended: unknown[] = [];
  host.onEnd((end) => ended.push(end));
  await host.ready;
  s.finish();
  await vi.waitFor(() => expect(ended).toEqual([{ kind: "finished" }]));
  await vi.waitFor(() => expect([...s.machines.values()].map((m) => m.state)).toEqual(["deleted"]));
}, 60_000);
