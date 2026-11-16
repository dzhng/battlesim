import { inProcessBattleMachines } from "../../../apps/game-server/src/machines/inProcess";
// @vitest-environment node
import { afterEach, beforeAll, expect, test } from "vitest";
import { createFrontlineService } from "../../../apps/game-server/src/frontline";
import { createOnlineState } from "../../../apps/game-server/src/online";
import { createBattleHost } from "../../../apps/game-server/src/battleHost/index";
import type { BattleHost } from "../../../apps/game-server/src/battleHost/index";
import { PageClock } from "./pageClock";
import { loadBattleSupport, data, generator, flush, seat, recordHost } from "./support/battle";
import type { BattleSetup, BattleTicket, Briefing } from "../../../packages/multiplayer/src/wire";
import { MAP_ATTEMPTS } from "../../../apps/game-server/src/serverData";

beforeAll(loadBattleSupport, 60_000);
const cleanups: (() => void)[] = [];
afterEach(() => {
  for (const close of cleanups.splice(0).reverse()) close();
});

/** The same game on a map the generator will not make. */
const unmakeable = (setup: BattleSetup): BattleSetup => ({
  ...setup,
  map: { ...setup.map, type: "no such map type" as never },
});

/** `refusedMaps`: how many of the first drawn maps the generator refuses. */
async function service(capacity = 2, admissionMs = 0, refusedMaps = 0) {
  const clock = new PageClock();
  const battleClock = new PageClock();
  const hosts: BattleHost[] = [];
  const failures: string[] = [];
  const records: ReturnType<typeof recordHost>[] = [];
  const forgotten = new Set<string>();
  const online = createOnlineState({
    capacity,
    clock,
    callsign: (who) => (forgotten.has(who) ? undefined : "SCOUT"),
    releaseCallsign: (who) => {
      forgotten.add(who);
    },
  });
  online.presence.track("alpha");
  let draw = 0;
  let refusals = refusedMaps;
  const frontline = createFrontlineService({
    online,
    mapAttempts: MAP_ATTEMPTS,
    random: () => [0, 0, 0.5, 0, 1 / 2 ** 32, 0, 0][draw++ % 7],
    createBattle(candidates) {
      const host = createBattleHost(
        { candidates: candidates.map((setup) => (refusals-- > 0 ? unmakeable(setup) : setup)) },
        { clock: battleClock, generator },
        data,
      );
      clock.suspend(admissionMs);
      void clock.advance(0);
      hosts.push(host);
      host.onEnd((end) => {
        if (end.kind === "crashed") failures.push(end.message);
      });
      records.push(recordHost(host, candidates[0].seats[0]));
      return host;
    },
  });
  cleanups.push(online.dispose, frontline.dispose);
  const join = async (who: string) => {
    const result = await frontline.join(who);
    if ("refused" in result) throw new Error(failures.join("; ") || result.refused);
    return result;
  };
  const a = await join("alpha");
  const host = hosts.at(-1)!;
  const ready = await host.ready;
  const ended: Parameters<Parameters<BattleHost["onEnd"]>[0]>[0][] = [];
  host.onEnd((end) => ended.push(end));
  const run = async (ticks: number) => {
    for (let i = 0; i < ticks; i++) await battleClock.advance(1000 / ready.tickHz);
  };
  const b = { host, ready, ended, run };
  const deploy = async (who: string, briefing: Briefing): Promise<BattleTicket> => {
    let done = false;
    const pending = frontline.deploy(who, briefing.battle).finally(() => {
      done = true;
    });
    for (let i = 0; i < 10 && !done; i++) {
      await flush();
      await run(1);
    }
    const result = await pending;
    if ("refused" in result) throw new Error(result.refused);
    return result;
  };
  const present = (who: string) => {
    const close = online.presence.open(who);
    cleanups.push(close);
    return close;
  };
  const closeAlpha = present("alpha");
  const ticket = await deploy("alpha", a);
  const alpha = seat(b, ticket),
    seen = alpha.read();
  for (let i = 0; i < ready.tickHz * 3 && !seen.latest; i++) await run(1);
  if (!seen.latest) throw new Error("the initial reader never received its snapshot");
  return {
    clock,
    battleClock,
    hosts,
    records,
    online,
    frontline,
    join,
    a,
    ticket,
    alpha,
    seen,
    present,
    closeAlpha,
    nextMap: () => {
      draw = 6;
    },
    deploy,
    b,
    run,
  };
}

