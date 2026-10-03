import type { IncomingMessage, ServerResponse } from "node:http";
import type { WorkbenchStore } from "./server";
import { WorkbenchError } from "./error";
import type { Draft, RunRequest, SaveReview } from "./src/protocol";

const namespace = "/__map-workbench";
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new WorkbenchError("Expected a request object");
  return value as Record<string, unknown>;
}
function artifact(body: Record<string, unknown>): string {
  if (typeof body.artifactId !== "string" || !/^[0-9a-f-]{36}$/.test(body.artifactId))
    throw new WorkbenchError("Expected an artifact ID");
  return body.artifactId;
}

export function workbenchMiddleware(store: WorkbenchStore) {
  return async (request: IncomingMessage, response: ServerResponse, next: () => void) => {
    const path = request.url?.split("?")[0];
    if (path !== namespace && !path?.startsWith(`${namespace}/`)) return next();
    response.setHeader("Content-Type", "application/json");
    response.setHeader("Cache-Control", "no-store");
    const controller = new AbortController();
    const cancel = () => {
      if (!response.writableEnded) controller.abort();
    };
    response.once("close", cancel);
    request.once("aborted", cancel);
    try {
      const host = request.headers.host ?? "";
      if (
        !/^(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?$/.test(host) ||
        !["127.0.0.1", "::1", "::ffff:127.0.0.1"].includes(request.socket.remoteAddress ?? "")
      )
        throw new WorkbenchError("The map workbench is local-only", 403);
      if (request.method === "GET" && path === `${namespace}/snapshot`) {
        response.end(JSON.stringify(await store.snapshot(controller.signal)));
        return;
      }
      if (request.method !== "POST") throw new WorkbenchError("Unknown workbench endpoint", 404);
      if (
        request.headers.origin !== `http://${host}` ||
        !/^application\/json(?:\s*;|$)/i.test(request.headers["content-type"] ?? "")
      )
        throw new WorkbenchError("Requests must come from this local workbench", 403);
      if (Number(request.headers["content-length"] ?? 0) > 4 * 1024 * 1024)
        throw new WorkbenchError("Request body exceeds the workbench limit", 413);
      const chunks: Buffer[] = [];
      let bytes = 0;
      for await (const chunk of request) {
        const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
        bytes += buffer.length;
        if (bytes > 4 * 1024 * 1024)
          throw new WorkbenchError("Request body exceeds the workbench limit", 413);
        chunks.push(buffer);
      }
      let body: Record<string, unknown>;
      try {
        body = object(JSON.parse(Buffer.concat(chunks).toString("utf8")));
      } catch {
        throw new WorkbenchError("Expected a JSON request object");
      }
      if (Object.hasOwn(body, "artifactDir") || Object.hasOwn(body, "path"))
        throw new WorkbenchError("Browser requests cannot name filesystem paths");
      const signal = controller.signal;
      let result: unknown;
      switch (path) {
        case `${namespace}/generate`:
          result = await store.generate(body as unknown as RunRequest, signal);
          break;
        case `${namespace}/inspect`:
          result = await store.inspect(artifact(body), body.crop as string | undefined, signal);
          break;
        case `${namespace}/sight`:
          result = await store.sight(artifact(body), signal);
          break;
        case `${namespace}/preview`:
          result = await store.preview(body as unknown as Draft, signal);
          break;
        case `${namespace}/save`:
          result = await store.save(body as unknown as SaveReview, signal);
          break;
        case `${namespace}/export`:
          result = await store.export(artifact(body), signal);
          break;
        default:
          throw new WorkbenchError("Unknown workbench endpoint", 404);
      }
      if (!response.destroyed) response.end(JSON.stringify(result));
    } catch (error) {
      if (!response.destroyed) {
        response.statusCode = error instanceof WorkbenchError ? error.status : 500;
        response.end(
          JSON.stringify({
            error: error instanceof Error ? error.message : String(error),
            ...(error instanceof WorkbenchError && { diagnostics: error.diagnostics }),
          }),
        );
      }
    } finally {
      response.removeListener("close", cancel);
      request.removeListener("aborted", cancel);
    }
  };
}
