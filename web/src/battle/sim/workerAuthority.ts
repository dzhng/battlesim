/** Worker entry: hosts the authority with the worker's clock, one
 * earliest-deadline timer and its message channel. */
import { createAuthority, type AuthorityHost } from "./authority";
import type { SimReply } from "./protocol";

export interface WorkerScope {
  postMessage(message: SimReply, options?: { transfer?: Transferable[] }): void;
  close(): void;
}
export function workerAuthority(scope: WorkerScope, load: AuthorityHost["load"]) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let timerAt = Infinity;

  const authority = createAuthority({
    post: (reply, transfer) => scope.postMessage(reply, { transfer: transfer ?? [] }),
    now: () => performance.now(),
    schedule(delayMs) {
      const at = performance.now() + delayMs;
      if (timer !== null && at >= timerAt) return;
      if (timer !== null) clearTimeout(timer);
      timerAt = at;
      timer = setTimeout(() => {
        timer = null;
        timerAt = Infinity;
        authority.pump();
      }, delayMs);
    },
    close() {
      if (timer !== null) clearTimeout(timer);
      timer = null;
      scope.close();
    },
    load,
  });

  return authority;
}