test("Play draws another map when the generator refuses one, and says so honestly when it refuses them all", async () => {
  // One refusal short of the game's limit: the last draw is admitted.
  const s = await service(1, 0, MAP_ATTEMPTS - 1);
  expect((await s.b.host.ready).candidate + 1).toBe(MAP_ATTEMPTS);
  expect(s.seen.latest).not.toBeNull();
  expect(s.hosts).toHaveLength(1);

  // Every draw refused: not "full", and the battle slot is free again.
  const online = createOnlineState({
    capacity: 1,
    clock: new PageClock(),
    callsign: () => "SCOUT",
    releaseCallsign: () => {},
  });
  const made: BattleHost[] = [];
  const frontline = createFrontlineService({
    online,
    mapAttempts: MAP_ATTEMPTS,
    random: () => 0.25,
    createBattle(candidates) {
      made.push(
        createBattleHost(
          { candidates: candidates.map(unmakeable) },
          { clock: new PageClock(), generator },
          data,
        ),
      );
      return made.at(-1)!;
    },
  });
  cleanups.push(online.dispose, frontline.dispose);
  online.presence.track("alpha");
  expect(await frontline.join("alpha")).toEqual({ refused: "unavailable" });
  expect(made).toHaveLength(1);
  await expect(made[0].ready).rejects.toThrow();
  expect(online.slotFree()).toBe(true);
  // Like the adjacent large-battle cases, allow preparation under parallel CPU
  // load. This is a hang watchdog; every retry and capacity assertion stays exact.
}, 120_000);

test("Play reserves a battle and Deploy takes its initial human seat without a seat event", async () => {
  const s = await service();
  expect(s.ticket.battle).toBe(s.a.battle);
  expect(s.ticket.mapIdentity).toBe(s.a.mapIdentity);
  expect(s.frontline.battle(s.a.battle)?.seatOf("alpha")).toEqual({
    side: s.ticket.side,
    player: s.ticket.player,
  });
  expect(JSON.parse(await s.b.host.replay()).seat_events).toBeUndefined();
  expect(s.seen.latest!.skirmish!.players.find((p) => p.id === s.ticket.player)!.controller).toBe(
    "human",
  );
  expect(await s.frontline.deploy("alpha", s.a.battle)).toEqual(s.ticket);
  expect(JSON.stringify([s.a, s.ticket])).not.toContain("alpha");
});

