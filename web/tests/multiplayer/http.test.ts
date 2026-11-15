import { pendingBattle } from "./support/pendingBattle";
// @vitest-environment node
import { timerClock } from "./timerClock";
import { PageClock } from "./pageClock";
import { join } from "node:path";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { createBattleHttp, battleRoute } from "../../../apps/game-server/src/http/index";
import { createBattleHost, type BattleClock } from "../../../apps/game-server/src/battleHost/index";
import { nodeSetupData } from "../../../apps/game-server/src/serverData";
import type { Seat } from "../../../packages/multiplayer/src/link";
import { toBattleSetup } from "../../../packages/multiplayer/src/setup";
import { preset, setOccupant } from "../../../packages/skirmish-setup/src/lobby";
import type { MapGenerator } from "../../src/maps/source";

const root = join(import.meta.dirname, "../../..");
const blueId = "11111111111111111111111111111111";
const redId = "22222222222222222222222222222222";
let data: Awaited<ReturnType<typeof nodeSetupData>>;
let generator: MapGenerator;
beforeAll(async () => {
  data = await nodeSetupData(root);
  const maps = new Map<string, string>();
  generator = {
    ...data.generator,
    generate_map(request, ...documents) {
      if (!maps.has(request)) maps.set(request, data.generator.generate_map(request, ...documents));
      return maps.get(request)!;
    },
  };
}, 60_000);
const cleanup: (() => Promise<void>)[] = [];
afterEach(async () => {
  for (const close of cleanup.splice(0)) await close();
  vi.restoreAllMocks();
});

