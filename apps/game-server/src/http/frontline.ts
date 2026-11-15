import type { IncomingMessage, ServerResponse } from "node:http";
import type { FrontlineService } from "../frontline.ts";
import { json } from "./io.ts";

export async function frontlineRequest(
  request: IncomingMessage,
  response: ServerResponse,
  url: URL,
  service: FrontlineService,
) {
  if (request.method !== "POST") return json(response, 405, { refused: "method" });
  const who = request.headers["x-player"];
  if (typeof who !== "string" || !/^[a-f0-9]{32}$/.test(who))
    return json(response, 403, { refused: "invalid" });
  if (url.pathname === "/frontline/join") return json(response, 200, await service.join(who));
  const match = /^\/frontline\/([^/]+)\/(deploy|leave)$/.exec(url.pathname);
  if (!match) return json(response, 404, { refused: "unknown" });
  return json(
    response,
    200,
    match[2] === "deploy" ? await service.deploy(who, match[1]) : service.leave(who, match[1]),
  );
}