test("a joiner's displaced units stay on their side under an AI that orders them; Deploy publishes only the completed fresh seat", async () => {
  const s = await service();
  expect((await s.alpha.order({ kind: "ready" })).error).toBeNull();
  const briefing = await s.join("bravo");
  s.present("bravo");
  const index = s.ticket.setup.seats.findIndex(
    (p) => p.side === briefing.side && p.player.endsWith(`-${briefing.seat}`),
  );
  const id = s.ticket.setup.seats[index].player;
  const ai = seat(s.b, { side: briefing.side, player: id });
  const watching = ai.read();
  for (let i = 0; i < 1500 && !(await s.b.host.referee()).seats[index].living; i++) await s.run(1);
  const old = watching.latest!.own.filter((u) => u.owner === index).map((u) => u.id);
  expect(old.length).toBeGreaterThan(0);
  // Clear the old AI's standing orders before transferring. A later goal
  // must then have been applied by the receiving AI, not inherited motion.
  s.b.host.setController(id, "human");
  expect((await ai.order({ kind: "stop", units: old })).error).toBeNull();
  await s.run(1);
  expect(watching.latest!.own.filter((u) => old.includes(u.id)).every((u) => u.goal === null)).toBe(
    true,
  );
  s.b.host.setController(id, "active_ai");
  const before = JSON.parse(await s.b.host.replay()).accepted.length;
  const ticket = await s.deploy("bravo", briefing);
  const view = await s.b.host.referee();
  expect(view.seats[index]).toMatchObject({
    controller: "human",
    living: 0,
    credits: view.opening_credits,
  });
  const moved = watching.latest!.own.filter((u) => old.includes(u.id));
  expect(moved.map((u) => u.id).sort()).toEqual(old.sort());
  expect(moved.every((u) => u.owner !== index)).toBe(true);
  const receiver = moved[0].owner!;
  expect(view.seats[receiver]).toMatchObject({ side: ticket.side, controller: "active_ai" });
  const commandsArmy = async () =>
    (
      JSON.parse(await s.b.host.replay()).accepted as [
        number,
        { player: string; order: { units?: number[] } },
      ][]
    )
      .slice(before)
      .some(
        ([, c]) =>
          c.player === view.seats[receiver].player &&
          c.order.units?.some((unit) => old.includes(unit)),
      );
  for (let i = 0; i < 900 && !(await commandsArmy()); i++) await s.run(1);
  expect(await commandsArmy()).toBe(true);
  expect(watching.latest!.own.some((u) => old.includes(u.id) && u.goal !== null)).toBe(true);
  const replay = JSON.parse(await s.b.host.replay());
  expect(replay.seat_events).toEqual([
    { kind: "hand_over", tick: expect.any(Number), player: ticket.player },
    { kind: "reset", tick: replay.seat_events[0].tick, player: ticket.player, opening: true },
  ]);
  const boundary = watching.seen.find((f) => f.tick === replay.seat_events[0].tick)!;
  expect(boundary.players.find((p) => p.id === ticket.player)).toMatchObject({
    controller: "human",
    unitCount: 0,
    credits: Math.fround(view.opening_credits),
  });
  expect((await ai.order({ kind: "ready" })).error).toBeNull();
}, 120_000);

test("leaving and returning while held restores the same army and records only controllers", async () => {
  const s = await service();
  s.present("bravo");
  const other = await s.deploy("bravo", await s.join("bravo"));
  expect(other.side).not.toBe(s.ticket.side);
  const { buy, firstUnit } = await import("./support/battle");
  await buy(s.alpha);
  await s.alpha.order({ kind: "ready" });
  const unit = await firstUnit(s.b, s.seen);
  const before = JSON.parse(await s.b.host.replay()).seat_events;
  s.frontline.leave("alpha", s.a.battle);
  await s.run(1);
  expect((await s.alpha.order({ kind: "ready" })).error?.reason).toBe("ai_controlled");
  const { GONE_TIMEOUT_MS } = await import("../../../packages/multiplayer/src/frontline");
  await s.clock.advance(GONE_TIMEOUT_MS - 1);
  const returned = await s.join("alpha");
  const ticket = await s.deploy("alpha", returned);
  expect([ticket.player, ticket.side, ticket.battle]).toEqual([
    s.ticket.player,
    s.ticket.side,
    s.ticket.battle,
  ]);
  expect((await s.alpha.order({ kind: "stop", units: [unit] })).error).toBeNull();
  await s.run(1);
  expect(JSON.parse(await s.b.host.replay()).seat_events).toEqual(before);
  expect(
    (await s.b.host.referee()).seats.find((p) => p.player === ticket.player)!.living,
  ).toBeGreaterThan(0);
}, 120_000);

test("a player returning after the hold joins afresh without another opening grant", async () => {
  const s = await service();
  s.present("bravo");
  await s.deploy("bravo", await s.join("bravo"));
  s.frontline.leave("alpha", s.a.battle);
  const { GONE_TIMEOUT_MS } = await import("../../../packages/multiplayer/src/frontline");
  await s.clock.advance(GONE_TIMEOUT_MS);
  const returned = await s.join("alpha");
  const ticket = await s.deploy("alpha", returned);
  const view = await s.b.host.referee();
  expect(view.seats.find((p) => p.player === ticket.player)).toMatchObject({
    controller: "human",
    living: 0,
    credits: 0,
  });
  const replay = JSON.parse(await s.b.host.replay());
  expect(replay.seat_events.at(-1)).toMatchObject({
    kind: "reset",
    player: ticket.player,
    opening: false,
  });
}, 120_000);

