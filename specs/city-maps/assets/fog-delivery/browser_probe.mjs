import { createRequire } from "node:module";
import { createServer } from "node:http";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
const root = process.cwd();
const { chromium } = createRequire(path.join(root, "web/package.json"))("playwright");
const oracle = JSON.parse(
  await readFile(path.join(root, "specs/city-maps/assets/fog-delivery/oracle.json"), "utf8"),
);
const files = {
  "game_wasm.js": "web/src/wasm/game_wasm.js",
  "game_wasm_bg.wasm": "web/src/wasm/game_wasm_bg.wasm",
  "observation.js": "throwaway/sa4/observation.js",
  "authority.js": "throwaway/sa4/authority.js",
};
const server = createServer(async (req, res) => {
  try {
    const name = req.url.slice(1);
    res.setHeader(
      "Content-Type",
      name.endsWith(".wasm")
        ? "application/wasm"
        : name.endsWith(".js")
          ? "text/javascript"
          : "text/html",
    );
    res.end(
      files[name]
        ? await readFile(path.join(root, files[name]))
        : "<!doctype html><title>Fog delivery capability probe</title>",
    );
  } catch (error) {
    res.statusCode = 500;
    res.end(String(error));
  }
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
let browser;
try {
  browser = await chromium.launch({ channel: "chrome", headless: true });
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result = await page.evaluate(async (rules) => {
    const { ObservationDecoder } = await import("/observation.js");
    const results = [];
    let largestLayout;
    for (const size of [12000, 15000, 18000]) {
      // S0's admitted empty-Battle peak plus cursor/copies is <600 MB; no terrain mesh or GroundView is built.
      const workerCode = `import {createAuthority} from '${location.origin}/authority.js';import init,* as wasm from '${location.origin}/game_wasm.js';let memory;const metrics={};const timed=(name,fn)=>{const start=performance.now();const out=fn();metrics[name]=performance.now()-start;return out};const authority=createAuthority({post:(reply,transfer)=>self.postMessage({...reply,probe:{...metrics,wasmBytes:memory?.buffer.byteLength}},transfer??[]),now:()=>0,schedule:()=>{},close:()=>self.close(),load:async()=>{memory=(await init()).memory;return {memory,createBattle:(s,seed)=>timed('constructorMs',()=>{const b=new wasm.Battle(s,seed);const publish=b.publish.bind(b);b.publish=side=>timed('publishMs',()=>publish(side));return b}),replayBattle:()=>{throw new Error('unused')}}}});self.onmessage=({data})=>authority.handle(data);`;
      const url = URL.createObjectURL(new Blob([workerCode], { type: "text/javascript" }));
      const worker = new Worker(url, { type: "module" });
      const inbox = [];
      let waiter = null;
      worker.onmessage = ({ data }) => {
        if (waiter) {
          const w = waiter;
          waiter = null;
          w(data);
        } else inbox.push(data);
      };
      const next = () =>
        inbox.length
          ? Promise.resolve(inbox.shift())
          : new Promise((resolve) => (waiter = resolve));
      const receive = async (type) => {
        while (true) {
          const r = await next();
          if (r.type === "error") throw new Error(r.message);
          if (r.type === type) return r;
        }
      };
      try {
        worker.postMessage({
          type: "init",
          scenario: JSON.stringify({
            map: { size: [size, size], height_grid_m: 4, slope_cutoff_deg: 50 },
            rules,
            units: [],
          }),
          seed: 1,
          side: "blue",
        });
        const ready = await receive("ready");
        largestLayout = JSON.parse(ready.layout);
        const decoder = new ObservationDecoder(largestLayout);
        const frames = [];
        const rows = [];
        worker.postMessage({ type: "start" });
        worker.postMessage({ type: "pause" });
        for (let i = 0; i < 5; i++) {
          if (i) {
            if (i === 2) worker.postMessage({ type: "side", side: "red" });
            if (i === 4) worker.postMessage({ type: "side", side: "blue" });
            worker.postMessage({ type: "advance", id: i, ticks: 1 });
          }
          const packet = await receive("publication");
          const start = performance.now();
          const frame = decoder.decode(new Float32Array(packet.buffer, 0, packet.length));
          const decodeMs = performance.now() - start;
          if (
            frame.tick !== i + 1 ||
            frame.fog.cellM !== 8 ||
            frame.fog.nx !== Math.ceil(size / 8) ||
            frame.fog.ny !== Math.ceil(size / 8) ||
            frame.fog.bits.some(Boolean)
          )
            throw new Error("full empty fog/tick reconstruction mismatch");
          frames.push(frame);
          rows.push({
            tick: packet.tick,
            bytes: packet.length * 4,
            digest: packet.digest,
            decodeMs,
            ...packet.probe,
          });
          worker.postMessage({ type: "credit", buffer: packet.buffer }, [packet.buffer]);
        }
        if (frames.some((f) => f.fog.bits.some(Boolean))) throw new Error("retained fog mutated");
        results.push({
          size_m: size,
          constructorMs: ready.probe.constructorMs,
          rows,
          retainedFogBytes:
            new Set(frames.map((f) => f.fog.bits)).size * frames[0].fog.bits.byteLength,
        });
      } finally {
        worker.terminate();
        URL.revokeObjectURL(url);
      }
    }
    // Exercise the actual maximum atomic codec payload without allocating a dense 2 m Battle.
    const words = largestLayout.fog.maxWords;
    const decoder = new ObservationDecoder(largestLayout);
    const make = (full, base, revision, payload) => {
      const data = new Float32Array(largestLayout.header.length + payload.length);
      const h = {
        tick: revision,
        fogCellM: 2,
        fogNx: 9000,
        fogNy: 9000,
        fogFloats: payload.length,
        fogFull: Number(full),
        fogBaseLo: base,
        fogRevisionLo: revision,
        groundEpoch: 1,
        groundSide: 0,
        groundFull: Number(base === 0),
      };
      largestLayout.header.forEach((key, i) => (data[i] = h[key] ?? 0));
      data.set(payload, largestLayout.header.length);
      return data;
    };
    const wire = make(true, 0, 1, new Float32Array(2 * words));
    let start = performance.now();
    const first = decoder.decode(wire);
    const fullDecodeMs = performance.now() - start;
    const delta = make(false, 1, 2, [words - 1, 0, 32768]);
    start = performance.now();
    const second = decoder.decode(delta);
    const deltaDecodeMs = performance.now() - start;
    if (
      first.fog.bits.length !== words ||
      first.fog.bits[words - 1] !== 0 ||
      second.fog.bits[words - 1] !== 0x80000000
    )
      throw new Error("maximum snapshot/index/lifetime mismatch");
    const replacement = make(true, 2, 3, new Float32Array(2 * words));
    const third = decoder.decode(replacement);
    if (third.fog.bits[words - 1] !== 0 || second.fog.bits[words - 1] !== 0x80000000)
      throw new Error("maximum replacement lifetime mismatch");
    return {
      rows: results,
      maximumCodec: {
        wireBytes: wire.byteLength,
        words,
        fogBytes: first.fog.bits.byteLength,
        deltaWireBytes: delta.byteLength,
        fullDecodeMs,
        deltaDecodeMs,
      },
    };
  }, oracle.scenario.rules);
  await writeFile(
    path.join(root, "specs/city-maps/assets/fog-delivery/browser.json"),
    JSON.stringify({ browser: browser.version(), ...result }, null, 2) + "\n",
  );
  console.log(
    JSON.stringify(
      result.rows.map((r) => ({
        size_m: r.size_m,
        wire: r.rows.map((r) => r.bytes),
        wasm: r.rows.at(-1).wasmBytes,
      })),
    ),
  );
} finally {
  await browser?.close();
  await new Promise((resolve) => server.close(resolve));
}
