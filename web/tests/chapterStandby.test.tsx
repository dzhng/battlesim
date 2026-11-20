// Restarting a chapter: the sim session keeps a battle waiting at the open
// chapter's start, from the publications themselves, and offers it only once
// it is ready. Clients are fakes at the worker seam: each records how it was
// made and hands back the replay and readiness the test settles.
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import type { LocalSimClientOptions, Publication, SimClient } from "@web/battle/sim/client";

/** A promise the test settles. */
function settled<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  promise.catch(() => {});
  return { promise, resolve, reject };
}

interface FakeClient {
  options: LocalSimClientOptions;
  ready: ReturnType<typeof settled<{ tickHz: number; tick: number; layout: string }>>;
  replays: ReturnType<typeof settled<string>>[];
  publish(tick: number, chapter: string | null): void;
  disposed: boolean;
}
const made: FakeClient[] = [];

vi.mock("@web/battle/sim/client", () => ({
  createSimClient(options: LocalSimClientOptions): SimClient {
    let consumer: ((publication: Publication) => void) | null = null;
    const fake: FakeClient = {
      options,
      ready: settled(),
      replays: [],
      disposed: false,
      publish(tick, chapter) {
        consumer?.({
          side: "blue",
          tick,
          digest: `d${tick}`,
          observation: {
            tick,
            refusals: [],
            scenario: { chapter, beats: [] },
          } as never,
          ground: null as never,
          bytes: 0,
          stepMs: 0,
          copyPacked: () => [],
          release() {},
        });
      },
    };
    made.push(fake);
    return {
      ready: fake.ready.promise,
      presentation: null,
      paused: false,
      start() {},
      pause() {},
      resume() {},
      onStatus() {},
      onPublication(next: (publication: Publication) => void) {
        consumer = next;
      },
      replay() {
        const recorded = settled<string>();
        fake.replays.push(recorded);
        return recorded.promise;
      },
      dispose() {
        fake.disposed = true;
      },
    } as unknown as SimClient;
  },
}));

const { useSimSession } = await import("@apps/battle-lab/src/useSimSession");

afterEach(() => {
  cleanup();
  made.length = 0;
  vi.unstubAllGlobals();
});

/** A played battle, its frames drawn at once. */
function session(options: Partial<Parameters<typeof useSimSession>[0]> = {}) {
  vi.stubGlobal("requestAnimationFrame", (frame: FrameRequestCallback) => {
    frame(0);
    return 1;
  });
  vi.stubGlobal("cancelAnimationFrame", () => {});
  return renderHook(() => useSimSession({ scenario: "S", seed: 3, ...options }));
}

const flush = () => act(async () => {});

test("a chapter's standby stands at the tick the chapter opened, and is offered once ready", async () => {
  const { result } = session();
  const live = made[0];
  act(() => {
    live.publish(10, "briefing");
    // Ticks the page never rendered: the chapter opened at 41, not later.
    live.publish(40, "briefing");
    live.publish(41, "crossing");
    live.publish(42, "crossing");
  });
  expect(live.replays).toHaveLength(2);
  await act(async () => live.replays[1].resolve("recorded"));
  const standby = made[1];
  expect(standby.options.resume).toEqual({ replay: "recorded", tick: 41 });
  // Not offered until it stands ready.
  expect(result.current.standbyAt).toBeNull();
  await act(async () => standby.ready.resolve({ tickHz: 30, tick: 41, layout: "" }));
  expect(result.current.standbyAt).toBe(41);
});

test("a standby the simulation refuses is never offered, and is let go", async () => {
  const { result } = session();
  act(() => made[0].publish(5, "briefing"));
  await act(async () => made[0].replays[0].resolve("recorded"));
  await act(async () => made[1].ready.reject(new Error("refused")));
  expect(result.current.standbyAt).toBeNull();
  expect(made[1].disposed).toBe(true);
});

test("a full restart lets the old battle's standby go, and a late replay of it makes none", async () => {
  const { result } = session();
  const first = made[0];
  act(() => first.publish(5, "briefing"));
  await act(async () => first.replays[0].resolve("recorded"));
  await act(async () => made[1].ready.resolve({ tickHz: 30, tick: 5, layout: "" }));
  expect(result.current.standbyAt).toBe(5);
  // The next chapter's replay is still on its way when the player restarts.
  act(() => first.publish(60, "crossing"));
  expect(result.current.standbyAt).toBeNull();
  act(() => result.current.restart());
  expect(made[1].disposed).toBe(true);
  expect(result.current.standbyAt).toBeNull();
  const clients = made.length;
  await act(async () => first.replays[1].resolve("stale"));
  await flush();
  expect(made).toHaveLength(clients);
  expect(result.current.standbyAt).toBeNull();
});

test("Restart in a battle started at a chapter starts it there again", async () => {
  const resume = { replay: "solution", tick: 900 };
  const { result } = session({
    prepared: {
      scenario: "S",
      connect: () => ({ send() {}, close() {} }),
      resume,
    },
  });
  expect(made[0].options.resume).toEqual(resume);
  act(() => result.current.restart());
  const again = made.at(-1)!;
  expect(again.options.connect).toBeUndefined();
  expect(again.options.resume).toEqual(resume);
});

test("Restart chapter plays on from the standby, and the chapter's start stays offered", async () => {
  const { result } = session();
  act(() => made[0].publish(5, "briefing"));
  await act(async () => made[0].replays[0].resolve("recorded"));
  const standby = made[1];
  await act(async () => standby.ready.resolve({ tickHz: 30, tick: 5, layout: "" }));
  act(() => result.current.restartChapter());
  expect(made[0].disposed).toBe(true);
  expect(standby.disposed).toBe(false);
  // The next standby waits at the same start, from the same recording.
  const next = made[2];
  expect(next.options.resume).toEqual({ replay: "recorded", tick: 5 });
  expect(result.current.standbyAt).toBeNull();
  await act(async () => next.ready.resolve({ tickHz: 30, tick: 5, layout: "" }));
  expect(result.current.standbyAt).toBe(5);
  // The adopted battle plays on in the same chapter: no new standby.
  act(() => standby.publish(6, "briefing"));
  expect(standby.replays).toHaveLength(0);
});
