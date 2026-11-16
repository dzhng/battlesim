// @vitest-environment node
import { readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { Worker } from "node:worker_threads";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { Fielding } from "@wasm/renderer_browser";
import { loadPresentation } from "@web/battle/sim/presentation";
import { resolveCatalogSet } from "@web/battle/catalog/sets";
import presentation from "@fixtures/presentation.json";
import { FieldingOwner, type RuntimeFiles } from "@web/battle/sim/fielding";

beforeEach(() => {
  let frame = 0;
  vi.stubGlobal("document", {
    timeline: {
      get currentTime() {
        return frame;
      },
    },
  });
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) =>
    setImmediate(() => callback(++frame)),
  );
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

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
  const pending = JSON.parse(source.pending_contents()) as { bundle: string; skeleton?: string }[];
  const hashes = [
    ...new Set(
      pending.flatMap(({ bundle, skeleton }) => [bundle, ...(skeleton ? [skeleton] : [])]),
    ),
  ];
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
  const byHash = new Map(hashes.map((hash, i) => [hash, packets[i]]));
  const contents = pending.map((content) => ({
    content,
    hashes: [content.bundle, ...(content.skeleton ? [content.skeleton] : [])],
  }));
  const largestContent = Math.max(
    ...contents.map(({ hashes }) => hashes.reduce((n, hash) => n + byHash.get(hash)!.length, 0)),
  );
  const largestSource = Math.max(...packets.map((packet) => packet.length));
  const worker = new Worker(new URL("./harness/fielding-memory.mjs", import.meta.url), {
    workerData: {
      input,
      sources: [...byHash],
      module: new URL("../src/wasm/renderer_browser/renderer_browser.js", import.meta.url).href,
      wasm: new URL("../src/wasm/renderer_browser/renderer_browser_bg.wasm", import.meta.url).href,
    },
  });
  try {
    const result = await new Promise<{
      before: number;
      after: number;
      missing: typeof pending;
      work: { kinds: number; appearances: number };
    }>((resolve, reject) => {
      worker.once("message", resolve);
      worker.once("error", reject);
      worker.once("exit", (code) => {
        if (code !== 0) reject(new Error(`fielding memory worker exited ${code}`));
      });
    });
    // The page sends one content at a time. Linear memory may hold that
    // boundary input and one source's working space, never another complete
    // set of padded sources. A smaller bound keeps the original no-copy claim.
    expect(result.after - result.before).toBeLessThanOrEqual(largestContent + largestSource);
    expect(result.missing).toEqual([]);
    expect(result.work).toEqual({ kinds: 0, appearances: 0 });
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
  const absorb = Fielding.prototype.absorb;
  vi.spyOn(Fielding.prototype, "absorb").mockImplementation(
    function (this: Fielding, content, bytes, lengths) {
      boundary.push(bytes);
      return absorb.call(this, content, bytes, lengths);
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
    expect(boundary.length).toBeGreaterThan(0);
    expect(boundary.map((packet) => packet.byteLength)).toEqual(boundary.map(() => 0));
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
  const absorb = Fielding.prototype.absorb;
  vi.spyOn(Fielding.prototype, "absorb").mockImplementation(
    function (this: Fielding, content, bytes, lengths) {
      boundary.push(bytes);
      return absorb.call(this, content, bytes, lengths);
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
    expect(boundary.length).toBeGreaterThan(0);
    expect(boundary.map((packet) => packet.byteLength)).toEqual(boundary.map(() => 0));
  } finally {
    owner.free();
  }
});

test("fielding starts independent source reads together and releases every result after absorption", async () => {
  await loadPresentation();
  const set = await resolveCatalogSet("test");
  const catalog = JSON.parse(readFileSync(new URL("catalog.json", runtime), "utf8"));
  const input = {
    rules: JSON.stringify(set.rules),
    catalog: set.units.view,
    kinds: ["test_tank"],
  };
  const fielding = new Fielding(
    JSON.stringify(presentation),
    input.rules,
    JSON.stringify(input.catalog),
    JSON.stringify(catalog),
    JSON.stringify(input.kinds),
  );
  const pending = JSON.parse(fielding.pending_contents()) as {
    bundle: string;
    skeleton?: string;
  }[];
  const wanted = [
    ...new Set(
      pending.flatMap(({ bundle, skeleton }) => [bundle, ...(skeleton ? [skeleton] : [])]),
    ),
  ];
  fielding.free();
  const requested: string[] = [];
  const files: Uint8Array[] = [];
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  const owner = new FieldingOwner();
  const result = owner.field({
    ...input,
    runtime: {
      catalog,
      async contents(hashes) {
        requested.push(...hashes);
        await gate;
        return hashes.map((hash) => {
          const file = new Uint8Array(
            gunzipSync(readFileSync(new URL(`${catalog.gzip[hash].hash}/bundle.bin`, runtime))),
          );
          files.push(file);
          return file;
        });
      },
    },
  });
  try {
    await new Promise((resolve) => setImmediate(resolve));
    expect(new Set(requested)).toEqual(new Set(wanted));
  } finally {
    release();
    await result;
    owner.free();
  }
  expect(files.map((file) => file.byteLength)).toEqual(wanted.map(() => 0));
});
