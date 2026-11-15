// One HTTP connection per records() call. resumingReader, above this seam,
// decides when to reconnect and which id to carry to the next connection.
import type { SeatLink } from "../../../../packages/multiplayer/src/link";
import { httpRecords } from "../../../../packages/multiplayer/src/stream/http";

/** A missing ghost answer must give the next cursor intent a turn. */
const PREVIEW_TIMEOUT_MS = 2000;

export function httpLink(address: string, battle: string, browserId: string): SeatLink {
  const base = `${address.replace(/\/$/, "")}/battles/${encodeURIComponent(battle)}`;
  const lifetime = new AbortController();
  let orderRequest: AbortController | undefined;
  async function post<T>(action: string, value: unknown, signal: AbortSignal): Promise<T> {
    const response = await fetch(`${base}/${action}`, {
      method: "POST",
      headers: { "X-Player": browserId, "Content-Type": "application/json" },
      body: JSON.stringify(value),
      signal: AbortSignal.any([lifetime.signal, signal]),
    });
    const reply = await response.json();
    // A seat/rate refusal is part of UpReply; route failures are not.
    if (
      !response.ok &&
      !(action === "up" && (reply.refused === "not_seated" || reply.refused === "rate"))
    )
      throw new Error(`Battle request failed (${response.status})`);
    return reply as T;
  }
  return {
    records(after, signal = new AbortController().signal) {
      return httpRecords(
        `${base}/stream${after === null ? "" : `?after=${after}`}`,
        browserId,
        AbortSignal.any([lifetime.signal, signal]),
      );
    },
    up(batch) {
      // The connection owns retry timing; replacing an attempt also closes
      // its old socket, so a missing reply cannot accumulate requests.
      orderRequest?.abort();
      orderRequest = new AbortController();
      return post("up", batch, orderRequest.signal);
    },
    preview: (query) => post("preview", query, AbortSignal.timeout(PREVIEW_TIMEOUT_MS)),
    close: () => lifetime.abort(),
  };
}
