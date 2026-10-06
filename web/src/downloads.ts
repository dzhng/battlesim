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
  const response = await fetch(input, init);
  if (!response.ok || !response.body) return response;
  const size = Number(response.headers.get("content-length")) || 0;
  set({ active: now.active + 1, loaded: now.loaded, total: now.total + size });
  let done = false;
  const finish = () => {
    if (done) return;
    done = true;
    set({ ...now, active: now.active - 1 });
  };
  const counted = response.body.pipeThrough(
    new TransformStream<Uint8Array, Uint8Array>({
      transform(chunk, controller) {
        set({ ...now, loaded: now.loaded + chunk.byteLength });
        controller.enqueue(chunk);
      },
      flush: finish,
    }),
  );
  return new Response(counted, {
    status: response.status,
    statusText: response.statusText,
    headers: response.headers,
  });
}
