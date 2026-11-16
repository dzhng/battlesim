// The battle thread's entry: the runtime, with the parent port as its host.
import { parentPort } from "node:worker_threads";
import type { BattleSetup } from "../../../../packages/multiplayer/src/wire.ts";
import {
  createBattleRuntime,
  type BattleData,
  type BattleRuntime,
  type HostRequest,
} from "./runtime.ts";

export type WorkerInit = { type: "battle-host-init"; candidates: BattleSetup[]; data: BattleData };

const port = parentPort;
if (!port) throw new Error("the battle worker runs only as a worker thread");
let runtime: BattleRuntime | null = null;

port.on("message", (message: WorkerInit | HostRequest) => {
  if (message.type !== "battle-host-init") {
    runtime?.handle(message);
    return;
  }
  runtime = createBattleRuntime({
    candidates: message.candidates,
    data: message.data,
    now: () => performance.now(),
    schedule: (run, delayMs) => void setTimeout(run, delayMs),
    post: (reply, transfer) => port.postMessage(reply, transfer ?? []),
    close: () => port.close(),
  });
  runtime.start().catch((error: unknown) => {
    port.postMessage({
      type: "error",
      message: error instanceof Error ? error.message : String(error),
    });
    port.close();
  });
});
