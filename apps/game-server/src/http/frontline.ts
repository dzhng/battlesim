import type { IncomingMessage, ServerResponse } from "node:http";
import type { FrontlineService } from "../frontline.ts";
import { clientDevice, json, player } from "./io.ts";

export async function frontlineRequest(
  request: IncomingMessage,
  response: ServerResponse,
  url: URL,
  service: FrontlineService,
  clientAddressHeader: string | null,
) {
  if (request.method !== "POST") return json(response, 405, { refused: "method" });
  const who = player(request);
  const device = clientDevice(request, clientAddressHeader, who);
  if (url.pathname === "/frontline/join")
    return json(response, 200, await service.join(who, device));
  const match = /^\/frontline\/([^/]+)\/(deploy|leave)$/.exec(url.pathname);
  if (!match) return json(response, 404, { refused: "unknown" });
  return json(
    response,
    200,
    match[2] === "deploy" ? await service.deploy(who, match[1]) : service.leave(who, match[1]),
  );
}
