// The generator's request pool: how many calls run at once, and when it
// retries or stops.
import { describe, expect, it } from "vitest";
import { Pool } from "@packages/battle-audio/tools/generate/pool";

describe("request pool", () => {
  const error = (status: number) => Object.assign(new Error(`status ${status}`), { status });
  const instant = { sleep: async () => {}, random: () => 0.5 };

  it("retries throttling and server errors with backoff", async () => {
    let attempts = 0;
    const pool = new Pool(2, instant);
    const value = await pool.run(async () => {
      attempts++;
      if (attempts < 3) throw error(attempts === 1 ? 429 : 503);
      return "ok";
    });
    expect(value).toBe("ok");
    expect(attempts).toBe(3);
  });

  it("stops every queued request on an authorisation or quota error", async () => {
    const pool = new Pool(1, instant);
    const started: number[] = [];
    const results = await Promise.allSettled(
      [0, 1, 2].map((i) =>
        pool.run(async () => {
          started.push(i);
          if (i === 0) throw error(401);
          return i;
        }),
      ),
    );
    expect(started).toEqual([0]);
    expect(results.every((r) => r.status === "rejected")).toBe(true);
    expect(pool.stopped?.message).toMatch(/401/);
  });

  it("stops after several failures in a row", async () => {
    const pool = new Pool(1, { ...instant, maxFailures: 2 });
    const started: number[] = [];
    await Promise.allSettled(
      [0, 1, 2].map((i) =>
        pool.run(async () => {
          started.push(i);
          throw error(400);
        }),
      ),
    );
    expect(started).toEqual([0, 1]);
    expect(pool.stopped).not.toBeNull();
  });

  it("never runs more requests at once than its limit", async () => {
    const pool = new Pool(3, instant);
    let running = 0;
    let peak = 0;
    await Promise.all(
      Array.from({ length: 10 }, () =>
        pool.run(async () => {
          peak = Math.max(peak, ++running);
          await new Promise((resolve) => setTimeout(resolve, 1));
          running--;
        }),
      ),
    );
    expect(peak).toBe(3);
  });
});
