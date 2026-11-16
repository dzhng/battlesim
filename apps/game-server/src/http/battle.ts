import type { IncomingMessage, ServerResponse } from "node:http";
import type { PreviewQuery, UpBatch } from "../../../../packages/multiplayer/src/wire.ts";
import type { Handlers } from "./seats.ts";
import { afterId, body, json, stream } from "./io.ts";
import { isBatch, isPreview } from "./input.ts";

/** The player and authenticated control-seat paths share request semantics. */
export async function battleRequest(
  request: IncomingMessage,
  response: ServerResponse,
  url: URL,
  action: string,
  seat: () => Handlers | undefined,
) {
  if (request.method !== (action === "stream" ? "GET" : "POST"))
    return json(response, 405, { refused: "method" });
  const value = action === "stream" ? undefined : await body(request);
  if ((action === "up" && !isBatch(value)) || (action === "preview" && !isPreview(value)))
    return json(response, 400, { refused: "invalid" });
  const handlers = seat();
  if (!handlers)
    return json(
      response,
      403,
      action === "up"
        ? { seq: (value as UpBatch).seq, refused: "not_seated" }
        : { refused: "not_seated" },
    );
  if (action === "stream")
    return stream(response, (deliver) => handlers.reader(afterId(url), deliver));
  if (action === "up") return json(response, 200, await handlers.up(value as UpBatch));
  json(response, 200, await handlers.preview(value as PreviewQuery));
}
