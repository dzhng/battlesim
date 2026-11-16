import { RequestRefused } from "../refusal";
import { BROWSER_ID_HEADER } from "../identity";
import { recordDecoder, StreamBroken } from "./index";
import type { StreamRecord } from "../wire";

/** One HTTP response, shared by battle and state streams. Resume lives above it. */
export async function* httpRecords(
  url: string,
  credential: string | Record<string, string>,
  signal: AbortSignal,
): AsyncGenerator<StreamRecord> {
  const connection = new AbortController();
  let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
  try {
    const response = await fetch(url, {
      headers: typeof credential === "string" ? { [BROWSER_ID_HEADER]: credential } : credential,
      signal: AbortSignal.any([signal, connection.signal]),
    });
    if (!response.ok) {
      const reply = (await response.json()) as { refused: string };
      throw new RequestRefused(reply.refused);
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
