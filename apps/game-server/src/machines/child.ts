import { fork } from "node:child_process";
import type { BattleMachines, Machine } from "./index.ts";
import { battleCapacity } from "../online.ts";

/** The credential travels only in the child's environment, never argv or IPC. */
export function childBattleMachines(
  entry = new URL("./battle.mjs", import.meta.url),
): BattleMachines {
  const capacity = battleCapacity();
  let running = 0;
  return {
    capacity,
    start(key) {
      if (running >= capacity) return Promise.reject(new Error("battle capacity exhausted"));
      running++;
      return new Promise<Machine>((resolve, reject) => {
        const child = fork(entry, [], {
          env: { ...process.env, BATTLE_KEY: key, HOST: "127.0.0.1", PORT: "0" },
          stdio: ["ignore", "ignore", "pipe", "ipc"],
        });
        child.stderr?.on("data", (chunk) => {
          process.stderr.write(chunk);
        });
        let allocated = true;
        const release = () => {
          if (allocated) {
            allocated = false;
            running--;
          }
        };
        let killing: ReturnType<typeof setTimeout> | undefined;
        const stop = () => {
          if (killing || child.exitCode !== null || child.signalCode !== null) return;
          release();
          child.kill("SIGTERM");
          killing = setTimeout(() => child.kill("SIGKILL"), 5000);
          killing.unref();
        };
        const timer = setTimeout(() => {
          stop();
          reject(new Error("battle server startup failed"));
        }, 15_000);
        child.once("error", () => {
          release();
          clearTimeout(timer);
          reject(new Error("battle server startup failed"));
        });
        child.once("exit", () => {
          release();
          clearTimeout(killing);
          clearTimeout(timer);
          reject(new Error("battle server exited"));
        });
        child.once("message", (message: { address?: string }) => {
          if (!message.address) return;
          clearTimeout(timer);
          resolve({ address: message.address, stop });
        });
      });
    },
  };
}
