import { scrubCredentials, isBrowserId } from "../../../../packages/multiplayer/src/identity.ts";
import { randomUUID, timingSafeEqual } from "node:crypto";
import type { BattleSeat } from "../../../../packages/multiplayer/src/link.ts";
import type { BattleHost, BattleSpec } from "../battleHost/index.ts";
import { afterId, body, json, stream, player } from "./io.ts";
import { battleRoute } from "./seatTransport.ts";
import { HttpRefusal } from "./refusal.ts";
import { battleRequest } from "./battle.ts";
import { httpServer } from "./server.ts";
import { controlEvents } from "./controlEvents.ts";

import { CONTROL_HEADER, LOBBY_LEASE_MS, CONTROL_REQUEST_TIMEOUT_MS } from "./controlProtocol.ts";

export function createBattleServer(options: {
  key: string;
  createBattle(spec: BattleSpec): BattleHost;
  ordersPerSecond: number;
  now?: () => number;
}) {
  let host: BattleHost | undefined;
  let id: string | undefined;
  const seating = new Map<string, BattleSeat>();
  let players: ReturnType<typeof battleRoute> | undefined;
  let touched = performance.now();
  const events = controlEvents();
  let phase: "waiting" | "preparing" | "running" | "stopped" = "waiting";
  const server = httpServer(async (request, response, url) => {
    if (url.pathname.startsWith("/control/")) {
      const supplied = request.headers[CONTROL_HEADER.toLowerCase()];
      const wanted = Buffer.from(options.key);
      const given = typeof supplied === "string" ? Buffer.from(supplied) : Buffer.alloc(0);
      if (given.length !== wanted.length || !timingSafeEqual(given, wanted))
        return json(response, 403, { refused: "forbidden" });
      touched = performance.now();
      if (url.pathname === "/control/events") {
        if (request.method !== "GET") throw new HttpRefusal(405, "method");
        return stream(response, (deliver) => events.source.attach(afterId(url), deliver));
      }
      if (request.method !== "POST") throw new HttpRefusal(405, "method");
      switch (url.pathname) {
        case "/control/ping":
          return json(response, 200, { ok: true });
        case "/control/start": {
          if (phase !== "waiting") throw new HttpRefusal(409, "started");
          phase = "preparing";
          response.once("close", () => {
            if (phase === "preparing") stop();
          });
          const value = (await body(request)) as BattleSpec & { id: string };
          id = value.id;
          host = options.createBattle({ candidates: value.candidates });
          const observed: Pick<BattleHost, "attach"> = {
            attach(port) {
              host!.attach({
                ...port,
                onReader(read) {
                  port.onReader((after, deliver) => {
                    const connection = { player: port.seat.player, connection: randomUUID() };
                    events.stream(connection, true);
                    let closed = false;
                    const close = () => {
                      if (closed) return;
                      closed = true;
                      events.stream(connection, false);
                    };
                    const detach = read(after, (record) => {
                      const accepted = deliver(record);
                      if (!accepted || record.kind === "end") close();
                      return accepted;
                    });
                    return () => {
                      detach();
                      close();
                    };
                  });
                },
              });
            },
          };
          players = battleRoute(
            observed,
            (who) => seating.get(who),
            options.ordersPerSecond,
            options.now,
          );
          host.onOrder(events.order);
          host.onEnd((end) => {
            if (end.kind === "crashed") {
              // Credentials remain private after a seat is released too.
              const message = scrubCredentials(end.message);
              console.error(`A battle crashed: ${message}`);
            }
            events.end(end);
          });
          try {
            const ready = await host.ready;
            touched = performance.now();
            json(response, 200, ready);
          } finally {
            if (phase === "preparing") phase = "running";
          }
          return;
        }
        case "/control/seating": {
          const entries = (await body(request)) as [string, BattleSeat][];
          if (entries.some(([who]) => !isBrowserId(who))) throw new HttpRefusal(400, "invalid");
          players?.seating(entries);
          seating.clear();
          for (const [who, seat] of entries) seating.set(who, seat);
          return json(response, 200, { ok: true });
        }
        case "/control/controller": {
          const value = (await body(request)) as {
            player: string;
            kind: Parameters<BattleHost["setController"]>[1];
          };
          await required().setController(value.player, value.kind);
          return json(response, 200, { ok: true });
        }
        case "/control/human": {
          const value = (await body(request)) as { player: string; opening: boolean };
          await required().seatHuman(value.player, value.opening);
          return json(response, 200, { ok: true });
        }
        case "/control/end-seat": {
          const value = (await body(request)) as {
            player: string;
            reason: Parameters<BattleHost["endSeat"]>[1];
          };
          // Reasons are server vocabulary, never reflected arbitrary input.
          if (value.reason !== "idle" && value.reason !== "leave")
            throw new HttpRefusal(400, "invalid");
          await required().endSeat(value.player, value.reason);
          return json(response, 200, { ok: true });
        }
        case "/control/referee":
          return json(response, 200, await required().referee());
        case "/control/replay":
          return json(response, 200, await required().replay());
        default:
          throw new HttpRefusal(404, "unknown");
      }
    }
    const match = /^\/battles\/([^/]+)\/(stream|up|preview|replay)$/.exec(url.pathname);
    if (!match || match[1] !== id || !players) throw new HttpRefusal(404, "unknown");
    return battleRequest(
      request,
      response,
      url,
      match[2],
      () => players!.seat(player(request)),
      () => required().replay(),
    );
  });
  function required() {
    if (!host) throw new HttpRefusal(409, "not_started");
    return host;
  }
  const lease = setInterval(() => {
    if (
      performance.now() - touched >=
      (phase === "preparing" ? CONTROL_REQUEST_TIMEOUT_MS : LOBBY_LEASE_MS)
    )
      stop();
  }, LOBBY_LEASE_MS / 4);
  function stop() {
    if (phase === "stopped") return;
    phase = "stopped";
    clearInterval(lease);
    host?.dispose();
    events.source.end("disposed");
    server.closeAllConnections();
    server.close();
  }
  server.once("close", stop);
  return { server, stop };
}