test("idle ends only that player's stream with a reason, its AI commands the army and the hold then opens to a joiner", async () => {
  const s = await service();
  s.present("bravo");
  const bt = await s.deploy("bravo", await s.join("bravo"));
  const bravo = seat(s.b, bt);
  const { buy, firstUnit } = await import("./support/battle");
  await buy(s.alpha);
  await s.alpha.order({ kind: "ready" });
  const unit = await firstUnit(s.b, s.seen);
  const before = JSON.parse(await s.b.host.replay()).accepted.length;
  const { IDLE_TIMEOUT_MS, GONE_TIMEOUT_MS } =
    await import("../../../packages/multiplayer/src/frontline");
  await s.clock.advance(IDLE_TIMEOUT_MS / 2);
  await bravo.order({ kind: "ready" });
  await s.run(1);
  await s.clock.advance(IDLE_TIMEOUT_MS / 2);
  await s.run(1);
  expect(s.seen.end).toBe("idle");
  expect((await s.alpha.order({ kind: "ready" })).error?.reason).toBe("ai_controlled");
  const ordersArmy = async () =>
    (
      JSON.parse(await s.b.host.replay()).accepted as [
        number,
        { player: string; order: { units?: number[] } },
      ][]
    )
      .slice(before)
      .some(([, c]) => c.player === s.ticket.player && c.order.units?.includes(unit));
  for (let i = 0; i < 900 && !(await ordersArmy()); i++) await s.run(1);
  expect(await ordersArmy()).toBe(true);
  await s.clock.advance(GONE_TIMEOUT_MS);
  const joiner = await s.join("charlie");
  expect([joiner.battle, joiner.side, joiner.seat]).toEqual([s.a.battle, s.a.side, s.a.seat]);
}, 120_000);

test("an undeployed reservation lapses and late Deploy is expired while its seat is offered again", async () => {
  const s = await service();
  const reserved = await s.join("bravo");
  expect(await s.join("bravo")).toEqual(reserved);
  expect(s.frontline.battle(reserved.battle)?.seatOf("bravo")).toBeUndefined();
  const { GONE_TIMEOUT_MS } = await import("../../../packages/multiplayer/src/frontline");
  await s.clock.advance(GONE_TIMEOUT_MS);
  expect(await s.frontline.deploy("bravo", reserved.battle)).toEqual({ refused: "expired" });
  const next = await s.join("charlie");
  expect([next.battle, next.side, next.seat]).toEqual([
    reserved.battle,
    reserved.side,
    reserved.seat,
  ]);
  s.present("charlie");
  const ticket = await s.deploy("charlie", next);
  expect((await s.b.host.referee()).seats.find((p) => p.player === ticket.player)!.controller).toBe(
    "human",
  );
}, 120_000);

test("a battle beyond its joining point refuses a newcomer, then its final observations end the game and release capacity", async () => {
  const { shortScoringFixture } = await import("./support/battle");
  const fixture = await shortScoringFixture();
  cleanups.push(() => fixture.mockRestore());
  const s = await service(1);
  const enemy = seat(s.b, s.ticket.setup.seats.find((p) => p.side !== s.ticket.side)!).read();
  const { buyFrom, firstUnit, cards } = await import("./support/battle");
  const { nodeCatalogSet } = await import("../../src/battle/catalog/node");
  const { join } = await import("node:path");
  const catalog = (await nodeCatalogSet("game", join(import.meta.dirname, "../../.."))).units;
  await buyFrom(
    s.alpha,
    cards.filter((id) => "foot" in catalog.type(id).mobility),
  );
  await s.alpha.order({ kind: "ready" });
  const unit = await firstUnit(s.b, s.seen);
  await s.alpha.order({ kind: "stop", units: [unit] });
  for (let i = 0; i < 1000 && Math.max(...(await s.b.host.referee()).scores) === 0; i++)
    await s.run(1);
  const scoring = await s.b.host.referee();
  expect(scoring.scores[0]).not.toBe(scoring.scores[1]);
  const second = await s.join("second");
  expect(second.side).toBe(scoring.scores[0] < scoring.scores[1] ? "blue" : "red");
  s.present("second");
  const secondTicket = await s.deploy("second", second);
  expect(
    (await s.b.host.referee()).seats.find((p) => p.player === secondTicket.player)!.controller,
  ).toBe("human");
  for (
    let i = 0;
    i < 3000 && Math.max(...(await s.b.host.referee()).scores) <= s.a.target * 0.9;
    i++
  )
    await s.run(1);
  const view = await s.b.host.referee();
  expect(
    Math.max(...view.scores),
    JSON.stringify({
      view,
      skirmish: s.seen.latest!.skirmish,
      own: s.seen.latest!.own.map((u) => ({ kind: u.kind, position: u.position, owner: u.owner })),
    }),
  ).toBeGreaterThan(view.target * 0.9);
  expect(view.phase).toBe("active");
  expect(await s.frontline.join("bravo")).toEqual({ refused: "full" });
  for (let i = 0; i < 3000 && !s.b.ended.length; i++) await s.run(1);
  expect(s.b.ended).toEqual([{ kind: "finished" }]);
  for (const seen of [s.seen, enemy]) {
    expect(seen.latest!.skirmish).toMatchObject({ phase: "finished", result: s.ticket.side });
    expect(seen.end).toBe("finished");
  }
  expect(s.frontline.battle(s.a.battle)).toBeUndefined();
  expect(s.online.slotFree()).toBe(true);
}, 120_000);

