import type { IncomingMessage, ServerResponse } from "node:http";
import type { RoomService } from "../rooms.ts";
import { afterId, body, clientDevice, json, player, stream } from "./io.ts";
import { isRoomAction } from "./input.ts";
import { HttpRefusal } from "./seats.ts";

export async function roomRequest(
  request: IncomingMessage,
  response: ServerResponse,
  url: URL,
  rooms: RoomService,
  clientAddressHeader: string | null,
) {
  const who = player(request);
  const device = clientDevice(request, clientAddressHeader, who);
  if (url.pathname === "/rooms") {
    if (request.method === "GET")
      return stream(response, (deliver) => rooms.listStream(who, afterId(url), deliver));
    if (request.method !== "POST") throw new HttpRefusal(405, "method");
    const value = await body(request);
    if (
      !value ||
      typeof value !== "object" ||
      Array.isArray(value) ||
      !("visibility" in value) ||
      (value.visibility !== "public" && value.visibility !== "private")
    )
      throw new HttpRefusal(400, "invalid");
    const result = rooms.create(who, value.visibility, device);
    return json(response, "refused" in result ? 409 : 200, result);
  }
  const match = /^\/rooms\/([^/]+)(\/act)?$/.exec(url.pathname);
  if (!match) throw new HttpRefusal(404, "unknown");
  if (request.method !== (match[2] ? "POST" : "GET")) throw new HttpRefusal(405, "method");
  if (match[2]) {
    const value = await body(request);
    if (!isRoomAction(value)) throw new HttpRefusal(400, "invalid");
    const result = rooms.act(who, decodeURIComponent(match[1]), value);
    return json(response, "refused" in result ? 409 : 200, result);
  }
  const after = afterId(url);
  return stream(response, (deliver) => {
    const opened = rooms.roomStream(who, decodeURIComponent(match[1]), after, deliver);
    if ("refused" in opened) throw new HttpRefusal(409, opened.refused);
    return opened.close;
  });
}
