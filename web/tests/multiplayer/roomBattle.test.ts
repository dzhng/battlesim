import { inProcessBattleMachines } from "../../../apps/game-server/src/machines/inProcess";
import { createOnlineState } from "../../../apps/game-server/src/online";
import { nodeCatalogSet } from "../../src/battle/catalog/node";
import { join } from "node:path";
// @vitest-environment node
import { afterEach, beforeAll, expect, test } from "vitest";
import { createRoomService, EMPTY_ROOM_TIMEOUT_MS } from "../../../apps/game-server/src/rooms";
import {
  createBattleHost,
  type BattleHost,
  type BattleEnd,
} from "../../../apps/game-server/src/battleHost/index";
import { PRESENCE_GRACE_MS } from "../../../apps/game-server/src/presence";
import { readJson } from "../../../packages/multiplayer/src/stream/index";
import type { RoomView, RoomList, StreamRecord } from "../../../packages/multiplayer/src/wire";
import type { ControllerKind } from "../../src/battle/prepare/protocol";
import { PageClock } from "./pageClock";
import {
  loadBattleSupport,
  data,
  generator,
  seat,
  buy,
  firstUnit,
  yardstick,
  digests,
  tickOf,
  flush,
} from "./support/battle";

beforeAll(loadBattleSupport, 60_000);
const cleanups: (() => void)[] = [];
afterEach(() => {
  for (const close of cleanups.splice(0).reverse()) close();
});

async function room() {
  const clock = new PageClock();
  let host!: BattleHost;
  let draw = 0;
  const service = createRoomService({
    online: createOnlineState({
      capacity: 1,
      clock: clock,
      callsign: (who) => ({ alpha: "SCOUT", bravo: "WING", stranger: "STRANGER" })[who],
      releaseCallsign: () => {},
    }),
    random: () => (++draw % 1000) / 1000,
    createBattle: (candidates) =>
      (host = createBattleHost({ candidates }, { clock, generator }, data)),
  });
  cleanups.push(service.dispose);
  const made = service.create("alpha", "public");
  if ("refused" in made) throw new Error(made.refused);
  const code = made.code;
  function open(who: string) {
    const records: StreamRecord[] = [];
    const stream = service.roomStream(who, code, "local", null, (r) => {
      records.push(r);
      return true;
    });
    if ("refused" in stream) throw new Error(stream.refused);
    cleanups.push(stream.close);
    return {
      close: stream.close,
      records,
      view: () => readJson<RoomView>(records.filter((r) => r.kind === "state").at(-1)!),
    };
  }
  const alpha = open("alpha");
  const bravo = open("bravo");
  const list: RoomList[] = [];
  cleanups.push(
    service.listStream("stranger", null, (r) => {
      if (r.kind === "state") list.push(readJson<RoomList>(r));
      return true;
    }),
  );
  const act = (kind: Parameters<typeof service.act>[2], who = "alpha") => {
    expect(service.act(who, code, kind)).toEqual({ ok: true });
  };
  // A hole before a filled seat proves compaction, rather than just filtering a trailing seat.
  act({ kind: "add_seat", side: "blue" });
  act({ kind: "set_seat", seat: "seat-3", to: "open" });
  act({ kind: "sit", seat: "seat-3" });
  act({ kind: "settings", patch: { map: { seed: "77" } } });
  act({ kind: "ready", ready: true });
  act({ kind: "ready", ready: true }, "bravo");
  const start = async () => {
    act({ kind: "start" });
    expect(alpha.view().phase).toEqual({ kind: "starting" });
    const ready = await host.ready;
    await flush();
    const ended: BattleEnd[] = [];
    host.onEnd((e) => ended.push(e));
    const run = async (ticks: number) => {
      for (let i = 0; i < ticks; i++) await clock.advance(1000 / ready.tickHz);
    };
    return { host, ready, ended, run };
  };
  return { service, clock, code, alpha, bravo, list, open, act, start };
}
function ticket(view: RoomView) {
  if (view.phase.kind !== "battle") throw new Error("room has no battle ticket");
  return view.phase.ticket;
}
const orders = async (host: BattleHost, player: string) =>
  (
    JSON.parse(await host.replay()) as {
      accepted: [
        number,
        { player: string; order: import("../../src/battle/sim/protocol").Order },
      ][];
    }
  ).accepted.filter(([, c]) => c.player === player);