test("the last human leaving or dropping deletes the battle and releases its slot; a crash does likewise", async () => {
  const { PRESENCE_GRACE_MS } = await import("../../../apps/game-server/src/presence");
  for (const end of ["leave", "drop", "crash"] as const) {
    const s = await service(1);
    expect(s.online.slotFree()).toBe(false);
    if (end === "leave") s.frontline.leave("alpha", s.a.battle);
    if (end === "drop") {
      // End all streams in the shared presence, not the test's observation tap.
      s.closeAlpha();
      await s.clock.advance(PRESENCE_GRACE_MS - 1);
      expect(s.frontline.battle(s.a.battle)).toBeDefined();
      await s.clock.advance(1);
    }
    if (end === "crash") s.b.host.setController(s.ticket.player, "invalid" as never);
    await flush();
    expect(s.frontline.battle(s.a.battle)).toBeUndefined();
    expect(s.online.slotFree()).toBe(true);
    await expect(s.b.host.referee()).rejects.toThrow(/ended/);
  }
}, 120_000);

test("rooms and Frontline reserve the same battle slots", async () => {
  const s = await service(1);
  const { createRoomService } = await import("../../../apps/game-server/src/rooms");
  const room = createRoomService({
    online: s.online,
    random: () => 0.1,
    createBattle: (candidates) =>
      createBattleHost({ candidates }, { clock: s.battleClock, generator }, data),
  });
  cleanups.push(room.dispose);
  expect(room.create("bravo", "public")).toEqual({ refused: "full" });
  s.frontline.leave("alpha", s.a.battle);
  const made = room.create("bravo", "public");
  expect(made).toEqual({ code: expect.any(String) });
  expect(await s.frontline.join("alpha")).toEqual({ refused: "full" });
}, 120_000);

test("placement fills seats on the losing side in the fullest game and never offers its permanent AI", async () => {
  const s = await service(2);
  const held = [{ who: "alpha", side: s.ticket.side }];
  for (let i = 0; i < 7; i++) {
    const who = `commander-${i}`;
    s.present(who);
    const briefing = await s.join(who);
    expect(briefing.battle).toBe(s.a.battle);
    expect(briefing.seat).toBeLessThan(5);
    if (i === 0) expect(briefing.side).not.toBe(s.ticket.side);
    const ticket = await s.deploy(who, briefing);
    expect(
      (await s.b.host.referee()).seats.find((p) => p.player === ticket.player)!.controller,
    ).toBe("human");
    held.push({ who, side: ticket.side });
  }
  s.nextMap();
  const sparse = await s.join("sparse");
  expect(sparse.battle).not.toBe(s.a.battle);
  const ready = await s.hosts[1].ready;
  const deploy = s.frontline.deploy("sparse", sparse.battle);
  await s.battleClock.advance(1000 / ready.tickHz);
  await deploy;
  s.present("sparse");
  for (const side of ["blue", "red"] as const)
    s.frontline.leave(held.find((p) => p.side === side)!.who, s.a.battle);
  const { GONE_TIMEOUT_MS } = await import("../../../packages/multiplayer/src/frontline");
  await s.clock.advance(GONE_TIMEOUT_MS);
  const next = await s.join("next");
  expect(next.battle).toBe(s.a.battle);
  expect(next.seat).toBeLessThan(5);
}, 120_000);