class Clock implements BattleClock {
  time = 0;
  tasks: { at: number; run: () => void }[] = [];
  now = () => this.time;
  schedule = (run: () => void, delay: number) => {
    this.tasks.push({ at: this.time + delay, run });
  };
  advance(ms: number) {
    this.time += ms;
    for (;;) {
      const due = this.tasks.filter((t) => t.at <= this.time);
      if (!due.length) return;
      this.tasks = this.tasks.filter((t) => t.at > this.time);
      for (const t of due) t.run();
    }
  }
}
const flush = () => new Promise<void>((resolve) => setImmediate(resolve));
async function serving() {
  const setup = toBattleSetup(
    setOccupant(preset("1v1"), "seat-2", { kind: "human", who: "guest" }),
    "skirmish",
  );
  const clock = new Clock();
  const host = createBattleHost({ setup }, { clock, generator }, data);
  const ready = await host.ready;
  const ordersPerSecond = (JSON.parse(data.rules) as { commands: { orders_per_s: number } })
    .commands.orders_per_s;
  const seats = new Map<string, Seat>([
    [blueId, { side: "blue", player: "blue-1" }],
    [redId, { side: "red", player: "red-1" }],
  ]);
  const route = battleRoute(host, (id) => seats.get(id), ordersPerSecond, clock.now);
  const routes = new Map([["duel", route]]);
  const server = createBattleHttp(
    (id) => routes.get(id),
    { commit: "test", sim: "test" },
    {
      createBattle: pendingBattle,
      ordersPerSecond: 100,
      capacity: 0,
      clock: timerClock(),
      random: Math.random,
      clientAddressHeader: null,
    },
  );
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("no TCP address");
  const base = `http://127.0.0.1:${address.port}`;
  cleanup.push(async () => {
    host.dispose();
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  });
  return {
    base,
    host,
    clock,
    ready,
    route,
    routes,
    ordersPerSecond,
    async run(ticks: number) {
      for (let i = 0; i < ticks; i++) {
        clock.advance(1000 / ready.tickHz);
        await flush();
      }
    },
    request(path: string, body?: unknown, browserId = blueId) {
      return fetch(`${base}${path}`, {
        method: body === undefined ? "GET" : "POST",
        headers: { "X-Player": browserId, "Content-Type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    },
  };
}

test("every response forbids caching and the cross-origin preflight permits the credential", async () => {
  const b = await serving();
  const preflight = await fetch(`${b.base}/battles/duel/up`, {
    method: "OPTIONS",
    headers: {
      Origin: "https://game.example",
      "Access-Control-Request-Method": "POST",
      "Access-Control-Request-Headers": "X-Player, Content-Type",
    },
  });
  expect(preflight.status).toBe(204);
  expect(preflight.headers.get("access-control-allow-origin")).toBe("*");
  expect(preflight.headers.get("access-control-allow-headers")?.toLowerCase()).toContain(
    "x-player",
  );
  expect(preflight.headers.get("access-control-allow-methods")).toContain("POST");
  expect(preflight.headers.get("access-control-max-age")).toBe("7200");
  const missing = await b.request("/missing");
  expect(missing.status).toBe(404);
  const stranger = await b.request("/battles/duel/up", { seq: 1, items: [] }, "unseated");
  expect(await stranger.json()).toEqual({ seq: 1, refused: "not_seated" });
  const up = await b.request("/battles/duel/up", { seq: 1, items: [] });
  expect(await up.json()).toEqual({ seq: 1, results: [] });
  const preview = await b.request("/battles/duel/preview", {
    kind: "move",
    move: { units: [], goal: [0, 0] },
  });
  expect(await preview.json()).toEqual({ moves: [] });
  const stream = await b.request("/battles/duel/stream");
  expect(stream.headers.get("content-type")).toBe("application/octet-stream");
  expect(stream.headers.get("x-accel-buffering")).toBe("no");
  for (const response of [preflight, missing, stranger, up, preview, stream]) {
    expect(response.headers.get("cache-control")).toBe("no-store, no-transform");
    expect(response.headers.get("access-control-allow-origin")).toBe("*");
  }
  await stream.body!.cancel();
});

test("a retried purchase batch is applied once, and a forged seat cannot buy for the opponent", async () => {
  const b = await serving();
  const { nodeCatalogSet } = await import("../../src/battle/catalog/node");
  const cards = (await nodeCatalogSet("game", root)).units.view.cards.filter(
    (c) => c.disabled_reason === null,
  );
  let seq = 0;
  let purchase: unknown;
  let acknowledged: unknown;
  for (const card of cards) {
    purchase = {
      seq: ++seq,
      side: "red",
      player: "red-1",
      seat: { side: "red", player: "red-1" },
      items: [
        {
          id: 7,
          queued: false,
          order: { kind: "confirm_purchase", variant: card.id, destination: [0, 0] },
        },
      ],
    };
    const reply = await (await b.request("/battles/duel/up", purchase)).json();
    if (reply.results[0].ack.error === null) {
      acknowledged = reply;
      break;
    }
    b.clock.advance(1000);
  }
  expect(acknowledged).toBeDefined();
  const repeated = await (await b.request("/battles/duel/up", purchase)).json();
  expect(repeated).toEqual(acknowledged);
  const replay = JSON.parse(await b.host.replay()) as {
    accepted: [number, { side: string; player: string; order: { kind: string } }][];
  };
  const purchases = replay.accepted.filter(([, c]) => c.order.kind === "confirm_purchase");
  expect(purchases).toHaveLength(seq);
  expect(purchases.every(([, c]) => c.side === "blue" && c.player === "blue-1")).toBe(true);
});

test("a cut HTTP stream resumes by id with every actual battle tick decoded once", async () => {
  const { httpLink } = await import("../../src/battle/remote/httpLink");
  const { resumingReader, readJson } =
    await import("../../../packages/multiplayer/src/stream/index");
  const { ScenePresentation, loadPresentation } = await import("../../src/battle/sim/presentation");
  await loadPresentation();
  const b = await serving();
  const decoder = new ScenePresentation(b.ready.layout);
  const { loopback } = await import("../../../packages/multiplayer/src/link");
  const witness = loopback({ side: "blue", player: "blue-1" });
  b.host.attach(witness.port);
  const actual = new Map<number, string>();
  const witnessReading = (async () => {
    for await (const record of witness.link.records(null)) {
      if (record.kind === "end") return;
      if (record.kind === "publication")
        actual.set(
          new DataView(record.payload.buffer, record.payload.byteOffset).getUint32(0, true),
          new TextDecoder().decode(record.payload.subarray(4, 20)),
        );
    }
  })();
  const opens: string[] = [];
  const ticks: number[] = [];
  const ids: number[] = [];
  const frames: { tick: number; digest: string }[] = [];
  const link = httpLink(b.base, "duel", blueId);
  cleanup.push(async () => link.close());
  let connections = 0;
  let readingError: unknown = null;
  const reading = (async () => {
    for await (const record of resumingReader(
      (after) =>
        (async function* () {
          connections++;
          for await (const record of link.records(after)) {
            yield record;
            if (connections === 1 && record.kind === "publication") return;
          }
        })(),
      { sleep: async () => {} },
    )) {
      if (record.kind === "open") opens.push(readJson<{ resume: string }>(record).resume);
      if (record.kind === "end") return;
      if (record.id) ids.push(record.id);
      if (record.kind !== "publication") continue;
      const frame = decoder.decode(new Float32Array(record.payload.slice(24).buffer));
      if (!frame) throw new Error("page dropped an HTTP publication");
      const tick = new DataView(record.payload.buffer, record.payload.byteOffset).getUint32(
        0,
        true,
      );
      expect(frame.tick).toBe(tick);
      ticks.push(tick);
      frames.push({ tick, digest: new TextDecoder().decode(record.payload.subarray(4, 20)) });
    }
  })().catch((error) => {
    readingError = error;
  });
  await vi.waitFor(() => expect(opens).toEqual(["keyframe"]));
  await b.run(5);
  await vi.waitFor(() => expect(opens).toEqual(["keyframe", "continued"]));
  await b.run(5);
  b.host.dispose();
  await Promise.all([reading, witnessReading]);
  expect(readingError).toBeNull();
  expect(connections).toBe(2);
  expect(ticks.length).toBeGreaterThan(5);
  expect(ticks).toEqual(Array.from({ length: ticks.length }, (_, i) => ticks[0] + i));
  expect(ids).toEqual(Array.from({ length: ids.length }, (_, i) => ids[0] + i));
  // Compare to a second, direct connection to the same side, not a decoder
  // reading its own output: the payload survived the socket and its framing.
  expect(frames).toEqual(ticks.map((tick) => ({ tick, digest: actual.get(tick) })));
});

test("an unseated browser is refused before its order can enter the battle", async () => {
  const b = await serving();
  const before = JSON.parse(await b.host.replay());
  const response = await b.request(
    "/battles/duel/up",
    { seq: 9, items: [{ id: 8, queued: false, order: { kind: "ready" } }], player: "blue-1" },
    "stranger",
  );
  expect(response.status).toBe(403);
  expect(await response.json()).toEqual({ seq: 9, refused: "not_seated" });
  expect(JSON.parse(await b.host.replay()).accepted).toEqual(before.accepted);
});

test("excess orders are refused before recording, retries spend no allowance, and allowance recovers", async () => {
  const b = await serving();
  const items = Array.from({ length: b.ordersPerSecond }, (_, id) => ({
    id,
    queued: false,
    order: { kind: "ready" },
  }));
  const first = await (await b.request("/battles/duel/up", { seq: 1, items })).json();
  expect(first.results).toHaveLength(items.length);
  expect(first.results.every((r: { ack: { error: unknown } }) => r.ack.error === null)).toBe(true);
  expect(await (await b.request("/battles/duel/up", { seq: 1, items })).json()).toEqual(first);
  const before = JSON.parse(await b.host.replay()).accepted;
  const refused = await b.request("/battles/duel/up", { seq: 2, items: items.slice(0, 1) });
  expect(await refused.json()).toEqual({ seq: 2, refused: "rate" });
  expect(JSON.parse(await b.host.replay()).accepted).toEqual(before);
  // Red has an independent allowance even while Blue's is empty.
  const red = await (
    await b.request("/battles/duel/up", { seq: 1, items: items.slice(0, 1) }, redId)
  ).json();
  expect(red.results[0].ack.error).toBeNull();
  await b.run(b.ready.tickHz + 1);
  const next = await (
    await b.request("/battles/duel/up", { seq: 3, items: items.slice(0, 1) })
  ).json();
  expect(next.results[0].ack.error).toBeNull();
  const accepted = JSON.parse(await b.host.replay()).accepted as [number, { player: string }][];
  expect(accepted.filter(([, c]) => c.player === "blue-1")).toHaveLength(items.length + 1);
});

test("preview floods are dropped per seat and the next interval answers the actual query", async () => {
  const b = await serving();
  const query = { kind: "building", building: { units: [], building: 0 } };
  expect(await (await b.request("/battles/duel/preview", query)).json()).toEqual({
    building: null,
  });
  for (let i = 0; i < 10; i++) {
    expect(await (await b.request("/battles/duel/preview", query)).json()).toEqual({
      dropped: true,
    });
  }
  expect(await (await b.request("/battles/duel/preview", query, redId)).json()).toEqual({
    building: null,
  });
  // Exercise two seconds at 20 queries/s. The contract says about five
  // answers/s; this does not pin the transport's exact interval.
  let answered = 0;
  for (let i = 0; i < 40; i++) {
    b.clock.advance(50);
    const answer = await (await b.request("/battles/duel/preview", query)).json();
    if ("building" in answer) {
      expect(answer).toEqual({ building: null });
      answered++;
    } else expect(answer).toEqual({ dropped: true });
  }
  expect(answered).toBeGreaterThanOrEqual(8);
  expect(answered).toBeLessThanOrEqual(12);
  // A later intent gets its own answer rather than a cached old preview.
  b.clock.advance(1000);
  expect(
    await (
      await b.request("/battles/duel/preview", { kind: "move", move: { units: [], goal: [0, 0] } })
    ).json(),
  ).toEqual({ moves: [] });
  expect(JSON.parse(await b.host.replay()).accepted).toEqual([]);
});

test("headers never reach a logger or any response, including malformed requests and ended battles", async () => {
  const b = await serving();
  const logs = ["log", "info", "warn", "error", "debug"] as const;
  const spies = logs.map((method) => vi.spyOn(console, method).mockImplementation(() => {}));
  const secret = "private-proxy-credential";
  const responses: Response[] = [];
  responses.push(
    await fetch(`${b.base}/absent`, { headers: { "X-Player": blueId, "X-Edge-Secret": secret } }),
  );
  responses.push(
    await fetch(`${b.base}/battles/duel/up`, {
      method: "POST",
      headers: { "X-Player": blueId, "X-Edge-Secret": secret },
      body: "{",
    }),
  );
  b.host.dispose();
  responses.push(
    await b.request("/battles/duel/up", {
      seq: 1,
      items: [{ id: 1, queued: false, order: { kind: "ready" } }],
    }),
  );
  responses.push(
    await b.request("/battles/duel/preview", { kind: "move", move: { units: [], goal: [0, 0] } }),
  );
  expect(responses.map((r) => r.status)).toEqual([404, 400, 410, 410]);
  const output =
    (await Promise.all(responses.map((r) => r.text()))).join("\n") +
    JSON.stringify(spies.flatMap((s) => s.mock.calls));
  for (const value of [secret, blueId, redId]) expect(output).not.toContain(value);
  expect(output).not.toContain("Error:");
  expect(output).not.toContain("at ");
  for (const response of responses)
    expect(response.headers.get("cache-control")).toBe("no-store, no-transform");
});

test("the online client decodes a real loopback battle, survives visibility changes, and offers only seat actions", async () => {
  const { loopback } = await import("../../../packages/multiplayer/src/link");
  const { remoteConnection } = await import("../../src/battle/remote/connection");
  const { createSimClient } = await import("../../src/battle/sim/client");
  const b = await serving();
  const pair = loopback({ side: "blue", player: "blue-1" });
  const handlers = b.route.seat(blueId)!;
  pair.port.onReader(handlers.reader);
  pair.port.onUp(handlers.up);
  pair.port.onPreview(handlers.preview);
  const document = new EventTarget();
  const listeners = vi.spyOn(document, "addEventListener");
  vi.stubGlobal("document", document);
  const pageClock = new PageClock();
  const run = async (ticks: number) => {
    for (let i = 0; i < ticks; i++) {
      await b.run(1);
      await pageClock.advance(1000 / b.ready.tickHz);
    }
  };
  const client = createSimClient({
    side: "blue",
    transport: "remote",
    connect: remoteConnection(pair.link, pageClock),
  });
  const ticks: number[] = [];
  const status: string[] = [];
  client.onStatus((s) => status.push(s));
  client.onPublication((p) => {
    ticks.push(p.observation.tick);
    p.release();
  });
  try {
    await run(3);
    const ready = await client.ready;
    expect(ready.tick).toBeGreaterThan(0);
    client.start();
    // It starts on a snapshot of its own, which a side gives at most once a
    // second: the one that described the battle to it was a moment ago.
    await run(ready.tickHz + 10);
    expect(status).toContain("running");
    const before = ticks.at(-1)!;
    document.dispatchEvent(new Event("visibilitychange"));
    await run(3);
    expect(ticks.at(-1)).toBeGreaterThan(before);
    expect(listeners.mock.calls.filter(([event]) => event === "visibilitychange")).toEqual([]);
    expect(status).not.toContain("failed");
    expect(await client.previewMove({ units: [], goal: [0, 0] })).toEqual([]);
    await expect(client.previewMove({ units: [], goal: [0, 0] })).rejects.toThrow("dropped");
    const ack = await client.command({ kind: "ready" });
    expect(ack.error).toBeNull();
    for (const action of [
      "pause",
      "resume",
      "advance",
      "paused",
      "replay",
      "observeAs",
      "setControllers",
    ])
      expect(action in client).toBe(false);
    // A rate refusal rejects this action rather than fabricating an ack or
    // failing the battle; a later command still gets a real simulation ack.
    const flood = Array.from({ length: b.ordersPerSecond + 1 }, () =>
      client.command({ kind: "ready" }),
    );
    expect(
      (await Promise.allSettled(flood)).every(
        (r) => r.status === "rejected" && r.reason.message === "rate",
      ),
    ).toBe(true);
    await b.run(b.ready.tickHz + 1);
    expect((await client.command({ kind: "ready" })).error).toBeNull();
  } finally {
    client.dispose();
    vi.unstubAllGlobals();
  }
});

test("malformed bodies and cursors are refused without ending or recording into the battle", async () => {
  const b = await serving();
  for (const body of [null, {}, { seq: 1, items: [null] }, { seq: -1, items: [] }]) {
    const response = await b.request("/battles/duel/up", body);
    expect(response.status).toBe(400);
    expect(await response.json()).toEqual({ refused: "invalid" });
  }
  const cursor = await b.request("/battles/duel/stream?after=NaN");
  expect(cursor.status).toBe(400);
  expect(JSON.parse(await b.host.replay()).accepted).toEqual([]);
  expect(
    await (
      await b.request("/battles/duel/up", {
        seq: 1,
        items: [{ id: 1, queued: false, order: { kind: "ready" } }],
      })
    ).json(),
  ).toMatchObject({ seq: 1, results: [{ id: 1, ack: { error: null } }] });
});

test("a reservation for a seat the host does not have gets an honest HTTP refusal", async () => {
  const b = await serving();
  b.routes.set(
    "bad",
    battleRoute(
      b.host,
      (id) => (id === blueId ? { side: "blue", player: "missing" } : undefined),
      b.ordersPerSecond,
    ),
  );
  const response = await b.request("/battles/bad/up", { seq: 1, items: [] });
  expect(response.status).toBe(410);
  expect(await response.json()).toEqual({ refused: "unavailable" });
  expect(
    await (
      await b.request("/battles/duel/up", {
        seq: 1,
        items: [{ id: 1, queued: false, order: { kind: "ready" } }],
      })
    ).json(),
  ).toMatchObject({ results: [{ ack: { error: null } }] });
});