test("Start unlists and locks the room; each member receives only their own compacted seat's ticket", async () => {
  const r = await room();
  const b = await r.start();
  const [a, g] = [ticket(r.alpha.view()), ticket(r.bravo.view())];
  expect(ticket(r.open("bravo").view())).toEqual(g);
  expect(r.list.at(-1)).toEqual({ rooms: [], frontline: { battles: 0, commanders: 0 } });
  expect(r.service.roomStream("stranger", r.code, "other", null, () => true)).toEqual({
    refused: "started",
  });
  expect(a).toMatchObject({
    battle: expect.any(String),
    player: "blue-1",
    side: "blue",
    mapIdentity: b.ready.mapIdentity,
  });
  expect(g).toMatchObject({
    battle: a.battle,
    player: "red-1",
    side: "red",
    mapIdentity: a.mapIdentity,
  });
  expect(a.setup.seats).toEqual([
    { player: "blue-1", side: "blue", controller: "human" },
    { player: "red-1", side: "red", controller: "human" },
  ]);
  const left = seat(b, a),
    right = seat(b, g);
  const seen = left.read(),
    redSeen = right.read();
  await b.run(3);
  expect(
    [...seen.latest!.skirmish!.players, ...redSeen.latest!.skirmish!.players].map((p) => p.id),
  ).toEqual(a.setup.seats.map((p) => p.player));
  expect((await right.order({ kind: "ready" })).error).toBeNull();
  expect(r.service.battle(a.battle)?.seatOf("alpha")).toEqual({ player: a.player, side: a.side });
  expect(r.service.battle(a.battle)?.seatOf("bravo")).toEqual({ player: g.player, side: g.side });
  expect(r.service.battle(a.battle)?.seatOf("stranger")).toBeUndefined();
  for (const member of [r.alpha, r.bravo]) {
    for (const view of member.records
      .filter((x) => x.kind === "state")
      .map((x) => readJson<RoomView>(x))) {
      if (view.phase.kind === "battle")
        expect(view.phase.ticket.player).toBe(member === r.alpha ? a.player : g.player);
    }
  }
  expect(JSON.stringify([r.alpha.records, r.bravo.records])).not.toMatch(/alpha|bravo|stranger/);
  expect(a.callsigns).toEqual({ "blue-1": "SCOUT", "red-1": "WING" });
});

test("one presence grace hands a dropped army to AI; returning reclaims it and replay agrees at every tick", async () => {
  const r = await room(),
    b = await r.start();
  const a = ticket(r.alpha.view()),
    g = ticket(r.bravo.view());
  const alpha = seat(b, a),
    bravo = seat(b, g),
    seen = alpha.read();
  await b.run(3);
  await buy(alpha);
  await alpha.order({ kind: "ready" });
  await bravo.order({ kind: "ready" });
  const unit = await firstUnit(b, seen);
  const before = (await orders(b.host, a.player)).length;
  r.alpha.close();
  await r.clock.advance(PRESENCE_GRACE_MS - 1);
  expect((await orders(b.host, a.player)).length).toBe(before);
  await r.clock.advance(1);
  expect((await alpha.order({ kind: "ready" })).error?.reason).toBe("ai_controlled");
  const commandsArmy = (entries: Awaited<ReturnType<typeof orders>>) =>
    entries.slice(before).some(([, c]) => "units" in c.order && c.order.units.includes(unit));
  for (let i = 0; i < 900 && !commandsArmy(await orders(b.host, a.player)); i++) await b.run(1);
  const aiOrders = await orders(b.host, a.player);
  expect(aiOrders.length).toBeGreaterThan(before);
  expect(commandsArmy(aiOrders)).toBe(true);
  const returned = r.open("alpha");
  expect(ticket(returned.view())).toEqual(a);
  expect(r.service.roomStream("stranger", r.code, "other", null, () => true)).toEqual({
    refused: "started",
  });
  expect((await alpha.order({ kind: "stop", units: [unit] })).error).toBeNull();
  const reclaimed = await orders(b.host, a.player);
  expect(reclaimed).toHaveLength(aiOrders.length + 1);
  expect(reclaimed.at(-1)![1].order).toEqual({ kind: "stop", units: [unit] });
  await b.run(30);
  const first = seen.seen[0].tick;
  expect(seen.seen.map((frame) => frame.tick)).toEqual(
    Array.from({ length: tickOf(seen) - first + 1 }, (_, index) => first + index),
  );
  const replay = await b.host.replay();
  expect(JSON.parse(replay).seat_events).toBeUndefined();
  const direct = await yardstick(a.setup, replay);
  direct.runTo(tickOf(seen));
  expect(direct.failure()).toBeNull();
  expect([...digests(seen)].filter(([tick, digest]) => direct.seen.get(tick) !== digest)).toEqual(
    [],
  );
}, 120_000);

