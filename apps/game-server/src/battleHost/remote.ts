import { setTimeout as delay } from "node:timers/promises";
import type { Seat, SeatPort } from "../../../../packages/multiplayer/src/link.ts";
import {
  jsonRecord,
  readJson,
  resumingReader,
} from "../../../../packages/multiplayer/src/stream/index.ts";
import { httpRecords } from "../../../../packages/multiplayer/src/stream/http.ts";
import type { PreviewAnswer, UpReply } from "../../../../packages/multiplayer/src/wire.ts";
import type { RefereeView } from "../../../../web/src/battle/sim/protocol.ts";
import { MachineUnavailable, type Machine } from "../machines/index.ts";
import {
  CONTROL_HEADER,
  CONTROL_REQUEST_TIMEOUT_MS,
  LOBBY_LEASE_MS,
  CONTROL_PING_EVERY_MS,
  CONTROL_PING_TIMEOUT_MS,
} from "../http/controlProtocol.ts";
import type { ControlEvent, StreamPresence } from "../http/controlEvents.ts";
import type { BattleEnd, BattleHost, BattleSpec } from "./index.ts";

/** These operations belong to hosting, outside the simulation host interface. */
export interface RemoteBattleHost extends BattleHost {
  readonly address: string | undefined;
  setSeating(entries: [string, Seat][]): Promise<void>;
  onStream(handle: (streams: StreamPresence[]) => void): void;
}

