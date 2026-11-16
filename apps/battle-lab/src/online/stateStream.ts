import { httpRecords } from "@packages/multiplayer/src/stream/http";
import { readJson, resumingReader } from "@packages/multiplayer/src/stream/index";
import { browserId } from "./identity";

/** A page retains only the latest view. Framing and reconnect policy are shared. */
export function stateStream<T>(
  address: string,
  path: string,
  signal: AbortSignal,
  receive: (view: T) => void,
  ended: (reason: string) => void,
  failed: (error: unknown) => void,
): void {
  const sleep = (milliseconds: number) =>
    new Promise<void>((resolve, reject) => {
      if (signal.aborted) {
        reject(signal.reason);
        return;
      }
      const abort = () => {
        clearTimeout(timer);
        reject(signal.reason);
      };
      const timer = setTimeout(() => {
        signal.removeEventListener("abort", abort);
        resolve();
      }, milliseconds);
      signal.addEventListener("abort", abort, { once: true });
    });
  void (async () => {
    for await (const record of resumingReader(
      (after) =>
        httpRecords(
          `${address}${path}${after === null ? "" : `?after=${after}`}`,
          browserId(),
          signal,
        ),
      { sleep },
    )) {
      if (signal.aborted) return;
      if (record.kind === "state") receive(readJson<T>(record));
      if (record.kind === "end") ended(readJson<{ reason: string }>(record).reason);
    }
  })().catch((error: unknown) => {
    if (!signal.aborted) failed(error);
  });
}

/** Opening a room joins it. Validate before leaving the list on a wrong code. */
export function joinRoom(address: string, code: string, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const opening = new AbortController();
    stateStream(
      address,
      `/rooms/${encodeURIComponent(code)}`,
      AbortSignal.any([signal, opening.signal]),
      () => {
        opening.abort();
        resolve();
      },
      () => {
        opening.abort();
        reject(new Error("Room closed."));
      },
      (error) => {
        opening.abort();
        reject(error);
      },
    );
  });
}