test("the whole game's seat events and controller changes replay to the live digest at every tick", async () => {
  const s = await service();
  s.present("bravo");
  await s.deploy("bravo", await s.join("bravo"));
  const { buy, firstUnit, yardstick } = await import("./support/battle");
  await buy(s.alpha);
  await s.alpha.order({ kind: "ready" });
  await firstUnit(s.b, s.seen);
  s.frontline.leave("alpha", s.a.battle);
  await s.run(180);
  await s.deploy("alpha", await s.join("alpha"));
  await s.run(30);
  const replay = await s.b.host.replay();
  expect(JSON.parse(replay).seat_events.map((e: { kind: string }) => e.kind)).toEqual([
    "hand_over",
    "reset",
  ]);
  expect(JSON.parse(replay).controller_transitions.length).toBeGreaterThan(0);
  const direct = await yardstick(s.ticket.setup, replay);
  const frames = s.records[0].frames;
  const last = Math.max(...frames.keys());
  expect([...frames.keys()]).toEqual(Array.from({ length: last }, (_, i) => i + 1));
  direct.runTo(last);
  expect(direct.failure()).toBeNull();
  expect([...frames].filter(([tick, digest]) => direct.seen.get(tick) !== digest)).toEqual([]);
}, 120_000);

test("a remote connection refuses referee and seat-event requests before they reach its seat link", async () => {
  const { loopback } = await import("../../../packages/multiplayer/src/link");
  const { remoteConnection } = await import("../../src/battle/remote/connection");
  const { timerClock } = await import("./timerClock");
  const requests: import("../../src/battle/sim/protocol").SimRequest[] = [
    { type: "referee", id: 1 },
    { type: "hand_over", player: "blue-1" },
    { type: "reset_seat", id: 2, player: "blue-1", opening: true },
  ];
  for (const request of requests) {
    const pair = loopback({ side: "blue", player: "blue-1" });
    const received: unknown[] = [],
      failures: string[] = [];
    pair.port.onUp(async (batch) => {
      received.push(batch);
      return { seq: batch.seq, results: [] };
    });
    pair.port.onPreview(async (preview) => {
      received.push(preview);
      return { dropped: true };
    });
    const connection = remoteConnection(pair.link, timerClock())(
      (r) => received.push(r),
      (why) => failures.push(why),
    );
    connection.send(request);
    await flush();
    expect(received).toEqual([]);
    expect(failures).toEqual([`Request ${request.type} is unavailable online`]);
    connection.close();
  }
});

