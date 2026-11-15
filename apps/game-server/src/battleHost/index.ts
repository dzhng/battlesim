// One battle that never waits for a player: orders in from each seat's port,
// each side's records out. It knows nothing of HTTP, rooms or presence.
import { Worker } from "node:worker_threads";
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
import type { CommandAck, SimRequest } from "../../../../web/src/battle/sim/protocol.ts";
import { createBattleRuntime, type BattleData, type RuntimeMessage } from "./runtime.ts";
import type { WorkerInit } from "./worker.ts";

/** How much of a side's stream is kept for a reader to resume from. */
const RING_BYTES = 600 * 1024;
/** A side is resynced for its readers at most this often. */
const KEYFRAME_EVERY_MS = 1000;

type Side = "blue" | "red";
const SIDES: Side[] = ["blue", "red"];
export type BattleEnd = { kind: "finished" } | { kind: "crashed"; message: string };

export interface BattleSpec {
  setup: BattleSetup;
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

export interface BattleHost {
  ready: Promise<{ layout: string; tickHz: number; mapIdentity: string }>;
  /** Serve a seat. The port's seat is the transport's word for who this is. */
  attach(port: SeatPort): void;
  setController(player: string, kind: ControllerKind): void;
  onEnd(handle: (end: BattleEnd) => void): void;
  /** The battle's recorded orders so far. */
  replay(): Promise<string>;
  dispose(): void;
}

type Waiter<T> = { resolve: (value: T) => void; reject: (error: Error) => void };

export function createBattleHost(spec: BattleSpec, run: BattleRun, data: BattleData): BattleHost {
  const clock: BattleClock =
    run === "thread"
      ? { now: () => performance.now(), schedule: (task, ms) => void setTimeout(task, ms) }
      : run.clock;
  const seats = new Set(spec.setup.seats.map((seat) => `${seat.side}/${seat.player}`));
  const players = new Set(spec.setup.seats.map((seat) => seat.player));

  let layout = "";
  let tickHz = 0;
  let mapIdentity: string | null = null;
  /** Why the battle is over, once it is. */
  let over: string | null = null;
  let end: BattleEnd | null = null;
  const endings: ((end: BattleEnd) => void)[] = [];
  let nextId = 1;
  const acks = new Map<number, Waiter<CommandAck>>();
  const previews = new Map<number, Waiter<PreviewAnswer>>();
  const replays: Waiter<string>[] = [];
  let ready!: Waiter<{ layout: string; tickHz: number; mapIdentity: string }>;
  const readiness = new Promise<{ layout: string; tickHz: number; mapIdentity: string }>(
    (resolve, reject) => (ready = { resolve, reject }),
  );
  // A battle nobody awaited may still fail to prepare; `onEnd` reports it.
  readiness.catch(() => {});

  let send: (request: SimRequest) => void = () => {};
  let stop: () => void = () => {};

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
        () => ({ layout, tickHz }),
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
    for (const waiter of [...acks.values(), ...previews.values(), ...replays, ready])
      waiter.reject(gone);
    acks.clear();
    previews.clear();
    replays.length = 0;
    stop();
    if (!result) return;
    end = result;
    for (const handle of endings) handle(result);
  };
  const crash = (error: unknown) => {
    const message = error instanceof Error ? error.message : String(error);
    close(message, { kind: "crashed", message });
  };

  let told: { status: string; slow: boolean } | null = null;
  const settle = <T>(waiters: Map<number, Waiter<T>>, id: number, value: T) => {
    waiters.get(id)?.resolve(value);
    waiters.delete(id);
  };
  const onMessage = (message: RuntimeMessage) => {
    if (over !== null) return;
    switch (message.type) {
      case "prepared":
        mapIdentity = message.mapIdentity;
        return;
      case "ready":
        if (mapIdentity === null) return crash("the battle was ready before its map");
        ({ layout, tickHz } = message);
        return ready.resolve({ layout, tickHz, mapIdentity });
      case "publication":
        sources[message.side as Side].append("publication", message.flags, message.payload);
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
        told = { status: message.status, slow: message.slow };
        return tellBoth("running", message.slow);
      case "error":
        return crash(message.message);
    }
  };

  if (run === "thread") {
    const worker = new Worker(new URL("./worker.js", import.meta.url));
    worker.on("message", onMessage);
    worker.on("error", crash);
    worker.on("exit", (code) => crash(`the battle's thread exited with code ${code}`));
    send = (request) => worker.postMessage(request);
    stop = () => void worker.terminate();
    const { presets, templates, limits, rules, root } = data;
    worker.postMessage({
      type: "battle-host-init",
      setup: spec.setup,
      data: { presets, templates, limits, rules, root },
    } satisfies WorkerInit);
  } else {
    const runtime = createBattleRuntime({
      setup: spec.setup,
      data,
      generator: run.generator,
      now: clock.now,
      schedule: clock.schedule,
      post: onMessage,
      close: () => {},
    });
    send = (request) => runtime.handle(request);
    stop = () => runtime.handle({ type: "dispose" });
    runtime.start().catch(crash);
  }

  /** Ask the battle something and wait for its answer by id. */
  const ask = <T>(waiters: Map<number, Waiter<T>>, request: (id: number) => SimRequest) => {
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
      port.onReader((after, deliver) => sources[side].attach(after, deliver));
      // The seat is the port's: an order is stamped with it whatever the
      // batch says, and a preview is answered from its side's knowledge.
      port.onUp(
        async (batch: UpBatch): Promise<UpReply> => ({
          seq: batch.seq,
          results: await Promise.all(
            batch.items.map(async ({ id, order, queued }) => ({
              id,
              ack: await ask(acks, (ask) => ({
                type: "command",
                id: ask,
                seat: { side, player },
                order,
                queued,
              })),
            })),
          ),
        }),
      );
      port.onPreview((query: PreviewQuery) =>
        ask(previews, (id): SimRequest => {
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
        }),
      );
    },
    setController(player, kind) {
      if (!players.has(player)) throw new Error(`${player} is not a player of this battle`);
      send({ type: "controllers", controllers: JSON.stringify([[player, kind]]) });
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
