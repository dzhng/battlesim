import { BROWSER_ID_HEADER } from "../../../../packages/multiplayer/src/identity";
// One HTTP connection per records() call. resumingReader, above this seam,
// decides when to reconnect and which id to carry to the next connection.
import type { SeatLink } from "../../../../packages/multiplayer/src/link";
import { httpRecords } from "../../../../packages/multiplayer/src/stream/http";
import type { UpBatch } from "../../../../packages/multiplayer/src/wire";

/** A missing ghost answer must give the next cursor intent a turn. */
const PREVIEW_TIMEOUT_MS = 2000;

export function httpLink(address: string, battle: string, browserId: string): SeatLink {
  const base = `${address.replace(/\/$/, "")}/battles/${encodeURIComponent(battle)}`;
  const lifetime = new AbortController();
  let orderRequest: AbortController | undefined;
  async function post<T>(action: string, value: unknown, signal: AbortSignal): Promise<T> {
    const response = await fetch(`${base}/${action}`, {
      method: "POST",
      headers: { [BROWSER_ID_HEADER]: browserId, "Content-Type": "application/json" },
      body: JSON.stringify(value),
      signal: AbortSignal.any([lifetime.signal, signal]),
    });
    const reply = await response.json();
    if (response.ok) return reply as T;
    // A refusal is the batch's answer whether or not it names the batch: its
    // orders are refused once. Only a reply that did not arrive is sent again.
    if (action === "up" && typeof reply?.refused === "string")
      return { seq: (value as UpBatch).seq, refused: reply.refused } as T;
    throw new Error(`Battle request failed (${response.status})`);
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
