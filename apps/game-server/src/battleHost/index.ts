// One battle that never waits for a player: orders in from each seat's port,
// each side's records out. It knows nothing of HTTP, rooms or presence.
import type { SeatPort } from "../../../../packages/multiplayer/src/link.ts";
import type {
  BattleSetup,
  PreviewAnswer,
  PreviewQuery,
  UpBatch,
  UpReply,
} from "../../../../packages/multiplayer/src/wire.ts";
import { jsonRecord, recordSource } from "../../../../packages/multiplayer/src/stream/index.ts";
import type { ControllerKind } from "../../../../web/src/battle/prepare/protocol.ts";
import type { MapGenerator } from "../../../../web/src/maps/source.ts";
import type { CommandAck, RefereeView } from "../../../../web/src/battle/sim/protocol.ts";
import {
  createBattleRuntime,
  type BattleData,
  type RuntimeMessage,
  type HostRequest,
} from "./runtime.ts";
import { prepareOnThreads } from "./threads.ts";
import { prepareInOrder } from "./candidates.ts";

/** How much of a side's stream is kept for a reader to resume from. */
const RING_BYTES = 600 * 1024;
/** A side is resynced for its readers at most this often. */
const KEYFRAME_EVERY_MS = 1000;

type Side = "blue" | "red";
const SIDES: Side[] = ["blue", "red"];
export type SeatEndReason = "idle" | "leave";
export type BattleEnd = { kind: "finished" } | { kind: "crashed"; message: string };

export interface BattleSpec {
  candidates: BattleSetup[];
}

export interface BattleClock {
  /** Monotonic milliseconds. */
  now(): number;
  schedule(run: () => void, delayMs: number): void;
}

/** `"thread"` gives the battle a worker thread and the machine's clock. The
 *  other form runs it on the caller's thread and clock, for a test to drive;
 *  its `generator` stands in for the simulation's map generator. */
export type BattleRun = "thread" | { clock: BattleClock; generator?: MapGenerator };

export type BattleReady = {
  layout: string;
  tickHz: number;
  mapIdentity: string;
  /** Zero-based index into the requested candidates. */
  candidate: number;
};

export interface BattleHost {
  ready: Promise<BattleReady>;
  /** Serve a seat. The port's seat is the transport's word for who this is. */
  attach(port: SeatPort): void;
  setController(player: string, kind: ControllerKind): Promise<void>;
  referee(): Promise<RefereeView>;
  seatHuman(player: string, opening: boolean): Promise<void>;
  onOrder(handle: (player: string) => void): void;
  /** End this seat's readers without ending its side. */
  endSeat(player: string, reason: SeatEndReason): Promise<void>;
  onEnd(handle: (end: BattleEnd) => void): void;
  /** The battle's recorded orders so far. */
  replay(): Promise<string>;
  dispose(): void;
}

type Waiter<T> = { resolve: (value: T) => void; reject: (error: Error) => void };
/** The battle could not read an order or preview a seat sent. */
class Unreadable extends Error {}