export function createRemoteBattleHost(
  spec: BattleSpec,
  id: string,
  machine: Promise<Machine>,
  key: string,
): RemoteBattleHost {
  const lifetime = new AbortController();
  const players = new Map((spec.candidates[0]?.seats ?? []).map((s) => [s.player, s]));
  const endings: ((end: BattleEnd) => void)[] = [];
  const orders: ((player: string) => void)[] = [];
  const listeners: ((streams: StreamPresence[]) => void)[] = [];
  const streams = new Map<string, StreamPresence>();
  const readers = new Set<(reason: string) => void>();
  const counts: Record<string, number> = {};
  let m: Machine | undefined;
  let end: BattleEnd | null = null;
  let over: string | null = null;
  let ping: ReturnType<typeof setTimeout> | undefined;
  let silence: ReturnType<typeof setTimeout> | undefined;
  let leased = false;
  const answered = () => {
    clearTimeout(silence);
    if (!over && leased) silence = setTimeout(crash, LOBBY_LEASE_MS);
  };
  let mutations: Promise<unknown> = Promise.resolve();
  const tellStreams = () => {
    for (const handle of listeners) handle([...streams.values()]);
  };
  const close = (reason: string, result: BattleEnd | null) => {
    if (over) return;
    over = reason;
    end = result;
    clearTimeout(ping);
    clearTimeout(silence);
    lifetime.abort();
    for (const finish of readers) finish(reason);
    readers.clear();
    streams.clear();
    tellStreams();
    m?.stop();
    if (result) for (const handle of endings) handle(result);
  };
  const crash = () => close("crashed", { kind: "crashed", message: "battle server unavailable" });
  const endedError = () => new Error(`the battle has ended: ${over ?? "unavailable"}`);
  async function post<T>(path: string, value?: unknown): Promise<T> {
    if (over) throw endedError();
    try {
      const remote = await machine;
      if (over) throw endedError();
      const response = await fetch(`${remote.control}/control/${path}`, {
        method: "POST",
        headers: { [CONTROL_HEADER]: key, "Content-Type": "application/json" },
        body: value === undefined ? undefined : JSON.stringify(value),
        signal: AbortSignal.any([lifetime.signal, AbortSignal.timeout(CONTROL_REQUEST_TIMEOUT_MS)]),
      });
      if (!response.ok) throw endedError();
      const result = (await response.json()) as T;
      answered();
      return result;
    } catch {
      throw endedError();
    }
  }
  const serial = <T>(run: () => Promise<T>) => {
    const result = mutations.then(run);
    mutations = result.catch(() => {});
    // Callers of fire-and-observe operations still hear failure through onEnd.
    void result.catch(() => {});
    return result;
  };
  const check = (player: string) => {
    if (!players.has(player)) throw new Error("unknown battle player");
  };
  async function heartbeat() {
    if (over || !m) return;
    try {
      const response = await fetch(`${m.control}/control/ping`, {
        method: "POST",
        headers: { [CONTROL_HEADER]: key },
        signal: AbortSignal.any([lifetime.signal, AbortSignal.timeout(CONTROL_PING_TIMEOUT_MS)]),
      });
      if (!response.ok) throw endedError();
      answered();
    } catch {
      // Silence, rather than an individual slow or failed request, ends the battle.
    } finally {
      if (!over) ping = setTimeout(() => void heartbeat(), CONTROL_PING_EVERY_MS);
    }
  }
  async function events() {
    try {
      for await (const record of resumingReader(
        (after) =>
          httpRecords(
            `${m!.control}/control/events${after === null ? "" : `?after=${after}`}`,
            { [CONTROL_HEADER]: key },
            lifetime.signal,
          ),
        { sleep: (ms) => delay(ms, undefined, { signal: lifetime.signal }) },
      )) {
        if (over) return;
        answered();
        if (record.kind === "end") {
          const { reason } = readJson<{ reason: string }>(record);
          close(
            reason === "finished" ? "finished" : "crashed",
            reason === "finished"
              ? { kind: "finished" }
              : { kind: "crashed", message: "battle server failed" },
          );
          return;
        }
        if (record.kind !== "state") continue;
        const event = readJson<ControlEvent>(record);
        switch (event.kind) {
          case "snapshot":
            streams.clear();
            for (const stream of event.streams) streams.set(stream.connection, stream);
            tellStreams();
            for (const [player, count] of Object.entries(event.orders)) {
              if (count > (counts[player] ?? 0)) for (const handle of orders) handle(player);
              counts[player] = count;
            }
            break;
          case "stream":
            if (event.open) streams.set(event.stream.connection, event.stream);
            else streams.delete(event.stream.connection);
            tellStreams();
            break;
          case "order":
            counts[event.player] = (counts[event.player] ?? 0) + 1;
            for (const handle of orders) handle(event.player);
            break;
          case "ended":
            close(event.end.kind === "finished" ? "finished" : "crashed", event.end);
            return;
        }
      }
    } catch {
      // The resumable reader retries dropped connections itself. Reaching this
      // catch means the stream broke its contract, or our lifetime ended.
      crash();
    }
  }
  const ready = (async () => {
    try {
      m = await machine.catch(() => {
        throw new MachineUnavailable();
      });
      if (over) {
        m.stop();
        throw endedError();
      }
      void events();
      const ready = await post<Awaited<BattleHost["ready"]>>("start", {
        id,
        candidates: spec.candidates,
      });
      players.clear();
      for (const seat of spec.candidates[ready.candidate].seats) players.set(seat.player, seat);
      leased = true;
      answered();
      void heartbeat();
      return ready;
    } catch (error) {
      crash();
      if (error instanceof MachineUnavailable) throw error;
      throw endedError();
    }
  })();
  void ready.catch(() => {});

  return {
    ready,
    get address() {
      return m?.address;
    },
    setSeating: (entries) =>
      serial(async () => {
        await ready;
        await post("seating", entries);
      }),
    onStream: (handle) => {
      listeners.push(handle);
      handle([...streams.values()]);
    },
    attach(port: SeatPort) {
      const seat = players.get(port.seat.player);
      if (!seat || seat.side !== port.seat.side) throw new Error("unknown battle seat");
      const path = `seats/${encodeURIComponent(seat.player)}`;
      port.onUp((batch) =>
        serial(async () => {
          await ready;
          return post<UpReply>(`${path}/up`, batch);
        }),
      );
      port.onPreview((query) =>
        serial(async () => {
          await ready;
          return post<PreviewAnswer>(`${path}/preview`, query);
        }),
      );
      port.onReader((after, deliver) => {
        const connection = new AbortController();
        let finished = false;
        const finish = (reason: string) => {
          if (finished) return;
          finished = true;
          deliver(jsonRecord("end", { reason }));
          connection.abort();
          readers.delete(finish);
        };
        readers.add(finish);
        void (async () => {
          if (over) {
            deliver(jsonRecord("open", { resume: "continued" }));
            finish(over);
            return;
          }
          await ready;
          await mutations;
          const signal = AbortSignal.any([lifetime.signal, connection.signal]);
          for await (const record of resumingReader(
            (cursor) =>
              httpRecords(
                `${m!.control}/control/${path}/stream${(cursor ?? after) === null ? "" : `?after=${cursor ?? after}`}`,
                { [CONTROL_HEADER]: key },
                signal,
              ),
            { sleep: (ms) => delay(ms, undefined, { signal }) },
          )) {
            if (finished) return;
            if (!deliver(record) || record.kind === "end") {
              finished = true;
              connection.abort();
              readers.delete(finish);
              return;
            }
          }
        })().catch(() => {
          if (!connection.signal.aborted && !over) crash();
        });
        return () => {
          finished = true;
          connection.abort();
          readers.delete(finish);
        };
      });
    },
    setController(player, kind) {
      check(player);
      return serial(async () => {
        await ready;
        await post("controller", { player, kind });
      });
    },
    seatHuman(player, opening) {
      check(player);
      return serial(async () => {
        await ready;
        await post("human", { player, opening });
      });
    },
    endSeat(player, reason) {
      if (over) return Promise.resolve();
      return serial(async () => {
        await ready;
        await post("end-seat", { player, reason });
      });
    },
    referee: () =>
      serial(async () => {
        await ready;
        return post<RefereeView>("referee");
      }),
    replay: () =>
      serial(async () => {
        await ready;
        return post<string>("replay");
      }),
    onOrder: (handle) => {
      orders.push(handle);
    },
    onEnd(handle) {
      if (end) handle(end);
      else endings.push(handle);
    },
    dispose: () => close("disposed", null),
  };
}
