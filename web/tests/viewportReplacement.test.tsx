import { act, cleanup } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { Feed } from "@web/feed";
import type { ScenePresentation as ScenePresentationType } from "@web/battle/sim/presentation";
import type { ViewportFrame } from "@apps/battle-lab/src/presentedFrame";
import { building, getGpu, mount } from "./viewportHarness";

test("a replaced session draws its new presentation without touching the retired observer", async () => {
  const box = (x: number) => ({
    x,
    y: 800,
    z: 0,
    yaw: 0,
    center: [0, 0, 0] as const,
    half: [1, 1, 1] as const,
  });
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const before = new ScenePresentation(),
    after = new ScenePresentation();
  before.setBattle({ world: {}, tables: new Map() });
  after.setBattle({ world: {}, tables: new Map() });
  const observer = new Feed<ScenePresentationType | null>(before);
  let candidate = { picks: [box(170)] };
  const { advance, view } = await mount({ presentation: observer, frame: () => candidate });
  await building();
  const canvas = view.container.querySelector("canvas")!;
  vi.spyOn(canvas, "getBoundingClientRect").mockReturnValue({
    x: 0,
    y: 0,
    left: 0,
    top: 0,
    right: 800,
    bottom: 600,
    width: 800,
    height: 600,
    toJSON() {},
  });
  const oldRender = vi.fn(() => true),
    oldDispose = vi.fn();
  await act(async () => getGpu().resolve!({ render: oldRender, dispose: oldDispose }));
  act(() => advance(1));
  expect(window.__lab!.instances!()).toEqual([box(170)]);
  const submittedRay = window.__lab!.rayAt!(400, 300);
  oldRender.mockClear();
  const retiredFrame = vi.spyOn(before, "frame");
  act(() => observer.set(after));
  await act(async () => advance(2));
  expect(retiredFrame).not.toHaveBeenCalled();
  expect(oldRender).not.toHaveBeenCalled();
  expect(getGpu().options).toHaveLength(2);
  expect(getGpu().options[1].presentation).toBe(after);
  const newRender = vi.fn(() => true);
  await act(async () => getGpu().resolve!({ render: newRender }));
  const camera = window.__lab!.camera!();
  window.__lab!.setCamera!({
    ...camera,
    target: [camera.target[0] + 10, camera.target[1], camera.target[2]],
  });
  expect(window.__lab!.instances!()).toEqual([]);
  expect(window.__lab!.rayAt!(400, 300)).toEqual(submittedRay);
  candidate = { picks: [box(180)] };
  act(() => advance(3));
  expect(newRender).toHaveBeenCalledTimes(1);
  expect(window.__lab!.instances!()).toEqual([box(180)]);
  expect(window.__lab!.rayAt!(400, 300)).not.toEqual(submittedRay);
  expect(oldDispose).toHaveBeenCalledTimes(1);
  view.unmount();
});

test("a replacement drops a staged frame from the retired observer", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const before = new ScenePresentation(),
    after = new ScenePresentation();
  before.setBattle({ world: {}, tables: new Map() });
  after.setBattle({ world: {}, tables: new Map() });
  const observer = new Feed<ScenePresentationType | null>(before);
  const oldSubmitted = vi.fn();
  let candidate: ViewportFrame | null = { submitted: oldSubmitted };
  let renderable = false;
  const { advance, view } = await mount({
    presentation: observer,
    frame: () => candidate,
  });
  await building();
  await act(async () => getGpu().resolve!({ render: () => renderable }));
  await act(async () => advance(1));
  act(() => observer.set(after));
  await act(async () => advance(2));
  candidate = null;
  renderable = true;
  const newRender = vi.fn(() => true);
  await act(async () => getGpu().resolve!({ render: newRender }));
  await act(async () => advance(3));
  expect(newRender).toHaveBeenCalled();
  expect(oldSubmitted).not.toHaveBeenCalled();
  view.unmount();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.resetModules();
});
