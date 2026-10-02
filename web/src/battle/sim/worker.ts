/** Worker entry for a scenario that needs no encounter preparation. */
import { loadSimModule } from "./module";
import { workerAuthority, type WorkerScope } from "./workerAuthority";
import type { SimRequest } from "./protocol";
const scope = self as unknown as WorkerScope & {
  addEventListener(type: "message", listener: (event: MessageEvent<SimRequest>) => void): void;
};
const authority = workerAuthority(scope, loadSimModule);
scope.addEventListener("message", (event) => authority.handle(event.data));
