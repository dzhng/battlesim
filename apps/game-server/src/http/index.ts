// HTTP owns credentials and sockets. The battle only sees the seat's port;
// recordSource owns all history and the decision to resume or resync.
import type { IncomingMessage, ServerResponse } from "node:http";
import { httpServer } from "./server.ts";
import type { BuildIdentity } from "../../../../packages/multiplayer/src/wire.ts";
import { helloRegistry } from "./hello.ts";

import { body, json } from "./io.ts";
import { createOnlineState } from "../online.ts";
import { createFrontlineService } from "../frontline.ts";
import { frontlineRequest } from "./frontline.ts";
import { roomRequest } from "./rooms.ts";
import { createRoomService } from "../rooms.ts";
import type { BattleMachines } from "../machines/index.ts";
import type { PresenceClock } from "../presence.ts";

export type OnlineHttpOptions = {
  machines: BattleMachines;
  clock: PresenceClock;
  random: () => number;
  /** How many maps a new Frontline game may draw before giving up. */
  mapAttempts: number;
  clientAddressHeader: string | null;
};

export function createLobbyHttp(build: BuildIdentity, options: OnlineHttpOptions) {
  const hello = helloRegistry(build);
  const online = createOnlineState({
    ...options,
    callsign: hello.callsignOf,
    releaseCallsign: hello.release,
  });
  const rooms = createRoomService({ ...options, online, createBattle: online.createBattle });
  const frontline = createFrontlineService({
    ...options,
    online,
    createBattle: online.createBattle,
  });
  const server = httpServer(route);
  server.once("close", () => {
    rooms.dispose();
    frontline.dispose();
    online.dispose();
  });
  return server;

  async function route(request: IncomingMessage, response: ServerResponse, url: URL) {
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
      if (!("refused" in result)) online.presence.track(id);
      return json(response, "refused" in result ? 409 : 200, result);
    }
    if (url.pathname === "/rooms" || url.pathname.startsWith("/rooms/"))
      return roomRequest(request, response, url, rooms, options.clientAddressHeader);
    if (url.pathname.startsWith("/frontline/"))
      return frontlineRequest(request, response, url, frontline);
    json(response, 404, { refused: "unknown" });
  }
}
