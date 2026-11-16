// @vitest-environment node
import type { PreviewQuery } from "../../../packages/multiplayer/src/wire";
import { afterEach, beforeAll, beforeEach, describe, expect, test } from "vitest";
import { type BattleEnd } from "../../../apps/game-server/src/battleHost/index";
import { loopback } from "../../../packages/multiplayer/src/link";
import type { ControllerKind } from "../../src/battle/prepare/protocol";
import type { Order } from "../../src/battle/sim/protocol";
import { PageClock } from "./pageClock";
import {
  hosts,
  battle,
  seat,
  buy,
  firstUnit,
  moveTo,
  yardstick,
  flush,
  tickOf,
  digests,
  testHost,
  useRemoteHosts,
  cards,
  duel,
  squad,
  loadBattleSupport,
  disposeBattles,
} from "./support/battle";
beforeAll(loadBattleSupport, 60_000);
afterEach(disposeBattles);

describe.each([false, true])("battle host (remote: %s)", (remote) => {
  beforeEach(() => useRemoteHosts(remote));
  afterEach(() => useRemoteHosts(false));

  test("candidates are tried in order and the admitted battle has the same digests and replay as that candidate alone", async () => {
    const refused = { ...duel, map: { ...duel.map, type: "no such map type" as never } };
    const later = { ...duel, battleSeed: duel.battleSeed + 1 };
    const retried = await battle(duel, [refused, refused, duel, later]);
    const alone = await battle(duel);
    expect(retried.ready.candidate).toBe(2);
    expect(retried.ready.mapIdentity).toBe(alone.ready.mapIdentity);
    const observed = seat(retried, { side: "blue", player: "blue-1" }).read();
    const control = seat(alone, { side: "blue", player: "blue-1" }).read();
    await retried.run(30);
    await alone.run(30);
    expect(observed.seen.length).toBeGreaterThan(0);
    const direct = await yardstick(duel);
    direct.runTo(Math.max(tickOf(observed), tickOf(control)));
    for (const reading of [observed, control])
      expect(
        [...digests(reading)].filter(([tick, digest]) => direct.seen.get(tick) !== digest),
      ).toEqual([]);
    expect(await retried.host.replay()).toBe(await alone.host.replay());
  }, 120_000);

  test("two seats' orders through the host make the battle the same orders make without it, and its replay makes it again", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const red = seat(b, { side: "red", player: "red-1" });
    const watching = blue.read();
    await b.run(3);
    await buy(blue);
    await buy(red);
    expect((await blue.order({ kind: "ready" })).error).toBeNull();
    expect((await red.order({ kind: "ready" })).error).toBeNull();
    const unit = await firstUnit(b, watching);
    expect((await blue.order(moveTo(watching, unit))).error).toBeNull();
    await b.run(300);
    const last = tickOf(watching);
    const replay = await b.host.replay();
    expect(watching.failure).toBeNull();

    // The same orders at the same ticks, with no host.
    const direct = await yardstick(duel);
    const orders = [...blue.sent, ...red.sent].sort((a, b) => a.tick - b.tick);
    for (const sent of orders) {
      direct.runTo(sent.tick);
      direct.order(sent.seat, sent.order);
    }
    direct.runTo(last);
    expect(direct.failure()).toBeNull();
    expect(direct.acks.filter((ack) => ack.error === null).length).toBe(5);
    expect(direct.seen.get(last)).toBe(digests(watching).get(last));
    const differ = (a: Map<number, string>, b: Map<number, string>) =>
      [...a].filter(([tick, digest]) => b.get(tick) !== digest).map(([tick]) => tick);
    // The page saw every tick from the one it joined on.
    const first = watching.seen[0].tick;
    expect(watching.seen.map((s) => s.tick)).toEqual(
      Array.from({ length: last - first + 1 }, (_, i) => first + i),
    );
    expect(differ(digests(watching), direct.seen)).toEqual([]);

    // And the host's recorded orders, replayed.
    const replayed = await yardstick(duel, replay);
    replayed.runTo(last);
    expect(replayed.failure()).toBeNull();
    expect(differ(digests(watching), replayed.seen)).toEqual([]);
  }, 120_000);

  test("each seat is sent its own side's records and never the other's", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const red = seat(b, { side: "red", player: "red-1" });
    const [blues, reds] = [blue.read(), red.read()];
    await b.run(3);
    await buy(blue);
    await buy(red);
    await blue.order({ kind: "ready" });
    await red.order({ kind: "ready" });
    await firstUnit(b, blues);
    await firstUnit(b, reds);
    expect([blues.failure, reds.failure]).toEqual([null, null]);
    expect(new Set(blues.seen.map((s) => s.side))).toEqual(new Set(["blue"]));
    expect(new Set(reds.seen.map((s) => s.side))).toEqual(new Set(["red"]));
    // Each side saw its own unit enter and not the other's: the two streams
    // are not one stream sent twice.
    expect(blues.latest!.own.length).toBeGreaterThan(0);
    expect(reds.latest!.own.length).toBeGreaterThan(0);
    expect(blues.latest!.own.map((unit) => unit.kind)).not.toEqual(
      expect.arrayContaining(reds.latest!.own.map((unit) => unit.kind)),
    );
    // Both were the same battle, tick for tick.
    expect([...digests(blues)]).toEqual([...digests(reds)].slice(0, blues.seen.length));
  }, 120_000);

  test("an order for a unit another player on the side owns is refused, and its owner's is applied", async () => {
    const b = await battle(squad);
    const owner = seat(b, { side: "blue", player: "blue-1" });
    const ally = seat(b, { side: "blue", player: "blue-2" });
    const watching = owner.read();
    await b.run(3);
    await buy(owner);
    await owner.order({ kind: "ready" });
    await ally.order({ kind: "ready" });
    const unit = await firstUnit(b, watching);
    const order = moveTo(watching, unit) as Extract<Order, { kind: "move" }>;
    const headedThere = () => {
      const goal = watching.latest!.own.find((own) => own.id === unit)!.goal;
      return goal !== null && Math.hypot(goal[0] - order.goal[0], goal[1] - order.goal[1]) < 1;
    };
    const refused = await ally.order(order);
    expect(refused.error!.reason).toMatch(/own|player/);
    await b.run(30);
    expect(headedThere()).toBe(false);
    const applied = await owner.order(order);
    expect(applied.error).toBeNull();
    await b.run(30);
    expect(headedThere()).toBe(true);
  }, 120_000);

  test("a seat the transport made up is not served", async () => {
    const b = await battle(duel);
    expect(() => b.host.attach(loopback({ side: "blue", player: "red-1" }).port)).toThrow();
    expect(() => b.host.attach(loopback({ side: "red", player: "blue-9" }).port)).toThrow();
    expect(() => b.host.setController("blue-9", "active_ai")).toThrow();
  });

  test("a reader that stops reading does not slow the battle, and resumes on a snapshot", async () => {
    const followed = await battle(duel);
    const stalled = await battle(duel);
    const steady = seat(followed, { side: "blue", player: "blue-1" }).read();
    const page = seat(stalled, { side: "blue", player: "blue-1" });
    const witness = seat(stalled, { side: "red", player: "red-1" }).read();
    const before = page.read();
    await Promise.all([followed.run(20), stalled.run(20)]);
    // The page stops reading: its connection is dropped and not reopened.
    page.cut();
    await flush();
    const stoppedAt = before.lastId!;
    // Long enough for the side's stream to have rolled past where it stopped.
    const away = 60 * followed.ready.tickHz;
    await Promise.all([followed.run(away), stalled.run(away)]);
    expect(tickOf(witness)).toBe(tickOf(steady));
    expect(digests(witness).get(tickOf(witness))).toBe(digests(steady).get(tickOf(steady)));

    // It comes back from where it stopped, with the decoder it had.
    const back = page.read(stoppedAt, before.decoder);
    await stalled.run(40);
    expect(back.failure).toBeNull();
    expect(back.opens[0].resume).toBe("keyframe");
    expect(back.seen[0].keyframe).toBe(true);
    expect(back.seen[0].epoch).toBeGreaterThan(before.seen.at(-1)!.epoch);
    expect(back.seen.length).toBeGreaterThan(20);
    expect(back.seen.slice(1).some((s) => s.keyframe)).toBe(false);
    expect(digests(back).get(tickOf(back))).toBe(digests(witness).get(tickOf(back)));
  }, 180_000);

  test("a snapshot sent for a late reader is taken in line by the readers already there", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const first = blue.read();
    await b.run(45);
    expect(first.seen.filter((s) => s.keyframe).map((s) => s.id)).toEqual([first.seen[0].id]);

    const late = seat(b, { side: "blue", player: "blue-1" }).read();
    await b.run(10);
    expect(late.opens[0].resume).toBe("keyframe");
    expect(late.seen[0].keyframe).toBe(true);
    const snapshot = late.seen[0].id;
    // The reader already there got the same record, in its place in the
    // stream, and went on decoding the records after it.
    expect(first.seen.filter((s) => s.keyframe).map((s) => s.id)).toEqual([
      first.seen[0].id,
      snapshot,
    ]);
    expect(first.seen.map((s) => s.id)).toEqual(first.seen.map((_, i) => first.seen[0].id + i));
    expect(first.failure).toBeNull();
    expect(late.failure).toBeNull();
    expect(first.seen.at(-1)).toEqual(late.seen.at(-1));

    // A third reader a moment later needs a snapshot of its own. The side is
    // resynced at most once a second, so it waits for that second and no
    // longer, and is not left with nothing.
    const third = seat(b, { side: "blue", player: "blue-1" }).read();
    await b.run(b.ready.tickHz - 15);
    expect(third.seen).toEqual([]);
    await b.run(10);
    expect(third.seen[0].keyframe).toBe(true);
    // A second of battle, give or take the tick the request landed in.
    expect(third.seen[0].tick - late.seen[0].tick).toBeGreaterThanOrEqual(b.ready.tickHz - 1);
    await b.run(10);
    expect([first.failure, late.failure, third.failure]).toEqual([null, null, null]);
    expect(third.seen.at(-1)).toEqual(first.seen.at(-1));
  }, 120_000);

  test("a seat given to its AI is ordered by the AI, and its player's next order after taking it back is applied", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const watching = blue.read();
    const ordersFor = async (player: string) =>
      (
        JSON.parse(await b.host.replay()) as { accepted: [number, { player?: string }][] }
      ).accepted.filter(([, command]) => command.player === player).length;
    await b.run(3);
    expect(await ordersFor("blue-1")).toBe(0);

    b.host.setController("blue-1", "active_ai");
    const refused = await blue.order({ kind: "ready" });
    expect(refused.error?.reason).toBe("ai_controlled");
    for (let i = 0; i < 3000 && !watching.latest?.own.length; i++) await b.run(1);
    // Nothing the port sent was recorded, so these are the AI's own.
    const fromAi = await ordersFor("blue-1");
    expect(fromAi).toBeGreaterThan(0);
    expect(watching.latest!.own.length).toBeGreaterThan(0);

    b.host.setController("blue-1", "human");
    const unit = watching.latest!.own[0].id;
    const applied = await blue.order({ kind: "stop", units: [unit] });
    expect(applied.error).toBeNull();
    expect(await ordersFor("blue-1")).toBe(fromAi + 1);
  }, 120_000);

  test("an order or preview the battle cannot read is refused to the seat that sent it, and the battle goes on for everyone", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const red = seat(b, { side: "red", player: "red-1" });
    const watching = red.read();
    await b.run(3);

    const unreadable = await blue.link.up({
      seq: 1,
      items: [{ id: 1, order: { kind: "no such order" } as never, queued: false }],
    });
    expect(unreadable).toEqual({ seq: 1, refused: "invalid" });
    expect(
      await blue.link.preview({
        kind: "move",
        move: { units: [0], goal: [1, 1], route: "no such route" } as never,
      }),
    ).toEqual({ dropped: true });

    const before = tickOf(watching);
    await b.run(10);
    expect(b.ended).toEqual([]);
    expect(watching.failure).toBeNull();
    expect(tickOf(watching)).toBe(before + 10);
    // Both seats, the sender's included, go on giving orders.
    expect((await red.order({ kind: "ready" })).error).toBeNull();
    expect((await blue.order({ kind: "ready" })).error).toBeNull();
  });

  test("a purchase preview's carrier reaches the battle", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    await b.run(3);
    const on: PreviewQuery = { kind: "purchase", variant: cards[0], destination: [0, 0] };
    const onFoot = await blue.link.preview(on);
    // A carrier the battle does not know is the battle's to refuse: the
    // answer differs from the same purchase with none.
    const aboard = await blue.link.preview({ ...on, carrier: "no such carrier" });
    expect("purchase" in onFoot && "purchase" in aboard).toBe(true);
    expect(aboard).not.toEqual(onFoot);
  });

  test("a battle whose simulation fails says it crashed, ends every stream and settles every request", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const red = seat(b, { side: "red", player: "red-1" });
    const [blues, reds] = [blue.read(), red.read()];
    await b.run(10);
    expect(b.ended).toEqual([]);

    // The simulation refuses a controller it does not know, mid-battle.
    await b.host.setController("blue-1", "no such controller" as ControllerKind).catch(() => {});
    await flush();
    expect(b.ended).toHaveLength(1);
    expect(b.ended[0].kind).toBe("crashed");
    const { message } = b.ended[0] as Extract<BattleEnd, { kind: "crashed" }>;
    // The server learns why; a player's stream is told only that it crashed.
    expect(message).not.toBe("");
    expect([blues.end, reds.end]).toEqual(["crashed", "crashed"]);
    expect(message).not.toBe(blues.end);
    expect([blues.failure, reds.failure]).toEqual([null, null]);

    const late: BattleEnd[] = [];
    b.host.onEnd((end) => late.push(end));
    expect(late).toEqual(b.ended);
    await expect(blue.order({ kind: "ready" })).rejects.toThrow(/ended/);
    await expect(
      blue.link.preview({ kind: "purchase", variant: cards[0], destination: [0, 0] }),
    ).rejects.toThrow(/ended/);
    await expect(b.host.replay()).rejects.toThrow(/ended/);
    // A reader that arrives after the end hears the end.
    const after = blue.read();
    await flush();
    expect(after.end).toBe("crashed");
    const ticked = tickOf(reds);
    await b.run(10);
    expect(tickOf(reds)).toBe(ticked);
  });

  test("a battle whose map cannot be made never becomes ready and says it crashed", async () => {
    const unmakeable = { ...duel, map: { ...duel.map, type: "no such map type" as never } };
    const host = testHost([unmakeable], new PageClock());
    hosts.push(host);
    const ended: BattleEnd[] = [];
    host.onEnd((end) => ended.push(end));
    const { link, port } = loopback({ side: "blue", player: "blue-1" });
    host.attach(port);
    await expect(host.ready).rejects.toThrow();
    expect(ended.map((end) => end.kind)).toEqual(["crashed"]);
    let last = "";
    for await (const record of link.records(null)) {
      last = record.kind;
      if (last === "end") break;
    }
    expect(last).toBe("end");
  });

  test("no admitted candidate rejects readiness and settles every queued request and reader", async () => {
    const refused = { ...duel, map: { ...duel.map, type: "no such map type" as never } };
    const host = testHost([refused, refused], new PageClock());
    hosts.push(host);
    const { link, port } = loopback({ side: "blue", player: "blue-1" });
    host.attach(port);
    const reading = (async () => {
      const kinds: string[] = [];
      for await (const record of link.records(null)) {
        kinds.push(record.kind);
        if (record.kind === "end") break;
      }
      return kinds;
    })();
    const waiting = Promise.allSettled([
      host.ready,
      host.referee(),
      host.replay(),
      host.seatHuman("blue-1", true),
      link.up({ seq: 1, items: [{ id: 1, order: { kind: "ready" }, queued: false }] }),
      link.preview({ kind: "purchase", variant: cards[0], destination: [0, 0] }),
    ]);
    expect((await waiting).map((result) => result.status)).toEqual(Array(6).fill("rejected"));
    expect((await reading).at(-1)).toBe("end");
  });

  test("a request still waiting when the battle is disposed is settled", async () => {
    const b = await battle(duel);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const reading = blue.read();
    await b.run(5);
    const waiting = b.host.replay();
    // Make the answer late: the battle is disposed before it is read.
    const answered = waiting.then(
      () => "answered",
      (error: Error) => error.message,
    );
    b.host.dispose();
    expect(await answered).toMatch(/answered|ended/);
    await expect(blue.order({ kind: "ready" })).rejects.toThrow(/ended/);
    await flush();
    expect(reading.end).toBe("disposed");
    expect(b.ended).toEqual([]);
  });

  test("ending one seat's readers leaves its ally reading and ordering", async () => {
    const b = await battle(squad);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const ally = seat(b, { side: "blue", player: "blue-2" });
    const seen = blue.read(),
      other = ally.read();
    await b.run(3);
    await b.host.endSeat("blue-1", "idle");
    await flush();
    expect(seen.end).toBe("idle");
    const before = tickOf(other);
    await b.run(3);
    expect(other.end).toBeNull();
    expect(tickOf(other)).toBeGreaterThan(before);
    expect((await ally.order({ kind: "ready" })).error).toBeNull();
    expect(b.ended).toEqual([]);
  });

  test("seating a human completes the administrative transaction before replying, reports orders, and replays exactly", async () => {
    const b = await battle(squad);
    const blue = seat(b, { side: "blue", player: "blue-1" });
    const seen = blue.read();
    await b.run(3);
    const applied: string[] = [];
    b.host.onOrder((player) => applied.push(player));
    expect((await blue.order({ kind: "ready" })).error).toBeNull();
    await b.run(2);
    expect(applied).toContain("blue-1");
    let completed = false;
    const taking = b.host.seatHuman("red-1", true).then(() => {
      completed = true;
    });
    for (let i = 0; i < 60 && !completed; i++) await b.advance();
    await taking;
    const referee = await b.host.referee();
    const human = referee.seats.find((s) => s.player === "red-1")!;
    expect(human).toMatchObject({
      controller: "human",
      living: 0,
      credits: referee.opening_credits,
    });
    const red = seat(b, { side: "red", player: "red-1" });
    expect((await red.order({ kind: "ready" })).error).toBeNull();
    await b.run(3);
    const replay = await b.host.replay();
    expect(JSON.parse(replay).seat_events.map((event: { kind: string }) => event.kind)).toEqual([
      "hand_over",
      "reset",
    ]);
    const replayed = await yardstick(squad, replay);
    replayed.runTo(tickOf(seen));
    expect(replayed.failure()).toBeNull();
    expect(
      [...digests(seen)].filter(([tick, digest]) => replayed.seen.get(tick) !== digest),
    ).toEqual([]);
  }, 120_000);
});
