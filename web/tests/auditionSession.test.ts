// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
import { AuditionSession } from "../../apps/sound-workbench/src/auditionSession";
import type { Auditioner, LiveMix } from "../../apps/sound-workbench/src/audition";
import type { SoundCatalog } from "@packages/battle-audio/src/catalog";

afterEach(() => vi.restoreAllMocks());

const mix: LiveMix = { loops: vi.fn(), hit: vi.fn() };
const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
};

function fakeAuditioner() {
  return {
    play: vi.fn(async () => {}),
    live: vi.fn(),
    voice: vi.fn(async () => {}),
    music: vi.fn(),
    lines: vi.fn(async () => null),
    stop: vi.fn(),
  } satisfies Auditioner;
}

test("a late replacement cannot publish the old key or live mix", async () => {
  const audition = fakeAuditioner();
  const session = new AuditionSession(audition);
  const old = deferred<LiveMix>();
  const next = deferred<LiveMix>();
  audition.live.mockReturnValueOnce(old.promise).mockReturnValueOnce(next.promise);

  const first = session.start(
    "drive:old",
    () => audition.live({} as SoundCatalog, ["old"]),
    () => mix,
  );
  expect(session.getSnapshot()).toMatchObject({ phase: "loading", key: "drive:old" });
  const replacement = session.start(
    "drive:new",
    () => audition.live({} as SoundCatalog, ["new"]),
    () => mix,
  );
  old.resolve(mix);
  expect(await first).toBeNull();
  expect(session.getSnapshot()).toMatchObject({ phase: "loading", key: "drive:new", mix: null });
  next.resolve(mix);
  expect(await replacement).toBe(mix);
  expect(session.getSnapshot()).toMatchObject({ phase: "playing", key: "drive:new", mix });
});

test("stop invalidates a pending completion and publishes one stop notification", async () => {
  const audition = fakeAuditioner();
  const session = new AuditionSession(audition);
  const pending = deferred<void>();
  const started = session.start("report", () => pending.promise);
  session.stop();
  pending.resolve();

  expect(await started).toBeNull();
  expect(session.getSnapshot()).toEqual({ phase: "stopped", key: null, mix: null, stops: 1 });
  expect(audition.stop).toHaveBeenCalled();
});