test("an AI-run dropped seat does not hold preparation open, and controller changes preserve ready flags", async () => {
  const r = await room(),
    b = await r.start();
  const a = seat(b, ticket(r.alpha.view())),
    g = seat(b, ticket(r.bravo.view()));
  const seen = a.read(),
    other = g.read();
  await b.run(3);
  expect(seen.latest!.skirmish!.phase).toBe("preparation");
  expect(
    [...seen.latest!.skirmish!.players, ...other.latest!.skirmish!.players].map((p) => p.ready),
  ).toEqual([false, false]);
  await g.order({ kind: "ready" });
  b.host.setController("blue-1", "active_ai");
  b.host.setController("blue-1", "human");
  await b.run(3);
  expect(
    [...seen.latest!.skirmish!.players, ...other.latest!.skirmish!.players].map((p) => p.ready),
  ).toEqual([false, true]);
  r.alpha.close();
  await r.clock.advance(PRESENCE_GRACE_MS);
  await b.run(3);
  expect(seen.latest!.skirmish!.phase).toBe("active");
  const ready = [...seen.latest!.skirmish!.players, ...other.latest!.skirmish!.players].map(
    (p) => p.ready,
  );
  r.open("alpha");
  await b.run(3);
  expect(
    [...seen.latest!.skirmish!.players, ...other.latest!.skirmish!.players].map((p) => p.ready),
  ).toEqual(ready);
}, 120_000);

test("a crashed battle ends every room stream, deletes the battle and frees its slot", async () => {
  const r = await room(),
    b = await r.start();
  const id = ticket(r.alpha.view()).battle;
  b.host.setController("blue-1", "no such controller" as ControllerKind);
  await flush();
  expect(b.ended.map((e) => e.kind)).toEqual(["crashed"]);
  for (const member of [r.alpha, r.bravo])
    expect(readJson(member.records.at(-1)!)).toEqual({ reason: "battle_failed" });
  expect(r.service.battle(id)).toBeUndefined();
  expect(r.service.roomStream("alpha", r.code, "local", null, () => true)).toEqual({
    refused: "unknown",
  });
  expect(r.service.create("stranger", "public")).toEqual({ code: expect.any(String) });
});

test("a minute without connected humans deletes the room and disposes its battle", async () => {
  const r = await room(),
    b = await r.start();
  const a = ticket(r.alpha.view());
  const watching = seat(b, a).read();
  await b.run(3);
  r.alpha.close();
  r.bravo.close();
  await r.clock.advance(EMPTY_ROOM_TIMEOUT_MS - 1);
  expect(r.service.create("stranger", "public")).toEqual({ refused: "full" });
  await r.clock.advance(1);
  expect(r.service.battle(a.battle)).toBeUndefined();
  await expect(b.host.replay()).rejects.toThrow(/ended/);
  expect(watching.end).toBe("disposed");
  expect(r.service.create("stranger", "public")).toEqual({ code: expect.any(String) });
}, 120_000);

