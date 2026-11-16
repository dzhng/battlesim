import { isBrowserId, BROWSER_ID_HEADER } from "../../../../packages/multiplayer/src/identity.ts";
import type { IncomingMessage, ServerResponse } from "node:http";
import {
  encodeRecord,
  type RecordSource,
} from "../../../../packages/multiplayer/src/stream/index.ts";
import { HttpRefusal } from "./refusal.ts";

/** Enough for a join snapshot, but a stalled socket must not retain history. */
const SOCKET_QUEUE_BYTES = 4 * 1024 * 1024;
/** Refuse unbounded JSON before it consumes unbounded memory. */
const BODY_BYTES = 64 * 1024;

export function json(response: ServerResponse, status: number, value: unknown) {
  response.writeHead(status, { "Content-Type": "application/json" });
  response.end(JSON.stringify(value));
}
export async function body(request: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  let size = 0;
  try {
    for await (const chunk of request) {
      size += chunk.length;
      if (size > BODY_BYTES) throw new Error("body too large");
      chunks.push(chunk);
    }
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    throw new HttpRefusal(400, "invalid");
  }
}
export function player(request: IncomingMessage): string {
  const id = request.headers[BROWSER_ID_HEADER.toLowerCase()];
  if (!isBrowserId(id)) throw new HttpRefusal(403, "invalid");
  return id;
}
/** Only the configured proxy header defines a device; direct lobbies use the browser id. */
export function clientDevice(
  request: IncomingMessage,
  headerName: string | null,
  who: string,
): string {
  if (headerName === null) return who;
  const address = request.headers[headerName.toLowerCase()];
  if (typeof address !== "string" || address.length === 0) throw new HttpRefusal(400, "address");
  return address;
}
export function afterId(url: URL): number | null {
  const parameter = url.searchParams.get("after");
  if (parameter === null) return null;
  const after = Number(parameter);
  if (!/^\d+$/.test(parameter) || !Number.isInteger(after) || after < 0 || after > 0xffffffff)
    throw new HttpRefusal(400, "invalid");
  return after;
}

/** Attach before committing headers so a refusal is still an ordinary JSON reply. */
export function stream(
  response: ServerResponse,
  attach: (deliver: Parameters<RecordSource["attach"]>[1]) => () => void,
) {
  let gone: (() => void) | undefined;
  response.once("close", () => gone?.());
  gone = attach((record) => {
    if (response.destroyed || response.writableEnded) return false;
    if (!response.headersSent) {
      response.writeHead(200, {
        "Content-Type": "application/octet-stream",
        "X-Accel-Buffering": "no",
      });
      response.flushHeaders();
    }
    response.write(encodeRecord(record));
    if (response.writableLength > SOCKET_QUEUE_BYTES) {
      response.destroy();
      return false;
    }
    if (record.kind === "end") response.end();
    return true;
  });
  if (response.destroyed) gone();
}
