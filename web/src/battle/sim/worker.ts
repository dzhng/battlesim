/** Worker entry: hosts the authority with the worker's clock, one
 * earliest-deadline timer and its message channel. */
import { createAuthority } from "./authority";
import { loadSimModule } from "./module";
import type { SimReply, SimRequest } from "./protocol";

interface WorkerScope {
  postMessage(message: SimReply, options?: { transfer?: Transferable[] }): void;
  addEventListener(type: "message", listener: (event: MessageEvent<SimRequest>) => void): void;
  close(): void;
}
const scope = self as unknown as WorkerScope;

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
  load: loadSimModule,
});

scope.addEventListener("message", (event) => authority.handle(event.data));
