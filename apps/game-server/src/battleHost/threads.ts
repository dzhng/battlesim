// Bounded speculative preparation. Only the chosen worker receives requests;
// an admitted candidate waiting for lower answers owns a world, not an authority.
import { availableParallelism } from "node:os";
import { Worker } from "node:worker_threads";
import { battleCapacity } from "../online.ts";
import type { BattleSpec } from "./index.ts";
import { chooseCandidate, type CandidateAnswer } from "./candidates.ts";
import type { BattleData, HostRequest, RuntimeMessage } from "./runtime.ts";
import type { WorkerInit, WorkerReply, WorkerRequest } from "./worker.ts";

export function prepareOnThreads(options: {
  spec: BattleSpec;
  data: BattleData;
  post(message: RuntimeMessage): void;
  crash(error: unknown): void;
}) {
  // A battle already has one reserved place. Extra preparations consume the
  // same CPU/memory budget as running battles, using currently available memory
  // (including the process/container limit), and leaving the event loop a core.
  const capacity = Math.max(1, battleCapacity(availableParallelism(), process.availableMemory()));
  const { candidates } = options.spec;
  const answers: CandidateAnswer[] = candidates.map(() => "pending");
  const workers = new Map<number, Worker>();
  const stopping = new Map<number, Promise<unknown>>();
  const backlog: HostRequest[] = [];
  let next = 0;
  let winner: number | null = null;
  let running = false;
  let disposed = false;

  function stopCandidate(candidate: number) {
    const existing = stopping.get(candidate);
    if (existing) return existing;
    const worker = workers.get(candidate);
    if (!worker) return Promise.resolve();
    const stopped = worker.terminate().then(() => {
      workers.delete(candidate);
      stopping.delete(candidate);
    });
    stopping.set(candidate, stopped);
    return stopped;
  }
  function stop() {
    disposed = true;
    backlog.length = 0;
    for (const candidate of workers.keys()) void stopCandidate(candidate);
  }
  function refuse(candidate: number) {
    if (disposed || stopping.has(candidate)) return;
    if (winner === candidate) {
      if (answers[candidate] === "admitted") return options.crash("the battle's thread exited");
      winner = null;
      running = false;
    }
    answers[candidate] = "refused";
    // Free the slot only once termination has completed: replacements must
    // not overlap the memory of a worker that is still stopping.
    void stopCandidate(candidate).then(advance);
    advance();
  }
  function launch(candidate: number) {
    try {
      const worker = new Worker(new URL("./worker.mjs", import.meta.url));
      workers.set(candidate, worker);
      worker.on("message", (message: WorkerReply) => {
        if (disposed || stopping.has(candidate)) return;
        if (message.type === "candidate-refused") return refuse(candidate);
        if (message.type === "candidate-prepared") {
          answers[candidate] = "admitted";
          advance();
        } else if (winner === candidate) {
          if (message.type === "prepared") answers[candidate] = "admitted";
          options.post(message);
        }
      });
      worker.on("error", () => refuse(candidate));
      worker.on("exit", () => refuse(candidate));
      // No credentials or functions enter a candidate's thread.
      const { presets, templates, limits, rules, root } = options.data;
      worker.postMessage({
        type: "battle-host-init",
        setup: candidates[candidate],
        candidate,
        data: { presets, templates, limits, rules, root },
        start: candidates.length === 1,
      } satisfies WorkerInit);
    } catch {
      refuse(candidate);
    }
  }
  function advance() {
    if (disposed || winner !== null) return;
    const chosen = chooseCandidate(answers);
    if (chosen !== null) {
      if (chosen < 0) {
        void Promise.all([...workers.keys()].map(stopCandidate)).then(() => {
          if (!disposed) options.crash("no battle candidate could be prepared");
        });
        return;
      }
      winner = chosen;
      void Promise.all([...workers.keys()].filter((i) => i !== chosen).map(stopCandidate)).then(
        () => {
          if (disposed) return;
          workers.get(chosen)!.postMessage({ type: "battle-host-start" } satisfies WorkerRequest);
          running = true;
          for (const request of backlog.splice(0)) workers.get(chosen)!.postMessage(request);
        },
      );
      return;
    }
    // Never launch above an already admitted world: it cannot beat that one.
    const admitted = answers.indexOf("admitted");
    while (
      workers.size < capacity &&
      next < candidates.length &&
      (admitted < 0 || next < admitted)
    ) {
      launch(next++);
    }
  }
  // A room keeps the original one-thread path: no speculative workers, no
  // parent round trip between preparation and starting the authority.
  if (candidates.length === 1) {
    winner = 0;
    running = true;
    launch(0);
  } else advance();
  return {
    send(request: HostRequest) {
      if (disposed) return;
      if (running) workers.get(winner!)?.postMessage(request);
      else backlog.push(request);
    },
    stop,
  };
}
