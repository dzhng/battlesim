// HTTP owns credentials and sockets. The battle only sees the seat's port;
// recordSource owns all history and the decision to resume or resync.
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type {
  BuildIdentity,
  PreviewQuery,
  UpBatch,
} from "../../../../packages/multiplayer/src/wire.ts";
import { helloRegistry } from "./hello.ts";
import { isBatch, isPreview } from "./input.ts";
import { battleRoute, HttpRefusal, type BattleRoute } from "./seats.ts";
export { battleRoute };

import { afterId, body, json, stream } from "./io.ts";
import { roomRequest } from "./rooms.ts";
import { createRoomService } from "../rooms.ts";
import type { BattleSetup } from "../../../../packages/multiplayer/src/wire.ts";
import type { BattleHost } from "../battleHost/index.ts";
import type { PresenceClock } from "../presence.ts";

export type RoomHttpOptions = {
  createBattle: (setup: BattleSetup) => BattleHost;
  ordersPerSecond: number;
  capacity: number;
  clock: PresenceClock;
  random: () => number;
  clientAddressHeader: string | null;
};

export function createBattleHttp(
  battle: (id: string) => BattleRoute | undefined,
  build: BuildIdentity,
  roomOptions: RoomHttpOptions,
) {
  const hello = helloRegistry(build);
  const rooms = createRoomService({
    ...roomOptions,
    callsign: hello.callsignOf,
    releaseCallsign: hello.release,
  });
  const battleRoutes = new WeakMap<BattleHost, BattleRoute>();
  const server = createServer((request, response) => {
    response.setHeader("Cache-Control", "no-store, no-transform");
    response.setHeader("Access-Control-Allow-Origin", "*");
    response.setHeader("Access-Control-Allow-Headers", "X-Player, Content-Type");
    response.setHeader("Access-Control-Max-Age", "7200");
    response.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    void route(request, response).catch((error: unknown) => {
      // Neither headers nor exception text may cross this boundary.
      if (response.headersSent) response.destroy();
      else if (error instanceof HttpRefusal)
        json(response, error.status, { refused: error.reason });
      else json(response, 410, { refused: "unavailable" });
    });
  });
  server.once("close", rooms.dispose);
  return server;

  async function route(request: IncomingMessage, response: ServerResponse) {
    if (request.method === "OPTIONS") {
      response.writeHead(204);
      response.end();
      return;
    }
    const url = new URL(request.url!, "http://localhost");
    if (url.pathname === "/hello") {
      if (request.method !== "POST") return json(response, 405, { refused: "method" });
      const id = request.headers["x-player"];
      if (typeof id !== "string" || !/^[a-f0-9]{32}$/.test(id))
        return json(response, 403, { refused: "invalid" });
      let value: unknown;
      try {
        value = await body(request);
      } catch {
        return json(response, 400, { refused: "invalid" });
      }
      const result = hello(id, value);
      if (!("refused" in result)) rooms.presence.track(id);
      return json(response, "refused" in result ? 409 : 200, result);
    }
    if (url.pathname === "/rooms" || url.pathname.startsWith("/rooms/"))
      return roomRequest(request, response, url, rooms, roomOptions.clientAddressHeader);
    const match = /^\/battles\/([^/]+)\/(stream|up|preview)$/.exec(url.pathname);
    const roomBattle = match && rooms.battle(match[1]);
    let found = match && battle(match[1]);
    if (roomBattle) {
      found = battleRoutes.get(roomBattle.host);
      if (!found) {
        found = battleRoute(
          roomBattle.host,
          roomBattle.seatOf,
          roomOptions.ordersPerSecond,
          roomOptions.clock.now,
        );
        battleRoutes.set(roomBattle.host, found);
      }
    }
    if (!match || !found) return json(response, 404, { refused: "unknown" });
    const action = match[2];
    if (request.method !== (action === "stream" ? "GET" : "POST"))
      return json(response, 405, { refused: "method" });
    let value: unknown;
    try {
      if (action !== "stream") value = await body(request);
    } catch {
      return json(response, 400, { refused: "invalid" });
    }
    if ((action === "up" && !isBatch(value)) || (action === "preview" && !isPreview(value)))
      return json(response, 400, { refused: "invalid" });
    const id = request.headers["x-player"];
    const handlers = typeof id === "string" ? found.seat(id) : undefined;
    if (!handlers)
      return json(
        response,
        403,
        action === "up"
          ? { seq: (value as UpBatch)?.seq, refused: "not_seated" }
          : { refused: "not_seated" },
      );
    if (action === "stream") {
      const after = afterId(url);
      return stream(response, (deliver) => {
        const closePresence = rooms.presence.open(id as string);
        const detach = handlers.reader(after, deliver);
        return () => {
          detach();
          closePresence();
        };
      });
    }
    if (action === "up") return json(response, 200, await handlers.up(value as UpBatch));
    return json(response, 200, await handlers.preview(value as PreviewQuery));
  }
}
