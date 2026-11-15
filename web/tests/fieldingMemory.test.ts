// @vitest-environment node
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { Worker } from "node:worker_threads";
import { afterEach, expect, test, vi } from "vitest";
import { Fielding } from "@wasm/renderer_browser";
import { loadPresentation } from "@web/battle/sim/presentation";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import presentation from "@fixtures/presentation.json";
import { FieldingOwner, type RuntimeFiles } from "@web/battle/sim/fielding";

afterEach(() => vi.restoreAllMocks());

const runtime = new URL("../../assets/runtime/", import.meta.url);

test("fielding borrows source packets instead of copying the complete catalog into its byte cache", async () => {
  await loadPresentation();
  const set = await resolveCatalogSet("test");
  const catalog = JSON.parse(readFileSync(new URL("catalog.json", runtime), "utf8"));
  const input: ConstructorParameters<typeof Fielding> = [
    JSON.stringify(presentation),
    JSON.stringify(set.rules),
    JSON.stringify(set.units.view),
    JSON.stringify(catalog),
    JSON.stringify(["test_tank"]),
    undefined,
  ];
  const source = new Fielding(...input);
  const hashes = source.contents();
  source.free();
  // Unreferenced body padding is legal container data. It makes copying a whole
  // source observable without making its geometry or poses expensive to derive.
  const packets = hashes.map((hash) => {
    const file = gunzipSync(
      readFileSync(new URL(`${catalog.gzip[hash].hash}/bundle.bin`, runtime)),
    );
    const padding = 16 * 1024 * 1024;
    const bytes = new Uint8Array(file.length + padding);
    bytes.set(file);
    const header = new DataView(bytes.buffer);
    header.setUint32(12, header.getUint32(12, true) + padding, true);
    return bytes;
  });
  expect(packets.length).toBeGreaterThan(1);
  const lengths = Uint32Array.from(packets, (packet) => packet.length);
  const bytes = new Uint8Array(packets.reduce((n, packet) => n + packet.length, 0));
  let at = 0;
  for (const packet of packets) {
    bytes.set(packet, at);
    at += packet.length;
  }
  const worker = new Worker(new URL("./harness/fielding-memory.mjs", import.meta.url), {
    workerData: {
      input,
      bytes,
      lengths,
      module: new URL("../src/wasm/renderer_browser/renderer_browser.js", import.meta.url).href,
      wasm: new URL("../src/wasm/renderer_browser/renderer_browser_bg.wasm", import.meta.url).href,
    },
  });
  try {
    const result = await new Promise<{
      before: number;
      after: number;
      missing: string[];
      same: boolean;
    }>((resolve, reject) => {
      worker.once("message", resolve);
      worker.once("error", reject);
      worker.once("exit", (code) => {
        if (code !== 0) reject(new Error(`fielding memory worker exited ${code}`));
      });
    });
    // One concatenated boundary input plus at most one source's working space;
    // retaining another complete set of packet bytes exceeds this budget.
    expect(result.after - result.before).toBeLessThanOrEqual(bytes.length + Math.max(...lengths));
    expect(result.missing).toEqual([]);
    expect(result.same).toBe(true);
  } finally {
    await worker.terminate();
  }
}, 30000);

test("fielding releases its owned file and boundary packets after their last reader", async () => {
  await loadPresentation();
  const set = await resolveCatalogSet("test");
  const catalog = JSON.parse(readFileSync(new URL("catalog.json", runtime), "utf8"));
  const files: Uint8Array[] = [];
  const boundary: Uint8Array[] = [];
  const resolve = Fielding.prototype.resolve;
  vi.spyOn(Fielding.prototype, "resolve").mockImplementation(
    function (this: Fielding, bytes, lengths) {
      boundary.push(bytes);
      return resolve.call(this, bytes, lengths);
    },
  );
  const source: RuntimeFiles = {
    catalog,
    contents: async (hashes) =>
      hashes.map((hash) => {
        const packet = new Uint8Array(
          gunzipSync(readFileSync(new URL(`${catalog.gzip[hash].hash}/bundle.bin`, runtime))),
        );
        files.push(packet);
        return packet;
      }),
  };
  const owner = new FieldingOwner();
  try {
    const input = {
      rules: JSON.stringify(set.rules),
      catalog: set.units.view,
      kinds: ["test_tank"],
      runtime: source,
    };
    const first = await owner.field(input);
    expect(files.length).toBeGreaterThan(0);
    expect(files.map((file) => file.byteLength)).toEqual(files.map(() => 0));
    expect(boundary.map((packet) => packet.byteLength)).toEqual([0]);
    expect(await owner.field(input)).toBe(first);
  } finally {
    owner.free();
  }
});

test("fielding releases owned transport when a decoder refuses it", async () => {
  await loadPresentation();
  const set = await resolveCatalogSet("test");
  const catalog = JSON.parse(readFileSync(new URL("catalog.json", runtime), "utf8"));
  const files: Uint8Array[] = [];
  const boundary: Uint8Array[] = [];
  const resolve = Fielding.prototype.resolve;
  vi.spyOn(Fielding.prototype, "resolve").mockImplementation(
    function (this: Fielding, bytes, lengths) {
      boundary.push(bytes);
      return resolve.call(this, bytes, lengths);
    },
  );
  const source: RuntimeFiles = {
    catalog,
    contents: async (hashes) =>
      hashes.map((hash) => {
        const packet = new Uint8Array(
          gunzipSync(readFileSync(new URL(`${catalog.gzip[hash].hash}/bundle.bin`, runtime))),
        );
        packet[0] = 0;
        files.push(packet);
        return packet;
      }),
  };
  const owner = new FieldingOwner();
  try {
    const input = {
      rules: JSON.stringify(set.rules),
      catalog: set.units.view,
      kinds: ["test_tank"],
      runtime: source,
    };
    await expect(owner.field(input)).rejects.toThrow(/not a bundle of format/);
    expect(files.length).toBeGreaterThan(0);
    expect(files.map((file) => file.byteLength)).toEqual(files.map(() => 0));
    expect(boundary.map((packet) => packet.byteLength)).toEqual([0]);
  } finally {
    owner.free();
  }
});
