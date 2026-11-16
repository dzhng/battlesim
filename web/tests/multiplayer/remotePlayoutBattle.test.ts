// @vitest-environment node
import { join } from "node:path";
import { expect, test } from "vitest";
import { createBattleHost } from "../../../apps/game-server/src/battleHost/index";
import { nodeSetupData } from "../../../apps/game-server/src/serverData";
import { loopback } from "../../../packages/multiplayer/src/link";
import { toBattleSetup } from "../../../packages/multiplayer/src/setup";
import { KEYFRAME } from "../../../packages/multiplayer/src/stream/index";
import type { StreamRecord } from "../../../packages/multiplayer/src/wire";
import { preset, setOccupant } from "../../../packages/skirmish-setup/src/lobby";
import { remoteConnection } from "../../src/battle/remote/connection";
import { ScenePresentation, loadPresentation } from "../../src/battle/sim/presentation";
import { PageClock } from "./pageClock";

const flush = () => new Promise<void>((resolve) => setImmediate(resolve));

test("jittery real battle records all decode contiguously; after a long stall a newer snapshot reaches the current tick", async () => {
  await loadPresentation();
  const data = await nodeSetupData(join(import.meta.dirname, "../../.."));
  const setup = toBattleSetup(
    setOccupant(preset("1v1"), "seat-2", { kind: "human", who: "guest" }),
    "skirmish",
  );
  const battleClock = new PageClock();
  const host = createBattleHost(
    { candidates: [setup] },
    { clock: battleClock, generator: data.generator },
    data,
  );
  const { tickHz } = await host.ready;
  const pageClock = new PageClock();
  const pair = loopback({ side: "blue", player: "blue-1" });
  const pipes = new Set<{ queue: StreamRecord[]; deliver: (record: StreamRecord) => boolean }>();
  const opens: (number | null)[] = [];
  let currentTick = 0;
  const keyframes: number[] = [];
  // Jitter belongs to the transport: retain delivery, never interpret resume.
  host.attach({
    ...pair.port,
    onReader(handle) {
      pair.port.onReader((after, deliver) => {
        opens.push(after);
        const pipe = { queue: [] as StreamRecord[], deliver };
        pipes.add(pipe);
        const detach = handle(after, (record) => {
          if (record.kind === "publication") {
            currentTick = new DataView(record.payload.buffer, record.payload.byteOffset).getUint32(
              0,
              true,
            );
            if (record.flags & KEYFRAME) keyframes.push(currentTick);
          }
          pipe.queue.push(record);
          return true;
        });
        return () => {
          pipes.delete(pipe);
          detach();
        };
      });
    },
  });
  const deliver = async () => {
    for (const pipe of pipes) for (const record of pipe.queue.splice(0)) pipe.deliver(record);
    await flush();
  };
  let decoder: ScenePresentation | undefined;
  const seen: { tick: number; epoch: number }[] = [];
  const failures: string[] = [];
  const channel = remoteConnection(pair.link, pageClock)(
    (reply) => {
      if (reply.type === "ready") {
        decoder = new ScenePresentation(reply.layout, reply.tickHz);
        decoder.invalidate("blue");
      }
      if (reply.type !== "publication") return;
      const frame = decoder!.decode(
        new Float32Array(reply.buffer, 0, reply.length),
        pageClock.now(),
      );
      expect(frame, `reader rejected tick ${reply.tick}`).not.toBeNull();
      expect(frame!.tick).toBe(reply.tick);
      seen.push({ tick: reply.tick, epoch: frame!.groundPatch.epoch });
    },
    (error) => failures.push(error),
  );
  const run = async (ticks: number, jitter = true) => {
    for (let i = 0; i < ticks; i++) {
      await battleClock.advance(1000 / tickHz);
      // Alternating burst sizes, so this is not merely a delayed steady stream.
      if (!jitter || i % 5 === 0 || i % 5 === 2) await deliver();
      await pageClock.advance(1000 / tickHz);
    }
  };
  try {
    channel.send({ type: "connect", side: "blue" });
    await run(3, false);
    expect(decoder).toBeDefined();
    channel.send({ type: "start" });
    await run(2 * tickHz);
    await deliver();
    await pageClock.advance(500);
    expect(seen.length).toBeGreaterThan(10);
    expect(seen.map((s) => s.tick)).toEqual(
      Array.from({ length: seen.length }, (_, i) => seen[0].tick + i),
    );
    const before = seen.at(-1)!;
    const count = seen.length;

    // A suspended page accumulates more than two seconds of the live battle.
    for (let i = 0; i < 3 * tickHz; i++) await battleClock.advance(1000 / tickHz);
    await deliver();
    const stalledAt = currentTick;
    pageClock.suspend(60000);
    await pageClock.advance(0);
    expect(opens).toEqual([null, null, null]);
    expect(seen).toHaveLength(count);
    // The host takes a fresh snapshot in line. Stop advancing it, then drain
    // playout: the final page tick must be the battle's actual current tick.
    await run(5, false);
    await deliver();
    await pageClock.advance(1000);
    const resumed = seen.slice(count);
    expect(resumed.length).toBeGreaterThan(0);
    expect(resumed[0].tick).toBeGreaterThanOrEqual(stalledAt);
    expect(keyframes).toContain(resumed[0].tick);
    expect(resumed[0].epoch).toBeGreaterThan(before.epoch);
    expect(resumed.map((s) => s.tick)).toEqual(
      Array.from({ length: resumed.length }, (_, i) => resumed[0].tick + i),
    );
    expect(seen.at(-1)!.tick).toBe(currentTick);
    expect(failures).toEqual([]);
  } finally {
    channel.close();
    decoder?.free();
    host.dispose();
    battleClock.dispose();
  }
}, 60_000);