test("localhost hello, briefing, deploy, battle stream and order share seating; strangers and administrative HTTP requests are refused", async () => {
  const { createLobbyHttp } = await import("../../../apps/game-server/src/http/index");
  const { httpRecords } = await import("../../../packages/multiplayer/src/stream/http");
  const { readJson } = await import("../../../packages/multiplayer/src/stream/index");
  const { GONE_TIMEOUT_MS } = await import("../../../packages/multiplayer/src/frontline");
  const clock = new PageClock(),
    battleClock = new PageClock();
  const hosts: BattleHost[] = [];
  let draw = 0;
  const build = { commit: "frontline-test", sim: "frontline-test" };
  const server = createLobbyHttp(build, {
    clock,
    clientAddressHeader: null,
    random: () => [0, 0, 0.5, 0, 1 / 2 ** 32, 0, 0][draw++ % 7],
    mapAttempts: 1,
    machines: inProcessBattleMachines({
      capacity: 1,
      now: battleClock.now,
      ordersPerSecond: JSON.parse(data.rules).commands.orders_per_s,
      createBattle(spec) {
        const host = createBattleHost(spec, { clock: battleClock, generator }, data);
        hosts.push(host);
        return host;
      },
    }),
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  const ids = ["1".repeat(32), "2".repeat(32), "3".repeat(32)];
  const replies: unknown[] = [],
    records: import("../../../packages/multiplayer/src/wire").StreamRecord[] = [];
  const abort = new AbortController();
  let reading: Promise<void> = Promise.resolve();
  let battleAddress = address;
  const post = async (path: string, value: unknown = {}, who = ids[0]) => {
    const response = await fetch(
      `${path.startsWith("/battles/") ? battleAddress : address}${path}`,
      {
        method: "POST",
        headers: { "X-Player": who, "Content-Type": "application/json" },
        body: JSON.stringify(value),
      },
    );
    const result = await response.json();
    replies.push(result);
    return result;
  };
  try {
    for (const [i, who] of ids.entries())
      expect(await post("/hello", { build, callsign: `SCOUT-${i}` }, who)).toEqual({
        callsign: `SCOUT-${i}`,
      });
    const briefing = (await post("/frontline/join")) as Briefing;
    expect(briefing).toMatchObject({
      battle: expect.any(String),
      side: expect.any(String),
      seat: expect.any(Number),
      credits: expect.any(Number),
      scores: [0, 0],
      target: expect.any(Number),
      minutesIn: expect.any(Number),
      setup: { rules: "frontline" },
      mapIdentity: expect.any(String),
    });
    expect(briefing.roster.filter((r) => r.callsign !== null)).toEqual([
      { side: briefing.side, callsign: "SCOUT-0" },
    ]);
    const ticket = (await post(`/frontline/${briefing.battle}/deploy`)) as BattleTicket;
    battleAddress = ticket.address ?? address;
    expect(ticket.callsigns[ticket.player]).toBe("SCOUT-0");
    const host = hosts[0];
    const ready = await host.ready;
    reading = (async () => {
      try {
        for await (const r of httpRecords(
          `${battleAddress}/battles/${ticket.battle}/stream`,
          ids[0],
          abort.signal,
        ))
          records.push(r);
      } catch (e) {
        if (!abort.signal.aborted) throw e;
      }
    })();
    for (let i = 0; i < 1000 && !records.some((r) => r.kind === "publication"); i++) {
      await battleClock.advance(1000 / ready.tickHz);
    }
    expect(records.some((r) => r.kind === "publication")).toBe(true);
    expect(readJson(records.find((r) => r.kind === "open")!)).toMatchObject({
      layout: ready.layout,
      tickHz: ready.tickHz,
    });
    const order = await post(`/battles/${ticket.battle}/up`, {
      seq: 1,
      items: [{ id: 1, order: { kind: "ready" }, queued: false }],
    });
    expect(order).toMatchObject({ results: [{ id: 1, ack: { error: null } }] });
    await battleClock.advance(1000 / ready.tickHz);
    expect(
      JSON.parse(await host.replay()).accepted.some(
        ([, c]: [number, { player: string; order: { kind: string } }]) =>
          c.player === ticket.player && c.order.kind === "ready",
      ),
    ).toBe(true);
    for (const suffix of ["stream", "preview", "up"]) {
      const response = await fetch(`${battleAddress}/battles/${ticket.battle}/${suffix}`, {
        method: suffix === "stream" ? "GET" : "POST",
        headers: { "X-Player": ids[2], "Content-Type": "application/json" },
        body:
          suffix === "up"
            ? JSON.stringify({ seq: 2, items: [] })
            : suffix === "preview"
              ? JSON.stringify({ kind: "purchase", variant: "unknown", destination: [0, 0] })
              : undefined,
      });
      expect(response.status).toBe(403);
      expect(await response.json()).toMatchObject({ refused: "not_seated" });
    }
    const before = await host.replay();
    for (const type of ["hand_over", "reset_seat", "referee"]) {
      expect(
        await post(`/battles/${ticket.battle}/up`, { type, player: ticket.player, opening: true }),
      ).toEqual({ refused: "invalid" });
      expect(await post(`/battles/${ticket.battle}/${type}`, { player: ticket.player })).toEqual({
        refused: "unknown",
      });
      expect(
        await post(`/battles/${ticket.battle}/preview`, { kind: type, player: ticket.player }),
      ).toEqual({ refused: "invalid" });
    }
    expect(await host.replay()).toBe(before);
    const reservation = (await post("/frontline/join", {}, ids[1])) as Briefing;
    await clock.advance(GONE_TIMEOUT_MS);
    expect(await post(`/frontline/${reservation.battle}/deploy`, {}, ids[1])).toEqual({
      refused: "expired",
    });
    // An administrative verb hidden inside a valid order envelope is not an
    // order: it is refused, applies no seat event, and the battle goes on.
    expect(
      await post(`/battles/${ticket.battle}/up`, {
        seq: 2,
        items: [{ id: 2, order: { kind: "hand_over", player: ticket.player }, queued: false }],
      }),
    ).toEqual({ seq: 2, refused: "invalid" });
    expect(JSON.parse(await host.replay()).seat_events ?? []).toEqual([]);
    expect(
      await post(`/battles/${ticket.battle}/up`, {
        seq: 3,
        items: [{ id: 3, order: { kind: "ready" }, queued: false }],
      }),
    ).toMatchObject({ seq: 3, results: [{ id: 3 }] });
    const outgoing = JSON.stringify([
      replies,
      records.map((r) => new TextDecoder().decode(r.payload)),
    ]);
    for (const id of ids) expect(outgoing.includes(id)).toBe(false);
  } finally {
    abort.abort();
    await reading;
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
  }
}, 120_000);

test("presence uses one grace before AI takes a dropped army and a returning stream restores it without seat events", async () => {
  const s = await service();
  s.present("bravo");
  await s.deploy("bravo", await s.join("bravo"));
  const { buy, firstUnit } = await import("./support/battle");
  const { PRESENCE_GRACE_MS } = await import("../../../apps/game-server/src/presence");
  await buy(s.alpha);
  await s.alpha.order({ kind: "ready" });
  const unit = await firstUnit(s.b, s.seen);
  const events = JSON.parse(await s.b.host.replay()).seat_events;
  s.closeAlpha();
  await s.clock.advance(PRESENCE_GRACE_MS - 1);
  expect((await s.alpha.order({ kind: "ready" })).error).toBeNull();
  await s.clock.advance(1);
  expect((await s.alpha.order({ kind: "ready" })).error?.reason).toBe("ai_controlled");
  await s.run(1);
  expect(
    (await s.b.host.referee()).seats.find((p) => p.player === s.ticket.player)!.controller,
  ).toBe("active_ai");
  s.present("alpha");
  expect(s.frontline.battle(s.a.battle)?.seatOf("alpha")).toEqual({
    side: s.ticket.side,
    player: s.ticket.player,
  });
  expect((await s.alpha.order({ kind: "stop", units: [unit] })).error).toBeNull();
  await s.run(1);
  expect(JSON.parse(await s.b.host.replay()).seat_events).toEqual(events);
}, 120_000);

test("map preparation longer than the presence grace retains the admitting commander's callsign", async () => {
  const { PRESENCE_GRACE_MS } = await import("../../../apps/game-server/src/presence");
  const s = await service(1, PRESENCE_GRACE_MS + 1);
  expect(s.a.roster.find((p) => p.side === s.ticket.side && p.callsign !== null)?.callsign).toBe(
    "SCOUT",
  );
  expect(s.ticket.callsigns[s.ticket.player]).toBe("SCOUT");
});

test("a preview the battle cannot read is dropped, quotes nothing back, and leaves the battle running", async () => {
  const s = await service();
  const credential = "a".repeat(32);
  const ends: import("../../../apps/game-server/src/battleHost/index").BattleEnd[] = [];
  s.b.host.onEnd((end) => ends.push(end));
  expect(
    await s.alpha.link.preview({
      kind: "move",
      move: { units: [], goal: [0, 0], route: credential as never },
    }),
  ).toEqual({ dropped: true });
  const before = s.seen.seen.length;
  await s.run(5);
  expect(ends).toEqual([]);
  expect(s.seen.end).toBeNull();
  expect(s.seen.seen.length).toBe(before + 5);
}, 120_000);
