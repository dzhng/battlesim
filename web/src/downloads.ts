// What the page is downloading, for the loading screen: a first visit pulls
// tens of megabytes (art bundles, the simulation module, the map) before
// anything can be drawn, and a number going up says it is not frozen.
// Downloads that should count go through `countedFetch`; the loading screen
// reads `downloads`. One store per realm: a worker's own fetches are not
// counted here.

export interface DownloadProgress {
  /** Downloads in flight. */
  active: number;
  /** Bytes received so far, of the downloads in this burst. */
  loaded: number;
  /** Their announced sizes, where announced (0 when none is). */
  total: number;
}

const IDLE: DownloadProgress = { active: 0, loaded: 0, total: 0 };
let now: DownloadProgress = IDLE;
const listeners = new Set<() => void>();

function set(next: DownloadProgress) {
  // A burst ends when its last download does; the next one counts afresh.
  now = next.active === 0 ? IDLE : next;
  for (const l of listeners) l();
}

/** The download counter, for `useSyncExternalStore`. */
export const downloads = {
  get: (): DownloadProgress => now,
  subscribe(listener: () => void): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};

/** `fetch`, its body counted into `downloads` as it arrives. */
export async function countedFetch(input: RequestInfo | URL, init?: RequestInit) {
  let response: Response;
  try {
    response = await fetch(input, init);
  } catch (error) {
    // Chromium private windows can reject a successful response when its
    // HTTP-cache write fails. The bytes are useful without persistence, so
    // retry once with caching disabled before reporting a network failure.
    if (
      init?.cache === "no-store" ||
      (error instanceof DOMException && error.name === "AbortError") ||
      !(error instanceof TypeError)
    )
      throw error;
    response = await fetch(input, { ...init, cache: "no-store" });
  }
  if (!response.ok || !response.body) return response;
  const size = Number(response.headers.get("content-length")) || 0;
  set({ active: now.active + 1, loaded: now.loaded, total: now.total + size });
  let done = false;
  const finish = () => {
    if (done) return;
    done = true;
    set({ ...now, active: now.active - 1 });
  };
  // A download ends however its body does: read whole, dropped by the
  // network, or cancelled by its reader. Each one ends the count.
  const source = response.body.getReader();
  const counted = new ReadableStream<Uint8Array>({
    async pull(controller) {
      // A rejected pull errors this stream with the same error.
      const chunk = await source.read().catch((error: unknown) => {
        finish();
        throw error;
      });
      if (chunk.done) {
        finish();
        controller.close();
      } else {
        set({ ...now, loaded: now.loaded + chunk.value.byteLength });
        controller.enqueue(chunk.value);
      }
    },
    cancel(reason) {
      finish();
      return source.cancel(reason);
    },
  });
  return new Response(counted, {
    status: response.status,
    statusText: response.statusText,
    headers: response.headers,
  });
}
