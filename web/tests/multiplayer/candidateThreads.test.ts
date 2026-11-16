// @vitest-environment node
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { writeFileSync } from "node:fs";
import type { Worker } from "node:worker_threads";
import { createBattleHost, type BattleHost } from "../../../apps/game-server/src/battleHost/index";
import { battleCapacity } from "../../../apps/game-server/src/online";
import { data, duel, loadBattleSupport, recordHost } from "./support/battle";

const root = join(import.meta.dirname, "../../..");
const out = join(root, "throwaway/multiplayer/candidate-threads");
const workers = vi.hoisted(() => ({
  autoDelays: [] as number[],
  made: [] as {
    worker: Worker;
    gate: Int32Array;
    waiting: boolean;
    messages: string[];
    exited: Promise<unknown>;
  }[],
}));
vi.mock("node:worker_threads", async (original) => {
  const actual = await original<typeof import("node:worker_threads")>();
  return {
    ...actual,
    Worker: class extends actual.Worker {
      constructor() {
        const gate = new Int32Array(new SharedArrayBuffer(4));
        super(
          new URL(
            "../../../throwaway/multiplayer/candidate-threads/candidateWorker.mjs",
            import.meta.url,
          ),
          {
            workerData: {
              gate: gate.buffer,
              delayMs: workers.autoDelays.shift(),
              entry: new URL(
                "../../../throwaway/multiplayer/candidate-threads/worker.mjs",
                import.meta.url,
              ).href,
            },
          },
        );
        const entry = {
          worker: this,
          gate,
          waiting: false,
          messages: [] as string[],
          exited: new Promise((resolve) => this.once("exit", resolve)),
        };
        workers.made.push(entry);
        const emit = this.emit.bind(this);
        this.emit = (event: string | symbol, ...args: unknown[]) => {
          if (event === "message") {
            if ((args[0] as { type: string }).type === "test-setup-waiting") {
              entry.waiting = true;
              return true;
            }
            entry.messages.push((args[0] as { type: string }).type);
          }
          return emit(event, ...args);
        };
      }
    },
  };
});
vi.mock("../../../apps/game-server/src/online", async (original) => ({
  ...(await original<typeof import("../../../apps/game-server/src/online")>()),
  battleCapacity: vi.fn(() => 3),
}));

const hosts: BattleHost[] = [];
beforeAll(async () => {
  execFileSync(
    "bun",
    [
      "build",
      "--target=node",
      "--format=esm",
      "--entry-naming",
      "[name].mjs",
      "--outdir",
      out,
      "apps/game-server/src/battleHost/worker.ts",
      "web/tests/multiplayer/support/candidateWorker.ts",
    ],
    { cwd: root, stdio: "pipe" },
  );
  await loadBattleSupport();
}, 60_000);
afterEach(async () => {
  for (const host of hosts.splice(0)) host.dispose();
  await Promise.all(workers.made.map((entry) => entry.worker.terminate()));
  workers.made.length = 0;
  workers.autoDelays.length = 0;
  vi.mocked(battleCapacity).mockReturnValue(3);
});
const release = (index: number, crash = false) => {
  const gate = workers.made[index].gate;
  Atomics.store(gate, 0, crash ? 2 : 1);
  Atomics.notify(gate, 0);
};
const waiting = (count: number) =>
  vi.waitFor(() => expect(workers.made.filter((w) => w.waiting)).toHaveLength(count));
const unmakeable = () => ({ ...duel, map: { ...duel.map, type: "no such map type" as never } });
function host(candidates = [unmakeable(), unmakeable(), duel]) {
  const result = createBattleHost({ candidates }, "thread", data);
  hosts.push(result);
  return result;
}

test("slow refused setups overlap; the first admitted worker becomes the identical battle and leaves no preparing worker", async () => {
  const b = host();
  const recording = recordHost(b, { side: "blue", player: "blue-1" });
  // Three independent setup delays have all begun before any is released.
  // Serial preparation cannot reach this barrier. There is no real-map timing assumption.
  await waiting(3);
  let ready = false;
  void b.ready.then(
    () => {
      ready = true;
    },
    () => {},
  );
  release(2);
  await vi.waitFor(
    () => expect(workers.made[2].messages.includes("candidate-prepared")).toBe(true),
    { timeout: 30_000 },
  );
  expect(ready).toBe(false);
  release(1);
  await workers.made[1].exited;
  expect(ready).toBe(false);
  release(0);
  const admitted = await b.ready;
  expect(admitted.candidate).toBe(2);
  expect(
    workers.made.filter((w) => w.worker.threadId !== -1).map((w) => workers.made.indexOf(w)),
  ).toEqual([2]);
  await vi.waitFor(() => expect(recording.frames.size).toBeGreaterThan(3));
  const replay = await b.replay();
  b.dispose();
  await workers.made[2].exited;
  expect(workers.made.every((w) => w.worker.threadId === -1)).toBe(true);

  const alone = host([duel]);
  const control = recordHost(alone, { side: "blue", player: "blue-1" });
  await waiting(4);
  release(3);
  expect((await alone.ready).candidate).toBe(0);
  expect(workers.made).toHaveLength(4); // One candidate starts exactly one worker.
  await vi.waitFor(
    () =>
      expect(Math.max(...control.frames.keys())).toBeGreaterThanOrEqual(
        Math.max(...recording.frames.keys()),
      ),
    { timeout: 30_000 },
  );
  expect(
    [...recording.frames].filter(([tick, digest]) => control.frames.get(tick) !== digest),
  ).toEqual([]);
  expect(await alone.replay()).toBe(replay);
}, 60_000);

