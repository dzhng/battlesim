import { recordDecoder, StreamBroken } from "./index";
import type { StreamRecord } from "../wire";

/** A normal HTTP refusal is terminal; reconnecting would repeat the request. */
export class StreamRefused extends StreamBroken {
  constructor(readonly reason: string) {
    super("Stream refused");
  }
}

/** One HTTP response, shared by battle and state streams. Resume lives above it. */
export async function* httpRecords(
  url: string,
  browserId: string,
  signal: AbortSignal,
): AsyncGenerator<StreamRecord> {
  const connection = new AbortController();
  let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
  try {
    const response = await fetch(url, {
      headers: { "X-Player": browserId },
      signal: AbortSignal.any([signal, connection.signal]),
    });
    if (!response.ok) {
      const reply = (await response.json()) as { refused: string };
      throw new StreamRefused(reply.refused);
    }
    if (!response.body) throw new StreamBroken("Stream has no body");
    reader = response.body.getReader();
    const decoder = recordDecoder();
    for (;;) {
      const chunk = await reader.read();
      if (chunk.done) return;
      for (const record of decoder.push(chunk.value)) yield record;
    }
  } finally {
    connection.abort();
    await reader?.cancel().catch(() => {});
    reader?.releaseLock();
  }
}