export function createBattleHost(spec: BattleSpec, run: BattleRun, data: BattleData): BattleHost {
  const clock: BattleClock =
    run === "thread"
      ? { now: () => performance.now(), schedule: (task, ms) => void setTimeout(task, ms) }
      : run.clock;
  const seats = new Set(
    (spec.candidates[0]?.seats ?? []).map((seat) => `${seat.side}/${seat.player}`),
  );
  const players = new Set((spec.candidates[0]?.seats ?? []).map((seat) => seat.player));

  let layout = "";
  let tickHz = 0;
  let prepared: { mapIdentity: string; candidate: number } | null = null;
  /** Why the battle is over, once it is. */
  let over: string | null = null;
  let end: BattleEnd | null = null;
  const endings: ((end: BattleEnd) => void)[] = [];
  let nextId = 1;
  const acks = new Map<number, Waiter<CommandAck>>();
  const previews = new Map<number, Waiter<PreviewAnswer>>();
  const replays: Waiter<string>[] = [];
  const referees = new Map<number, Waiter<RefereeView>>();
  const resets = new Map<number, Waiter<void>>();
  const orders: ((player: string) => void)[] = [];
  const readers = new Map<string, Set<(reason: string) => void>>();
  let ready!: Waiter<BattleReady>;
  const readiness = new Promise<BattleReady>((resolve, reject) => (ready = { resolve, reject }));
  // A battle nobody awaited may still fail to prepare; `onEnd` reports it.
  readiness.catch(() => {});

  let send: (request: HostRequest) => void = () => {};
  let stop: () => void = () => {};

  /** The status last written to both streams; a late reader's `open` says it. */
  let told: { status: "running"; slow: boolean } | null = null;

  // A side's readers ask for a keyframe through its source. One resync
  // serves every reader waiting, and a side is resynced at most once a
  // second: an ask that comes sooner is served when the second is up.
  const resyncedAt = new Map<Side, number>();
  const resyncDue = new Set<Side>();
  const resync = (side: Side) => {
    if (over !== null || resyncDue.has(side)) return;
    const wait = (resyncedAt.get(side) ?? -Infinity) + KEYFRAME_EVERY_MS - clock.now();
    if (wait > 0) {
      resyncDue.add(side);
      clock.schedule(() => {
        resyncDue.delete(side);
        resync(side);
      }, wait);
      return;
    }
    resyncedAt.set(side, clock.now());
    send({ type: "resync", side });
  };
  const sources = Object.fromEntries(
    SIDES.map((side) => [
      side,
      recordSource(
        RING_BYTES,
        () => ({ layout, tickHz, ...(told && { status: told }) }),
        () => resync(side),
      ),
    ]),
  ) as Record<Side, ReturnType<typeof recordSource>>;
  const tellBoth = (status: "running" | "finished" | "failed", slow: boolean) => {
    const { payload } = jsonRecord("status", { status, slow });
    for (const side of SIDES) sources[side].append("status", 0, payload);
  };

  /** The battle is over: every stream says so and every request settles. */
  const close = (reason: string, result: BattleEnd | null) => {
    if (over !== null) return;
    over = reason;
    if (result) tellBoth(result.kind === "finished" ? "finished" : "failed", false);
    for (const side of SIDES) sources[side].end(reason);
    const gone = new Error(`the battle has ended: ${reason}`);
    for (const waiter of [
      ...acks.values(),
      ...previews.values(),
      ...replays,
      ...referees.values(),
      ...resets.values(),
      ready,
    ])
      waiter.reject(gone);
    referees.clear();
    resets.clear();
    acks.clear();
    previews.clear();
    replays.length = 0;
    stop();
    if (!result) return;
    end = result;
    for (const handle of endings) handle(result);
  };
  /** Players are told only that the battle crashed. Why is the server's to
   *  know (`onEnd`): an error's text may quote what a player sent. */
  const crash = (error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    close("crashed", { kind: "crashed", message });
  };

  const settle = <T>(waiters: Map<number, Waiter<T>>, id: number, value: T) => {
    waiters.get(id)?.resolve(value);
    waiters.delete(id);
  };
  const onMessage = (message: RuntimeMessage) => {
    if (over !== null) return;
    switch (message.type) {
      case "prepared":
        prepared = message;
        seats.clear();
        players.clear();
        for (const seat of spec.candidates[message.candidate].seats) {
          seats.add(`${seat.side}/${seat.player}`);
          players.add(seat.player);
        }
        return;
      case "ready":
        if (prepared === null) return crash("the battle was ready before its map");
        ({ layout, tickHz } = message);
        return ready.resolve({
          layout,
          tickHz,
          mapIdentity: prepared.mapIdentity,
          candidate: prepared.candidate,
        });
      case "publication":
        sources[message.side as Side].append("publication", message.flags, message.payload);
        return;
      case "request_refused": {
        // The battle could not read what this seat sent. That seat is told
        // so; nobody else is affected.
        const waiter = message.request === "command" ? acks : previews;
        waiter.get(message.id)?.reject(new Unreadable());
        waiter.delete(message.id);
        return;
      }
      case "referee":
        return settle(referees, message.id, message.view);
      case "seat_reset":
        return settle(resets, message.id, undefined);
      case "order_applied":
        for (const handle of orders) handle(message.player);
        return;
      case "ack":
        return settle(acks, message.id, message.ack);
      case "purchase_preview":
        return settle(previews, message.id, { purchase: message.placement });
      case "move_preview":
        return settle(previews, message.id, { moves: message.destinations });
      case "building_preview":
        return settle(previews, message.id, { building: message.placement });
      case "replay":
        return replays.shift()?.resolve(message.json);
      case "status":
        // The wire has three statuses; the last two are said by `close`.
        if (message.status === "finished") return close("finished", { kind: "finished" });
        if (message.status !== "running") return;
        if (told?.status === message.status && told.slow === message.slow) return;
        told = { status: "running", slow: message.slow };
        return tellBoth("running", message.slow);
      case "error":
        return crash(message.message);
    }
  };

  if (run === "thread") {
    const threads = prepareOnThreads({ spec, data, post: onMessage, crash });
    send = threads.send;
    stop = threads.stop;
  } else {
    const runtime = createBattleRuntime({
      now: clock.now,
      schedule: clock.schedule,
      post: onMessage,
      close: () => {},
    });
    send = (request) => runtime.handle(request);
    stop = () => runtime.handle({ type: "dispose" });
    prepareInOrder(spec.candidates, data, run.generator)
      .then(({ prepared, candidate }) => runtime.start(prepared, candidate))
      .catch(crash);
  }

  /** Ask the battle something and wait for its answer by id. */
  const ask = <T>(waiters: Map<number, Waiter<T>>, request: (id: number) => HostRequest) => {
    if (over !== null) return Promise.reject(new Error(`the battle has ended: ${over}`));
    const id = nextId++;
    return new Promise<T>((resolve, reject) => {
      waiters.set(id, { resolve, reject });
      send(request(id));
    });
  };

  return {
    ready: readiness,
    attach(port) {
      const { side, player } = port.seat;
      if (!seats.has(`${side}/${player}`))
        throw new Error(`${player} on ${side} is not a seat of this battle`);
      port.onReader((after, deliver) => {
        const group = readers.get(player) ?? new Set();
        readers.set(player, group);
        const detach = sources[side].attach(after, deliver);
        const end = (reason: string) => {
          detach();
          group.delete(end);
          deliver(jsonRecord("end", { reason }));
        };
        group.add(end);
        return () => {
          detach();
          group.delete(end);
        };
      });
      // The seat is the port's: an order is stamped with it whatever the
      // batch says, and a preview is answered from its side's knowledge.
      port.onUp(async (batch: UpBatch): Promise<UpReply> => {
        const acked = batch.items.map(async ({ id, order, queued }) => ({
          id,
          ack: await ask(acks, (ask) => ({
            type: "command",
            id: ask,
            seat: { side, player },
            order,
            queued,
          })),
        }));
        try {
          return { seq: batch.seq, results: await Promise.all(acked) };
        } catch (error) {
          if (!(error instanceof Unreadable)) throw error;
          // A page of this build never sends one; a batch holding one is
          // refused whole, whatever became of its other orders.
          await Promise.allSettled(acked);
          return { seq: batch.seq, refused: "invalid" };
        }
      });
      port.onPreview((query: PreviewQuery) =>
        preview(query).catch((error: unknown) => {
          if (!(error instanceof Unreadable)) throw error;
          return { dropped: true as const };
        }),
      );
      const preview = (query: PreviewQuery) =>
        ask(previews, (id): HostRequest => {
          if (query.kind === "purchase")
            return {
              type: "purchase_preview",
              id,
              side,
              variant: query.variant,
              destination: query.destination,
            };
          if (query.kind === "move") return { type: "move_preview", id, side, move: query.move };
          return { type: "building_preview", id, side, building: query.building };
        });
    },
    setController(player, kind) {
      if (!players.has(player)) throw new Error(`${player} is not a player of this battle`);
      send({ type: "controllers", controllers: JSON.stringify([[player, kind]]) });
      return Promise.resolve();
    },
    referee: () => ask(referees, (id) => ({ type: "referee", id })),
    seatHuman(player, opening) {
      if (!players.has(player)) throw new Error("unknown battle player");
      return ask(resets, (id) => ({ type: "seat_human", id, player, opening }));
    },
    onOrder: (handle) => {
      orders.push(handle);
    },
    endSeat(player, reason) {
      for (const end of readers.get(player) ?? []) end(reason);
      return Promise.resolve();
    },
    onEnd(handle) {
      if (end) handle(end);
      else endings.push(handle);
    },
    replay() {
      if (over !== null) return Promise.reject(new Error(`the battle has ended: ${over}`));
      return new Promise((resolve, reject) => {
        replays.push({ resolve, reject });
        send({ type: "replay" });
      });
    },
    dispose: () => close("disposed", null),
  };
}
