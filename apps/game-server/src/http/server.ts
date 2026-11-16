import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { json } from "./io.ts";
import { HttpRefusal } from "./seats.ts";

/** Common HTTP boundary. Exception text and request headers never leave it. */
export function httpServer(
  route: (request: IncomingMessage, response: ServerResponse, url: URL) => Promise<void>,
) {
  return createServer((request, response) => {
    response.setHeader("Cache-Control", "no-store, no-transform");
    response.setHeader("Access-Control-Allow-Origin", "*");
    response.setHeader("Access-Control-Allow-Headers", "X-Player, Content-Type");
    response.setHeader("Access-Control-Max-Age", "7200");
    response.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
    if (request.method === "OPTIONS") {
      response.writeHead(204);
      response.end();
      return;
    }
    void Promise.resolve()
      .then(() => route(request, response, new URL(request.url!, "http://localhost")))
      .catch((error: unknown) => {
        if (response.headersSent) response.destroy();
        else if (error instanceof HttpRefusal)
          json(response, error.status, { refused: error.reason });
        else json(response, 410, { refused: "unavailable" });
      });
  });
}
