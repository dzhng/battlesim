import { createHash } from "node:crypto";
import type { IncomingMessage, ServerResponse } from "node:http";

/** A refusal the workbench reports to the page with its HTTP status. */
export class WorkbenchError extends Error {
  constructor(
    message: string,
    public status = 400,
  ) {
    super(message);
  }
}
/** A plain request object holding only `fields`. */
export function object(value: unknown, fields: string[]): Record<string, unknown> {
  if (
    !value ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.keys(value).some((key) => !fields.includes(key))
  )
    throw new WorkbenchError("Unexpected request fields");
  return value as Record<string, unknown>;
}
export const hash = (bytes: string | Buffer) => createHash("sha256").update(bytes).digest("hex");
export const equal = (a: unknown, b: unknown): boolean => {
  if (a === b) return true;
  if (
    !a ||
    !b ||
    typeof a !== "object" ||
    typeof b !== "object" ||
    Array.isArray(a) !== Array.isArray(b)
  )
    return false;
  const keys = Object.keys(a);
  return (
    keys.length === Object.keys(b).length &&
    keys.every(
      (key) =>
        Object.hasOwn(b, key) &&
        equal((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]),
    )
  );
};

/** The workbench answers only this machine, and writes only for its own page. */
export function localOnly(request: IncomingMessage): void {
  const host = request.headers.host ?? "";
  if (
    !/^(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?$/.test(host) ||
    !["127.0.0.1", "::1", "::ffff:127.0.0.1"].includes(request.socket.remoteAddress ?? "")
  )
    throw new WorkbenchError("The sound workbench is local-only", 403);
}

/** A write's JSON body, from this workbench's own page and within the size limit. */
export async function writeBody(request: IncomingMessage): Promise<unknown> {
  if (
    request.headers.origin !== `http://${request.headers.host}` ||
    !/^application\/json(?:\s*;|$)/i.test(request.headers["content-type"] ?? "")
  )
    throw new WorkbenchError("Requests must come from this local workbench", 403);
  const chunks: Buffer[] = [];
  let bytes = 0;
  for await (const chunk of request) {
    const part = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    bytes += part.length;
    if (bytes > 1024 * 1024)
      throw new WorkbenchError("Request body exceeds the workbench limit", 413);
    chunks.push(part);
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf8"));
  } catch {
    throw new WorkbenchError("Expected a JSON request object");
  }
}

/**
 * Serves one local-only JSON route table: `GET` names read, `POST` names write.
 * Every answer is uncached JSON, and a failure is `{ error }` with its status.
 */
export function jsonRoutes(
  prefix: string,
  reads: Record<string, () => Promise<unknown>>,
  writes: Record<string, (body: unknown) => Promise<unknown>>,
) {
  return async (request: IncomingMessage, response: ServerResponse, next: () => void) => {
    const path = request.url?.split("?")[0];
    if (!path?.startsWith(prefix)) return next();
    const name = path.slice(prefix.length);
    response.setHeader("Content-Type", "application/json");
    response.setHeader("Cache-Control", "no-store");
    try {
      localOnly(request);
      let result;
      if (request.method === "GET" && Object.hasOwn(reads, name)) result = await reads[name]();
      else if (request.method === "POST" && Object.hasOwn(writes, name))
        result = await writes[name](await writeBody(request));
      else throw new WorkbenchError("Unknown sound workbench endpoint", 404);
      response.end(JSON.stringify(result));
    } catch (error) {
      response.statusCode =
        error instanceof WorkbenchError ? error.status : error instanceof SyntaxError ? 400 : 500;
      response.end(
        JSON.stringify({ error: error instanceof Error ? error.message : String(error) }),
      );
    }
  };
}