test("normal skirmish victory publishes the final result to both seats, ends each room stream and frees the battle slot", async () => {
  const r = await room();
  r.act({ kind: "settings", patch: { map: { type: "open", size: "small", seed: "1" } } });
  const b = await r.start();
  const a = ticket(r.alpha.view()),
    g = ticket(r.bravo.view());
  const alpha = seat(b, a),
    bravo = seat(b, g);
  const [seen, other] = [alpha.read(), bravo.read()];
  await b.run(3);
  const catalog = (await nodeCatalogSet("game", join(import.meta.dirname, "../../.."))).units;
  let bought = false;
  for (const card of catalog.view.cards.filter(
    (c) => c.disabled_reason === null && "wheeled" in catalog.type(c.id).mobility,
  )) {
    if (
      (await alpha.order({ kind: "confirm_purchase", variant: card.id, destination: [0, 0] }))
        .error === null
    ) {
      bought = true;
      break;
    }
  }
  expect(bought).toBe(true);
  await alpha.order({ kind: "ready" });
  await bravo.order({ kind: "ready" });
  const unit = await firstUnit(b, seen);
  // Red is a connected human who buys nothing. Blue takes every objective;
  // no shortened rule document, special victory endpoint or simulation override.
  const objectives = seen.latest!.skirmish!.objectives;
  for (const objective of objectives) {
    expect(
      (
        await alpha.order({
          kind: "move",
          units: [unit],
          gesture: 1,
          goal: objective.center,
          route: "fastest",
        })
      ).error,
    ).toBeNull();
    for (
      let ticks = 0;
      ticks < 30000 &&
      !b.ended.length &&
      seen.latest!.skirmish!.objectives[objectives.indexOf(objective)].owner !== "blue";
      ticks++
    )
      await b.run(1);
    expect(
      seen.latest!.skirmish!.objectives[objectives.indexOf(objective)].owner,
      JSON.stringify({
        own: seen.latest!.own.map((u) => ({ kind: u.kind, position: u.position, goal: u.goal })),
        tick: tickOf(seen),
        objective,
      }),
    ).toBe("blue");
  }
  await b.run(3);
  expect(b.ended).toEqual([{ kind: "finished" }]);
  for (const reading of [seen, other]) {
    expect(reading.latest!.skirmish).toMatchObject({
      phase: "finished",
      result: "blue",
      report: expect.any(Object),
    });
    expect(reading.end).toBe("finished");
  }
  for (const member of [r.alpha, r.bravo])
    expect(readJson(member.records.at(-1)!)).toEqual({ reason: "battle_finished" });
  expect(r.service.roomStream("alpha", r.code, "local", null, () => true)).toEqual({
    refused: "unknown",
  });
  expect(r.service.battle(a.battle)).toBeUndefined();
  expect(r.service.create("stranger", "public")).toEqual({ code: expect.any(String) });
}, 180_000);

test("a room whose map fails preparation closes honestly and releases its capacity", async () => {
  const r = await room();
  // The injected host cannot prepare its map. This exercises a real host's
  // readiness rejection and end notification, before any ticket exists.
  let failed!: BattleHost;
  const service = createRoomService({
    online: createOnlineState({
      capacity: 1,
      clock: r.clock,
      callsign: () => "SCOUT",
      releaseCallsign: () => {},
    }),
    random: () => 0.1,
    createBattle: (candidates) =>
      (failed = createBattleHost(
        {
          candidates: candidates.map((setup) => ({
            ...setup,
            map: { ...setup.map, type: "unmakeable test map" as never },
          })),
        },
        { clock: r.clock, generator },
        data,
      )),
  });
  cleanups.push(service.dispose);
  const made = service.create("alpha", "public");
  if ("refused" in made) throw new Error(made.refused);
  const records: StreamRecord[] = [];
  const stream = service.roomStream("alpha", made.code, "local", null, (r) => {
    records.push(r);
    return true;
  });
  if ("refused" in stream) throw new Error(stream.refused);
  cleanups.push(stream.close);
  service.act("alpha", made.code, { kind: "set_seat", seat: "seat-2", to: "ai" });
  service.act("alpha", made.code, { kind: "ready", ready: true });
  expect(service.act("alpha", made.code, { kind: "start" })).toEqual({ ok: true });
  await expect(failed.ready).rejects.toThrow();
  await flush();
  expect(records.at(-1)?.kind).toBe("end");
  expect(readJson(records.at(-1)!)).toEqual({ reason: "battle_failed" });
  expect(service.create("bravo", "public")).toEqual({ code: expect.any(String) });
});

