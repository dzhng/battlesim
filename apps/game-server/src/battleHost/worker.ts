// The candidate's thread retains its world until chosen, then runs the battle.
import { parentPort } from "node:worker_threads";
import type { BattleSetup } from "../../../../packages/multiplayer/src/wire.ts";
import {
  createBattleRuntime,
  prepareBattle,
  type BattleData,
  type HostRequest,
  type PreparedBattle,
  type RuntimeMessage,
} from "./runtime.ts";

export type WorkerInit = {
  type: "battle-host-init";
  setup: BattleSetup;
  candidate: number;
  data: BattleData;
  /** A sole candidate needs no selection round trip. */
  start: boolean;
};
export type WorkerRequest = WorkerInit | HostRequest | { type: "battle-host-start" };
export type WorkerReply =
  | RuntimeMessage
  | { type: "candidate-prepared" }
  | { type: "candidate-refused" };

const port = parentPort;
if (!port) throw new Error("the battle worker runs only as a worker thread");
const runtime = createBattleRuntime({
  now: () => performance.now(),
  schedule: (run, delayMs) => void setTimeout(run, delayMs),
  post: (reply, transfer) => port.postMessage(reply, transfer ?? []),
  close: () => port.close(),
});
let prepared: PreparedBattle;
let candidate: number;
port.on("message", (message: WorkerRequest) => {
  if (message.type === "battle-host-start") {
    runtime.start(prepared, candidate);
  } else if (message.type === "battle-host-init") {
    candidate = message.candidate;
    void prepareBattle(message.setup, message.data).then(
      (world) => {
        prepared = world;
        if (message.start) runtime.start(prepared, candidate);
        else port.postMessage({ type: "candidate-prepared" } satisfies WorkerReply);
      },
      () => {
        // Preparation errors are candidate refusals. Exception text can quote
        // setup input; it never needs to cross the thread boundary.
        port.postMessage({ type: "candidate-refused" } satisfies WorkerReply);
        port.close();
      },
    );
  } else runtime.handle(message);
});