test("exhaustion rejects and settles queued operations and readers only after every worker stops", async () => {
  vi.mocked(battleCapacity).mockReturnValue(2);
  const b = host([unmakeable(), unmakeable(), unmakeable()]);
  const ended: string[] = [];
  b.onEnd((end) => ended.push(end.kind));
  let read!: Promise<string[]>;
  b.attach({
    seat: { side: "blue", player: "blue-1" },
    onReader(open) {
      read = new Promise((resolve) => {
        const kinds: string[] = [];
        open(null, (record) => {
          kinds.push(record.kind);
          if (record.kind === "end") resolve(kinds);
          return true;
        });
      });
    },
    onUp() {},
    onPreview() {},
  });
  const settled = Promise.allSettled([
    b.ready,
    b.replay(),
    b.referee(),
    b.seatHuman("blue-1", true),
  ]);
  await waiting(2);
  release(1);
  await workers.made[1].exited;
  await waiting(3);
  expect(workers.made.filter((w) => w.worker.threadId !== -1)).toHaveLength(2);
  release(2);
  release(0);
  expect((await settled).map((r) => r.status)).toEqual(Array(4).fill("rejected"));
  expect((await read).at(-1)).toBe("end");
  expect(ended).toEqual(["crashed"]);
  expect(workers.made.every((w) => w.worker.threadId === -1)).toBe(true);
}, 60_000);

test("a worker that dies during preparation refuses only its candidate and frees its slot", async () => {
  vi.mocked(battleCapacity).mockReturnValue(2);
  const b = host([duel, duel, duel]);
  const ended: string[] = [];
  b.onEnd((end) => ended.push(end.kind));
  await waiting(2);
  release(0, true);
  await waiting(3);
  expect(ended).toEqual([]);
  expect(workers.made.filter((w) => w.worker.threadId !== -1)).toHaveLength(2);
  release(1);
  expect((await b.ready).candidate).toBe(1);
  expect(workers.made.map((w) => w.worker.threadId !== -1)).toEqual([false, true, false]);
  expect(ended).toEqual([]);
}, 60_000);

test("a lower admission wins at once, stopping prepared and blocked losers without starting the suffix", async () => {
  const b = host([duel, { ...duel, battleSeed: duel.battleSeed + 1 }, unmakeable(), duel]);
  await waiting(3);
  release(1);
  await vi.waitFor(
    () => expect(workers.made[1].messages.includes("candidate-prepared")).toBe(true),
    { timeout: 30_000 },
  );
  expect(workers.made[1].messages.includes("publication")).toBe(false);
  release(2);
  await workers.made[2].exited;
  await new Promise<void>((resolve) => setImmediate(resolve));
  expect(workers.made).toHaveLength(3);
  release(0);
  expect((await b.ready).candidate).toBe(0);
  expect(workers.made.map((w) => w.worker.threadId !== -1)).toEqual([true, false, false]);
  // Candidate three cannot win once one has been admitted, so never starts.
  expect(workers.made).toHaveLength(3);
}, 60_000);

test("disposing during preparation stops every worker, settles requests and never starts queued candidates", async () => {
  vi.mocked(battleCapacity).mockReturnValue(2);
  const b = host([duel, duel, duel]);
  const settled = Promise.allSettled([b.ready, b.replay(), b.referee()]);
  await waiting(2);
  b.dispose();
  expect((await settled).map((r) => r.status)).toEqual(Array(3).fill("rejected"));
  await vi.waitFor(() => expect(workers.made.every((w) => w.worker.threadId === -1)).toBe(true));
  expect(workers.made).toHaveLength(2);
});

test("readiness costs the slowest setup delay, not the sum of refused setup delays", async () => {
  const delay = 5000;
  async function timed(candidates: Parameters<typeof host>[0], delays: number[]) {
    workers.autoDelays.push(...delays);
    const from = workers.made.length;
    const started = performance.now();
    const b = host(candidates);
    const candidate = await b.ready.then(
      (ready) => ready.candidate,
      () => -1,
    );
    const elapsed = performance.now() - started;
    b.dispose();
    await Promise.all(workers.made.slice(from).map((w) => w.exited));
    return { candidate, elapsed };
  }
  const slow = await timed([unmakeable()], [delay]);
  const alone = await timed([duel], [0]);
  const together = await timed([unmakeable(), unmakeable(), duel], [delay, delay, 0]);
  expect([slow.candidate, alone.candidate, together.candidate]).toEqual([-1, 0, 2]);
  // Controlled setup delays dominate the comparison; the real admitted map's
  // cost is measured separately. Half a delay is load slack, not a map guess.
  expect(together.elapsed).toBeLessThan(Math.max(slow.elapsed, alone.elapsed) + delay / 2);
  writeFileSync(
    join(out, "timings.json"),
    JSON.stringify({
      refusedAlone: slow.elapsed,
      admittedAlone: alone.elapsed,
      together: together.elapsed,
    }) + "\n",
  );
}, 60_000);
