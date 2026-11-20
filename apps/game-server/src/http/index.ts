import { isBrowserId, BROWSER_ID_HEADER } from "../../../../packages/multiplayer/src/identity.ts";
// The lobby's routes: build identity, hello, rooms and Frontline. HTTP owns
// credentials and sockets; the services see a browser id and a device.
import type { IncomingMessage, ServerResponse } from "node:http";
import { httpServer } from "./server.ts";
import type { BuildIdentity, ReplayAnswer } from "../../../../packages/multiplayer/src/wire.ts";
import { helloRegistry } from "./hello.ts";

import { body, json, player } from "./io.ts";
import { battleArchive } from "../archive.ts";
import { createOnlineState } from "../online.ts";
import { createFrontlineService } from "../frontline.ts";
import { frontlineRequest } from "./frontline.ts";
import { roomRequest } from "./rooms.ts";
import { createRoomService } from "../rooms.ts";
import type { BattleMachines } from "../machines/index.ts";
import type { PresenceClock } from "../presence.ts";

export type OnlineHttpOptions = {
  machines: BattleMachines;
  /** The folder that keeps every battle's record. */
  archive: string;
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
    archive: battleArchive(options.archive, build),
    callsign: hello.callsignOf,
    releaseCallsign: hello.release,
  });
  const rooms = createRoomService({ ...options, online });
  const frontline = createFrontlineService({
    ...options,
    online,
  });
  const server = httpServer(route);
  server.once("close", () => {
    rooms.dispose();
    frontline.dispose();
    online.dispose();
  });
  return server;

  async function route(request: IncomingMessage, response: ServerResponse, url: URL) {
    if (url.pathname === "/build") {
      if (request.method !== "GET") return json(response, 405, { refused: "method" });
      const { sim, data, wire } = build;
      return json(response, 200, { sim, data, wire });
    }
    if (url.pathname === "/deployment") {
      if (request.method !== "GET") return json(response, 405, { refused: "method" });
      return json(response, 200, {
        commit: (build as BuildIdentity & { commit?: string }).commit,
      });
    }
    if (url.pathname === "/hello") {
      if (request.method !== "POST") return json(response, 405, { refused: "method" });
      const id = request.headers[BROWSER_ID_HEADER.toLowerCase()];
      if (!isBrowserId(id)) return json(response, 403, { refused: "invalid" });
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
    const exported = /^\/battles\/([^/]+)\/replay$/.exec(url.pathname);
    if (exported) {
      if (request.method !== "POST") return json(response, 405, { refused: "method" });
      const answer = await online.battleReplay(decodeURIComponent(exported[1]), player(request));
      if (!("refused" in answer)) return json(response, 200, answer satisfies ReplayAnswer);
      const status = { unknown: 404, not_seated: 403, unavailable: 410 }[answer.refused];
      return json(response, status, answer);
    }
    if (url.pathname.startsWith("/frontline/"))
      return frontlineRequest(request, response, url, frontline, options.clientAddressHeader);
    json(response, 404, { refused: "unknown" });
  }
}
