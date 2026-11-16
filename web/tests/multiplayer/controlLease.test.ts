// @vitest-environment node
import { beforeAll, expect, test, vi } from "vitest";
import { createServer } from "node:http";
import { setTimeout as delay } from "node:timers/promises";
import { createRemoteBattleHost } from "../../../apps/game-server/src/battleHost/remote";
import {
  LOBBY_LEASE_MS,
  CONTROL_PING_EVERY_MS,
} from "../../../apps/game-server/src/http/controlProtocol";
import { duel, loadBattleSupport } from "./support/battle";
import type { BattleEnd } from "../../../apps/game-server/src/battleHost/index";

beforeAll(loadBattleSupport);

test(
  "a stalled control answer survives inside the lease, recovers, then fails after a whole silent lease",
  async () => {
    let blocked = false;
    let answers = 0;
    const server = createServer((request, response) => {
      if (request.url === "/control/start")
        response.end(
          JSON.stringify({ layout: "{}", tickHz: 30, mapIdentity: "test", candidate: 0 }),
        );
      else if (request.url === "/control/events") {
        response.writeHead(200);
        response.flushHeaders();
      } else if (!blocked) {
        answers++;
        response.end(JSON.stringify({ ok: true }));
      }
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
    const ended: BattleEnd[] = [];
    const stopped = vi.fn();
    const host = createRemoteBattleHost(
      { candidates: [duel] },
      "lease",
      Promise.resolve({ address, stop: stopped }),
      "a".repeat(32),
    );
    host.onEnd((end) => ended.push(end));
    try {
      await host.ready;
      await vi.waitFor(() => expect(answers).toBeGreaterThan(0));
      blocked = true;
      await delay(LOBBY_LEASE_MS * 0.75);
      expect(ended).toEqual([]);
      blocked = false;
      const previous = answers;
      await vi.waitFor(() => expect(answers).toBeGreaterThan(previous), {
        timeout: LOBBY_LEASE_MS,
      });
      // Let an actual reply arrive and reset the lease, then cut the link again.
      await delay(CONTROL_PING_EVERY_MS / 4);
      blocked = true;
      await delay(LOBBY_LEASE_MS / 2);
      expect(ended).toEqual([]);
      await vi.waitFor(() => expect(ended).toMatchObject([{ kind: "crashed" }]), {
        timeout: LOBBY_LEASE_MS,
      });
    } finally {
      host.dispose();
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  },
  LOBBY_LEASE_MS * 4,
);

test("a broken control stream fails explicitly even while its health checks answer", async () => {
  const { encodeRecord, jsonRecord } =
    await import("../../../packages/multiplayer/src/stream/index");
  let events: import("node:http").ServerResponse | undefined;
  const server = createServer((request, response) => {
    if (request.url === "/control/events") {
      events = response;
      response.writeHead(200);
      response.flushHeaders();
    } else if (request.url === "/control/start")
      response.end(JSON.stringify({ layout: "{}", tickHz: 30, mapIdentity: "test", candidate: 0 }));
    else response.end(JSON.stringify({ ok: true }));
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  const ended: BattleEnd[] = [];
  const host = createRemoteBattleHost(
    { candidates: [duel] },
    "broken",
    Promise.resolve({ address, stop() {} }),
    "b".repeat(32),
  );
  host.onEnd((end) => ended.push(end));
  try {
    await host.ready;
    await vi.waitFor(() => expect(events).toBeDefined());
    events!.write(encodeRecord(jsonRecord("open", { resume: "keyframe" })));
    for (const id of [1, 3])
      events!.write(
        encodeRecord({ ...jsonRecord("state", { kind: "order", player: "blue-1" }), id }),
      );
    // A numbered gap is an explicit protocol failure; a healthy ping cannot
    // make a lobby safely ignore the battle's end and presence events forever.
    await vi.waitFor(() => expect(ended).toMatchObject([{ kind: "crashed" }]), {
      timeout: LOBBY_LEASE_MS / 4,
    });
  } finally {
    host.dispose();
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
});
