// A fresh WASM realm makes linear-memory growth independent of other tests.
import { readFileSync } from "node:fs";
import { parentPort, workerData } from "node:worker_threads";
const { default: init, Fielding } = await import(workerData.module);
const { memory } = await init({ module_or_path: readFileSync(new URL(workerData.wasm)) });
const fielding = new Fielding(...workerData.input);
const sources = new Map(workerData.sources);
try {
  const before = memory.buffer.byteLength;
  const pending = JSON.parse(fielding.pending_contents());
  const hashesOf = ({ bundle, skeleton }) => [bundle, ...(skeleton ? [skeleton] : [])];
  const uses = new Map();
  for (const content of pending)
    for (const hash of hashesOf(content)) uses.set(hash, (uses.get(hash) ?? 0) + 1);
  for (const content of pending) {
    const hashes = hashesOf(content);
    const files = hashes.map((hash) => sources.get(hash));
    const lengths = Uint32Array.from(files, (file) => file.length);
    const bytes = new Uint8Array(files.reduce((n, file) => n + file.length, 0));
    let at = 0;
    for (const file of files) {
      bytes.set(file, at);
      at += file.length;
    }
    try {
      fielding.absorb(JSON.stringify(content), bytes, lengths);
    } finally {
      bytes.buffer.transfer(0);
    }
    for (const hash of hashes) {
      const remaining = uses.get(hash) - 1;
      uses.set(hash, remaining);
      if (!remaining) {
        sources.get(hash).buffer.transfer(0);
        sources.delete(hash);
      }
    }
  }
  const first = fielding.resolve();
  const after = memory.buffer.byteLength;
  first.free();
  const missing = JSON.parse(fielding.pending_contents());
  const warm = fielding.resolve();
  parentPort.postMessage({ before, after, missing, work: JSON.parse(warm.work()) });
  warm.free();
} finally {
  for (const source of sources.values()) source.buffer.transfer(0);
  fielding.free();
  parentPort.close();
}
