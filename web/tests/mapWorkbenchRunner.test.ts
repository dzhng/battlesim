// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { LatestRunner } from "../../apps/map-workbench/src/runner";

afterEach(() => vi.useRealTimers());

test("cancelling waits for owned work to finish and suppresses a late reply", async () => {
  vi.useFakeTimers();
  let finish!: (value: string) => void;
  let signal!: AbortSignal;
  const publish = vi.fn();
  const runner = new LatestRunner({
    delayMs: 300,
    run: (_input: number, abort: AbortSignal) => {
      signal = abort;
      return new Promise<string>((resolve) => {
        finish = resolve;
      });
    },
    publish,
    fail: vi.fn(),
  });
  runner.request(1);
  await vi.advanceTimersByTimeAsync(300);
  let settled = false;
  const cancelled = Promise.resolve(runner.cancel()).then(() => {
    settled = true;
  });
  await vi.advanceTimersByTimeAsync(0);
  expect(signal.aborted).toBe(true);
  expect(settled).toBe(false);
  finish("late result");
  await cancelled;
  expect(settled).toBe(true);
  expect(publish).not.toHaveBeenCalled();
  runner.dispose();
});

test("rapid edits preserve only the latest pending draft and never publish a superseded result", async () => {
  vi.useFakeTimers();
  const completed: { input: number; result: string }[] = [];
  const starts: number[] = [];
  const replies = new Map<number, (value: string) => void>();
  const runner = new LatestRunner({
    delayMs: 300,
    run: (input: number) => {
      starts.push(input);
      return new Promise<string>((resolve) => replies.set(input, resolve));
    },
    publish: (result, input) => completed.push({ input, result }),
    fail: (error) => {
      throw error;
    },
  });
  runner.request(1);
  await vi.advanceTimersByTimeAsync(300);
  runner.request(2);
  runner.request(3);
  await vi.advanceTimersByTimeAsync(300);
  replies.get(1)!("old plan");
  await vi.advanceTimersByTimeAsync(0);
  expect(starts).toEqual([1, 3]);
  expect(completed).toEqual([]);
  replies.get(3)!("current plan");
  await vi.advanceTimersByTimeAsync(0);
  expect(completed).toEqual([{ input: 3, result: "current plan" }]);
  runner.dispose();
});
