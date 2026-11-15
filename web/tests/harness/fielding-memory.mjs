// A fresh WASM realm makes linear-memory growth independent of other tests.
import { readFileSync } from "node:fs";
import { parentPort, workerData } from "node:worker_threads";
const { default: init, Fielding } = await import(workerData.module);
const { memory } = await init({ module_or_path: readFileSync(new URL(workerData.wasm)) });
const fielding = new Fielding(...workerData.input);
try {
  const before = memory.buffer.byteLength;
  const first = fielding.resolve(workerData.bytes, workerData.lengths);
  const after = memory.buffer.byteLength;
  const missing = fielding.contents();
  const warm = fielding.resolve(new Uint8Array(), new Uint32Array());
  parentPort.postMessage({ before, after, missing, same: first === warm });
} finally {
  fielding.free();
  parentPort.close();
}