test("localhost room streams deliver each ticket and route both seats' streams and orders through /battles, refusing a stranger", async () => {
  const { createLobbyHttp } = await import("../../../apps/game-server/src/http/index");
  const { httpRecords } = await import("../../../packages/multiplayer/src/stream/http");
  const { ScenePresentation } = await import("../../src/battle/sim/presentation");
  const clock = new PageClock();
  let host!: BattleHost;
  const build = { commit: "room-battle-test", sim: "room-battle-test" };
  const server = createLobbyHttp(build, {
    clock,
    random: Math.random,
    clientAddressHeader: null,
    machines: inProcessBattleMachines({
      capacity: 1,
      now: clock.now,
      createBattle: (spec) => (host = createBattleHost(spec, { clock, generator }, data)),
      ordersPerSecond: JSON.parse(data.rules).commands.orders_per_s,
    }),
    mapAttempts: 1,
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  const ids = ["1".repeat(32), "2".repeat(32), "3".repeat(32)];
  const lifetime = new AbortController();
  const recordings: StreamRecord[][] = [];
  const readers: Promise<void>[] = [];
  const replies: unknown[] = [];
  let battleAddress = address;
  const target = (path: string) => (path.startsWith("/battles/") ? battleAddress : address);
  async function post(path: string, value: unknown, who = ids[0]) {
    const response = await fetch(`${target(path)}${path}`, {
      method: "POST",
      headers: { "X-Player": who, "Content-Type": "application/json" },
      body: JSON.stringify(value),
    });
    const reply = await response.json();
    replies.push(reply);
    return reply;
  }
  function stream(path: string, who: string) {
    const connection = new AbortController();
    const signal = AbortSignal.any([lifetime.signal, connection.signal]);
    const records: StreamRecord[] = [];
    recordings.push(records);
    readers.push(
      (async () => {
        try {
          for await (const r of httpRecords(`${target(path)}${path}`, who, signal)) records.push(r);
        } catch (error) {
          if (!signal.aborted) throw error;
        }
      })(),
    );
    return Object.assign(records, { close: () => connection.abort() });
  }
  async function until(predicate: () => boolean) {
    for (let i = 0; i < 1000 && !predicate(); i++) {
      await clock.advance(10);
    }
    expect(predicate()).toBe(true);
  }
  try {
    for (const [i, id] of ids.entries())
      expect(await post("/hello", { build, callsign: `COMMANDER-${i}` }, id)).toEqual({
        callsign: `COMMANDER-${i}`,
      });
    const made = await post("/rooms", { visibility: "public" });
    const path = `/rooms/${made.code}`;
    const blue = stream(path, ids[0]),
      red = stream(path, ids[1]);
    await until(() => red.some((r) => r.kind === "state"));
    expect(await post(`${path}/act`, { kind: "settings", patch: { map: { seed: "1" } } })).toEqual({
      ok: true,
    });
    for (const id of ids.slice(0, 2))
      expect(await post(`${path}/act`, { kind: "ready", ready: true }, id)).toEqual({ ok: true });
    expect(await post(`${path}/act`, { kind: "start" })).toEqual({ ok: true });
    await until(() => !!host);
    const ready = await host.ready;
    const view = (records: StreamRecord[]) =>
      records
        .filter((r) => r.kind === "state")
        .map((r) => readJson<RoomView>(r))
        .at(-1)!;
    await until(() => [blue, red].every((records) => view(records)?.phase.kind === "battle"));
    const tickets = [ticket(view(blue)), ticket(view(red))];
    battleAddress = tickets[0].address ?? address;
    expect(tickets.map((t) => [t.side, t.player])).toEqual([
      ["blue", "blue-1"],
      ["red", "red-1"],
    ]);
    expect(tickets[0].battle).toBe(tickets[1].battle);
    expect(tickets.map((t) => t.mapIdentity)).toEqual([ready.mapIdentity, ready.mapIdentity]);
    const battles = tickets.map((t, i) => stream(`/battles/${t.battle}/stream`, ids[i]));
    await until(() => battles.every((records) => records.some((r) => r.kind === "publication")));
    // The room pages have gone away. Only the battle sockets keep these
    // browsers present; neither seat may become AI after the same grace.
    blue.close();
    red.close();
    await flush();
    await clock.advance(PRESENCE_GRACE_MS + 1);
    for (const [i, records] of battles.entries()) {
      const decoder = new ScenePresentation(ready.layout);
      const frames = records
        .filter((r) => r.kind === "publication")
        .map((r) => decoder.decode(new Float32Array(r.payload.slice(24).buffer))!);
      expect(frames.length).toBeGreaterThan(0);
      expect(new Set(frames.map((f) => f.groundPatch.side))).toEqual(new Set([tickets[i].side]));
      expect(frames.at(-1)!.skirmish!.players.map((p) => p.id)).toEqual([tickets[i].player]);
      const response = await post(
        `/battles/${tickets[i].battle}/up`,
        { seq: 1, items: [{ id: 1, order: { kind: "ready" }, queued: false }] },
        ids[i],
      );
      expect(response).toMatchObject({ seq: 1, results: [{ id: 1, ack: { error: null } }] });
    }
    await clock.advance(100);
    const replay = JSON.parse(await host.replay()) as {
      accepted: [number, { player: string; order: { kind: string } }][];
    };
    for (const t of tickets)
      expect(
        replay.accepted.some(([, c]) => c.player === t.player && c.order.kind === "ready"),
      ).toBe(true);
    for (const suffix of ["stream", "up"]) {
      const response = await fetch(`${battleAddress}/battles/${tickets[0].battle}/${suffix}`, {
        method: suffix === "stream" ? "GET" : "POST",
        headers: { "X-Player": ids[2], "Content-Type": "application/json" },
        body: suffix === "up" ? JSON.stringify({ seq: 2, items: [] }) : undefined,
      });
      expect(response.status).toBe(403);
      expect(await response.json()).toMatchObject({ refused: "not_seated" });
    }
    expect(await post("/hello", { build, callsign: "COMMANDER-2" }, ids[2])).toEqual({
      callsign: "COMMANDER-2",
    });
    const joined = await fetch(`${target(path)}${path}`, { headers: { "X-Player": ids[2] } });
    expect(await joined.json()).toEqual({ refused: "started" });
    // An order the battle cannot read is refused to its sender, quotes
    // nothing back (here it carried the sender's own credential), and ends
    // nobody's battle: the other seat's next order is still applied.
    expect(
      await post(`/battles/${tickets[0].battle}/up`, {
        seq: 2,
        items: [{ id: 2, order: { kind: ids[0] }, queued: false }],
      }),
    ).toEqual({ seq: 2, refused: "invalid" });
    expect(
      await post(
        `/battles/${tickets[1].battle}/up`,
        { seq: 2, items: [{ id: 2, order: { kind: "ready" }, queued: false }] },
        ids[1],
      ),
    ).toMatchObject({ seq: 2, results: [{ id: 2, ack: { error: null } }] });
    expect(battles.some((records) => records.some((record) => record.kind === "end"))).toBe(false);
    const outgoing = JSON.stringify([
      recordings.map((records) =>
        records.map((record) => new TextDecoder().decode(record.payload)),
      ),
      replies,
    ]);
    for (const id of ids)
      expect(outgoing.includes(id), "a credential appeared in a response or stream").toBe(false);
  } finally {
    lifetime.abort();
    await Promise.allSettled(readers);
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
}, 120_000);
