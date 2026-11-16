// The test's worker-loader seam holds setup delivery behind a gate. Production
// worker/runtime code is bundled separately and receives the original setup.
import { parentPort, workerData } from "node:worker_threads";
const port = parentPort!;
port.once("message", async (message) => {
  port.postMessage({ type: "test-setup-waiting" });
  const gate = new Int32Array(workerData.gate);
  Atomics.wait(gate, 0, 0, workerData.delayMs);
  if (Atomics.load(gate, 0) === 2) process.exit(1);
  await import(workerData.entry);
  port.emit("message", message);
});
