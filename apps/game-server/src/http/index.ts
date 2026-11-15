// HTTP owns credentials and sockets. The battle only sees the seat's port;
// recordSource owns all history and the decision to resume or resync.
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import type {
  BattleTicket,
  PreviewQuery,
  UpBatch,
} from "../../../../packages/multiplayer/src/wire.ts";
import { isBatch, isPreview } from "./input.ts";
import { battleRoute, HttpRefusal, type BattleRoute, type Handlers } from "./seats.ts";
export { battleRoute };
import { encodeRecord } from "../../../../packages/multiplayer/src/stream/index.ts";

/** Enough for a join snapshot, but a stalled socket must never retain a battle's history. */
const SOCKET_QUEUE_BYTES = 4 * 1024 * 1024;
/** Orders are small; cap a hostile JSON body before it can consume unbounded memory. */
const BODY_BYTES = 64 * 1024;

function json(response: ServerResponse, status: number, value: unknown) {
  response.writeHead(status, { "Content-Type": "application/json" });
  response.end(JSON.stringify(value));
}
async function body(request: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > BODY_BYTES) throw new Error("body too large");
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}
function stream(response: ServerResponse, handlers: Handlers, after: number | null) {
  response.writeHead(200, {
    "Content-Type": "application/octet-stream",
    "X-Accel-Buffering": "no",
  });
  response.flushHeaders();
  let gone: (() => void) | undefined;
  response.once("close", () => gone?.());
  gone = handlers.reader(after, (record) => {
    if (response.destroyed || response.writableEnded) return false;
    response.write(encodeRecord(record));
    if (response.writableLength > SOCKET_QUEUE_BYTES) {
      response.destroy();
      return false;
    }
    if (record.kind === "end") response.end();
    return true;
  });
}

export function createBattleHttp(
  battle: (id: string) => BattleRoute | undefined,
  devJoin?: (browserId: string) => Promise<BattleTicket | { refused: "full" }>,
) {
  return createServer((request, response) => {
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

  async function route(request: IncomingMessage, response: ServerResponse) {
    if (request.method === "OPTIONS") {
      response.writeHead(204);
      response.end();
      return;
    }
    const url = new URL(request.url!, "http://localhost");
    if (url.pathname === "/dev/battle" && devJoin) {
      if (request.method !== "POST") return json(response, 405, { refused: "method" });
      const browserId = request.headers["x-player"];
      if (typeof browserId !== "string" || !/^[a-f0-9]{32}$/.test(browserId))
        return json(response, 403, { refused: "not_seated" });
      const result = await devJoin(browserId);
      return json(response, "refused" in result ? 409 : 200, result);
    }
    const match = /^\/battles\/([^/]+)\/(stream|up|preview)$/.exec(url.pathname);
    const found = match && battle(match[1]);
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
      const parameter = url.searchParams.get("after");
      const after = parameter === null ? null : Number(parameter);
      if (
        after !== null &&
        (!/^\d+$/.test(parameter!) || !Number.isInteger(after) || after < 0 || after > 0xffffffff)
      )
        return json(response, 400, { refused: "invalid" });
      return stream(response, handlers, after);
    }
    if (action === "up") return json(response, 200, await handlers.up(value as UpBatch));
    return json(response, 200, await handlers.preview(value as PreviewQuery));
  }
}
