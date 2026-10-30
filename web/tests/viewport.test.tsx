import { act, cleanup, fireEvent, render } from "@testing-library/react";
import type { ComponentProps } from "react";
import { afterEach, expect, test, vi } from "vitest";
import type { LabViewport } from "@apps/battle-lab/src/LabViewport";
import { Feed } from "@web/feed";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import type { LoadingTasks } from "@apps/battle-lab/src/LabLoading";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import type { SceneOptions } from "@web/battle/render/scene";
import type { ScenePresentation } from "@web/battle/sim/presentation";
import type { ViewportFrame } from "@apps/battle-lab/src/presentedFrame";
import {
  graphicsProfiles,
  type GraphicsQualityProfile,
} from "@apps/battle-lab/src/graphicsSettings";

const gpu = vi.hoisted(() => ({
  resolve: null as null | ((overrides?: Record<string, unknown>) => void),
  reject: null as null | ((error: Error) => void),
  options: [] as SceneOptions[],
  device: { info: () => JSON.stringify({ name: "test GPU", backend: "WebGPU" }) },
  refused: [] as unknown[],
}));
vi.mock("@apps/battle-lab/src/appResources", () => ({
  gpuFailureMessage: String,
  appResources: {
    gpu: async () => gpu.device,
    refuse: (error: unknown) => gpu.refused.push(error),
  },
}));
vi.mock("@web/battle/sim/presentation", () => ({
  NO_PROP_STATS: {},
  ScenePresentation: class {
    battle = false;
    setBattle(battle: unknown) {
      this.battle = battle !== null;
    }
    hasBattle() {
      return this.battle;
    }
    setBuildings() {}
    setGround() {}
    setShadow() {}
    setFog() {}
    setDetailScale() {}
    setSceneryQuality() {}
    sceneryWalk() {
      return {};
    }
    frame() {}
    free() {}
    shadowStats() {
      return null;
    }
    propStats() {
      return {};
    }
  },
}));
// The viewport owns scene lifetime and camera/input. The GPU scene is the
// external boundary; browser scenes exercise its real Rust implementation.
vi.mock("@web/battle/render/scene", () => ({
  pixelsPng: () => Promise.resolve(new Blob()),
  BrowserScene: {
    create: (options: SceneOptions) => {
      gpu.options.push(options);
      return new Promise((resolve, reject) => {
        gpu.reject = reject;
        gpu.resolve = (overrides = {}) => {
          const scene = {
            presentation: options.presentation,
            world: () => options.world,
            worldCurrent: () => true,
            current: () => true,
            appearances: () => options.appearances,
            flush: () => false,
            inheritControls() {},
            staticGeneration: 0,
            setModels() {},
            setInstances() {},
            setOverlay() {},
            setPointerMarks() {},
            setFogStyle() {},
            resize() {},
            render: () => true,
            dispose() {},
            updateSources: () => "current",
            stats: () => ({ memory: { buffers: 0, textures: 0 } }),
            settled: async () => false,
            ...overrides,
          };
          resolve(scene);
        };
      });
    },
  },
}));
async function mount(
  overrides: Partial<ComponentProps<typeof LabViewport>> = {},
  report: ComponentProps<typeof LoadingTasks>["report"] = () => {},
) {
  const { LabViewport } = await import("@apps/battle-lab/src/LabViewport");
  const { LoadingTasks } = await import("@apps/battle-lab/src/LabLoading");
  gpu.resolve = null;
  gpu.reject = null;
  gpu.options = [];
  gpu.refused = [];
  const frames = new Map<number, FrameRequestCallback>();
  let frameId = 0;
  vi.stubGlobal("requestAnimationFrame", (frame: FrameRequestCallback) => {
    frames.set(++frameId, frame);
    return frameId;
  });
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
  const props = {
    fixture: "lifetime",
    world: new Feed({
      terrain: { input: {}, mesh: new Float32Array(0) },
    } as unknown as WorldLayers),
    initialCamera: gameCamera.opening(),
    ...overrides,
  };
  const view = render(<LabViewport {...props} />, {
    wrapper: ({ children }) => <LoadingTasks report={report}>{children}</LoadingTasks>,
  });
  const advance = (now: number) => {
    const scheduled = [...frames.values()];
    frames.clear();
    for (const frame of scheduled) frame(now);
  };
  return { view, props, frames, advance, Viewport: LabViewport };
}
const building = () =>
  act(() =>
    vi.waitFor(() => {
      expect(gpu.refused).toEqual([]);
      expect(gpu.resolve).not.toBeNull();
    }),
  );

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.resetModules();
});

test("a viewport disposed during admission releases the late scene and never becomes ready", async () => {
  const ready = vi.fn();
  const { view, advance } = await mount({ onReady: ready });
  await building();
  view.unmount();
  const dispose = vi.fn();
  await act(async () => gpu.resolve!({ dispose }));
  expect(dispose).toHaveBeenCalledTimes(1);
  act(() => advance(1));
  expect(ready).not.toHaveBeenCalled();
});

test("failed scene admission reports its error to the loading boundary", async () => {
  const reported: (string | null | undefined)[] = [];
  const { view } = await mount({}, (...args) => reported.push(args[1]?.error));
  await building();
  await act(async () => gpu.options[0].failed(new Error("scene setup failed")));
  expect(reported).toContainEqual(expect.stringContaining("scene setup failed"));
  view.unmount();
});

test("appearance updates reconcile on a frame and unmount releases the scene", async () => {
  const appearance = (generation: number): InstalledAppearances => ({
    generation,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    appearances: new Map(),
    onRequest: new Map(),
    skeletons: new Map(),
  });
  const first = appearance(1),
    latest = appearance(2);
  const appearances = new Feed<InstalledAppearances | null>(first);
  const { view, advance } = await mount({ appearances, models: [] });
  await building();
  let requested = first;
  const dispose = vi.fn();
  await act(async () =>
    gpu.resolve!({
      dispose,
      updateSources(_world: WorldLayers, next: InstalledAppearances) {
        requested = next;
        return true;
      },
    }),
  );
  act(() => appearances.set(latest));
  expect(requested).toBe(first);
  await act(async () => advance(1));
  expect(requested).toBe(latest);
  view.unmount();
  expect(dispose).toHaveBeenCalledTimes(1);
});

test("a complete topology/art replacement bypasses asset-only admission on the old renderer", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const decoder = new ScenePresentation();
  decoder.setBattle({ world: {}, tables: new Map() });
  const art = (generation: number): InstalledAppearances => ({
    generation,
    sides: { blue: [1, 1, 1], red: [1, 1, 1] },
    appearances: new Map(),
    onRequest: new Map(),
    skeletons: new Map(),
  });
  const first = art(1),
    next = art(2);
  const appearances = new Feed<InstalledAppearances | null>(first);
  let replaced = false,
    currentWorld = true;
  const { view, advance } = await mount({
    appearances,
    presentation: new Feed<ScenePresentation | null>(decoder),
    frame: () => {
      if (replaced) currentWorld = false;
      return null;
    },
  });
  await building();
  const install = vi.fn(() => "current");
  await act(async () => gpu.resolve!({ updateSources: install, worldCurrent: () => currentWorld }));
  await act(async () => advance(1));
  install.mockClear();
  act(() => {
    appearances.set(next);
    replaced = true;
  });
  expect(install).not.toHaveBeenCalled();
  await act(async () => advance(2));
  expect(install).not.toHaveBeenCalled();
  expect(gpu.options.at(-1)?.appearances).toBe(next);
  replaced = false;
  currentWorld = true;
  await act(async () => gpu.resolve!());
  await act(async () => advance(3));
  view.unmount();
});

test("a later viewport borrows the same page GPU and independently disposes its scene", async () => {
  const first = await mount();
  await building();
  const dispose = vi.fn();
  await act(async () => gpu.resolve!({ dispose }));
  const device = gpu.options[0].gpu;
  first.view.unmount();
  expect(dispose).toHaveBeenCalledTimes(1);
  const second = await mount();
  await building();
  expect(gpu.options[0].gpu).toBe(device);
  second.view.unmount();
  const late = vi.fn();
  await act(async () => gpu.resolve!({ dispose: late }));
  expect(late).toHaveBeenCalledTimes(1);
});

test("battle GPU admission waits for canonical topology before the worker publishes", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const decoder = new ScenePresentation();
  let loaded = false;
  const { view, advance, frames } = await mount({
    presentation: new Feed<ScenePresentation | null>(decoder),
    frame: () => {
      if (loaded) decoder.setBattle({ world: {}, tables: new Map() });
      return null;
    },
  });
  await act(async () => {});
  expect(gpu.options).toEqual([]);
  await act(async () => advance(1));
  expect(gpu.options).toEqual([]);
  loaded = true;
  await act(async () => advance(2));
  expect(gpu.options).toHaveLength(1);
  expect(gpu.options[0].presentation).toBe(decoder);
  view.unmount();
  await act(async () => gpu.resolve!());
  expect(frames.size).toBe(0);
});

test("a session replaced while awaiting topology admits the latest observer", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const before = new ScenePresentation(),
    after = new ScenePresentation();
  after.setBattle({ world: {}, tables: new Map() });
  const observer = new Feed<ScenePresentation | null>(before);
  const { advance, view } = await mount({ presentation: observer, frame: () => null });
  await act(async () => {});
  expect(gpu.options).toEqual([]);
  act(() => observer.set(after));
  await act(async () => advance(1));
  expect(gpu.options).toHaveLength(1);
  expect(gpu.options[0].presentation).toBe(after);
  view.unmount();
  await act(async () => gpu.resolve!());
});

test("departing before battle topology arrives cancels pending admission", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const { view, frames } = await mount({
    presentation: new Feed<ScenePresentation | null>(new ScenePresentation()),
    frame: () => null,
  });
  await act(async () => {});
  expect(frames.size).toBe(1);
  view.unmount();
  await act(async () => {});
  expect(frames.size).toBe(0);
  expect(gpu.options).toEqual([]);
  expect(gpu.refused).toEqual([]);
});

test("an external world reload admits once and skips the old GPU scene while awaiting replacement", async () => {
  const { advance, view } = await mount();
  await building();
  let current = true;
  const oldRender = vi.fn(() => true),
    oldDispose = vi.fn();
  await act(async () =>
    gpu.resolve!({ worldCurrent: () => current, render: oldRender, dispose: oldDispose }),
  );
  act(() => advance(1));
  oldRender.mockClear();
  current = false;
  await act(async () => advance(2));
  expect(gpu.options).toHaveLength(2);
  await act(async () => {
    advance(3);
    advance(4);
  });
  expect(gpu.options).toHaveLength(2);
  expect(oldRender).not.toHaveBeenCalled();
  const nextRender = vi.fn(() => true);
  await act(async () => gpu.resolve!({ render: nextRender }));
  expect(oldDispose).toHaveBeenCalledTimes(1);
  act(() => advance(5));
  expect(nextRender).toHaveBeenCalledTimes(1);
  view.unmount();
});

test("a requested frame waits for the replacement to actually render", async () => {
  const { advance, view } = await mount();
  await building();
  let current = true;
  await act(async () => gpu.resolve!({ worldCurrent: () => current }));
  act(() => advance(1));
  current = false;
  await act(async () => advance(2));
  let complete = false;
  const frame = window.__lab!.frame!().then(() => {
    complete = true;
  });
  await act(async () => {
    advance(3);
    advance(4);
  });
  expect(complete).toBe(false);
  await act(async () => gpu.resolve!());
  await act(async () => advance(5));
  await frame;
  expect(complete).toBe(true);
  view.unmount();
});

test("a skipped surface neither completes a frame nor publishes its observation", async () => {
  const submitted = vi.fn();
  const { advance, view } = await mount({ frame: () => ({ clock: 1, submitted }) });
  await building();
  let usable = false;
  await act(async () => gpu.resolve!({ render: () => usable }));
  let complete = false;
  const frame = window.__lab!.frame!().then(() => {
    complete = true;
  });
  await act(async () => {
    advance(1);
    advance(2);
  });
  expect(complete).toBe(false);
  expect(submitted).not.toHaveBeenCalled();
  usable = true;
  await act(async () => advance(3));
  await frame;
  expect(submitted).toHaveBeenCalledTimes(1);
  view.unmount();
});

test("skipped frames retain the submitted pick boxes and camera rays", async () => {
  const box = (x: number) => ({
    x,
    y: 0,
    z: 0,
    yaw: 0,
    center: [0, 0, 0] as const,
    half: [1, 1, 1] as const,
  });
  let candidate: ViewportFrame = { picks: [box(1)] };
  const { advance, view } = await mount({ frame: () => candidate });
  await building();
  let usable = true;
  await act(async () => gpu.resolve!({ render: () => usable }));
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
  await act(async () => advance(1));
  const initialRay = window.__lab!.rayAt!(400, 300);
  expect(window.__lab!.instances!()).toEqual([box(1)]);

  usable = false;
  candidate = { picks: [box(2)] };
  const camera = window.__lab!.camera!();
  window.__lab!.setCamera!({
    ...camera,
    target: [camera.target[0] + 10, camera.target[1], camera.target[2]],
  });
  await act(async () => advance(2));
  expect(window.__lab!.instances!()).toEqual([box(1)]);
  expect(window.__lab!.rayAt!(400, 300)).toEqual(initialRay);

  usable = true;
  candidate = {}; // Omitted fields keep the staged value across a skipped draw.
  await act(async () => advance(3));
  expect(window.__lab!.instances!()).toEqual([box(2)]);
  expect(window.__lab!.rayAt!(400, 300)).not.toEqual(initialRay);
  view.unmount();
});

test("a requested frame waits for the active scene's desired art generation", async () => {
  const ready = vi.fn();
  const { advance, view } = await mount({ onReady: ready });
  await building();
  let current = true;
  await act(async () => gpu.resolve!({ current: () => current }));
  await act(async () => advance(1));
  expect(ready).toHaveBeenCalledTimes(1);
  current = false;
  let complete = false;
  const frame = window.__lab!.frame!().then(() => {
    complete = true;
  });
  await act(async () => {
    advance(2);
    advance(3);
  });
  expect(complete).toBe(false);
  current = true;
  gpu.options[0].redraw();
  await act(async () => advance(4));
  await frame;
  view.unmount();
});

test("departing during a quality rebuild does not refuse the page's shared GPU", async () => {
  const { advance, view, props, Viewport } = await mount();
  await building();
  await act(async () => gpu.resolve!());
  await act(async () => advance(1));
  await act(async () => view.rerender(<Viewport {...props} quality={graphicsProfiles.low} />));
  expect(gpu.options).toHaveLength(2);
  await act(async () => gpu.resolve!());
  view.unmount();
  await act(async () => {});
  expect(gpu.refused).toEqual([]);
});

test("a replaced session draws its new presentation without touching the retired observer", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const before = new ScenePresentation(),
    after = new ScenePresentation();
  before.setBattle({ world: {}, tables: new Map() });
  after.setBattle({ world: {}, tables: new Map() });
  const observer = new Feed<ScenePresentation | null>(before);
  const { advance, view } = await mount({ presentation: observer, frame: () => null });
  await building();
  const oldRender = vi.fn(() => true),
    oldDispose = vi.fn();
  await act(async () => gpu.resolve!({ render: oldRender, dispose: oldDispose }));
  act(() => advance(1));
  oldRender.mockClear();
  const retiredFrame = vi.spyOn(before, "frame");
  act(() => observer.set(after));
  await act(async () => advance(2));
  expect(retiredFrame).not.toHaveBeenCalled();
  expect(oldRender).not.toHaveBeenCalled();
  expect(gpu.options).toHaveLength(2);
  expect(gpu.options[1].presentation).toBe(after);
  const newRender = vi.fn(() => true);
  await act(async () => gpu.resolve!({ render: newRender }));
  act(() => advance(3));
  expect(newRender).toHaveBeenCalledTimes(1);
  expect(oldDispose).toHaveBeenCalledTimes(1);
  view.unmount();
});

test("a preset changed during admission installs only the newest grass and shadow settings", async () => {
  const quality: GraphicsQualityProfile = {
    unitLodScale: 1,
    buildingLodScale: 1,
    treeLodScale: 1,
    grassDensity: 0.2,
    renderScale: 1,
    forestDressing: true,
    understorey: true,
    shadowMapSize: 512,
    shadowReach: 300,
    bloom: true,
    castLights: true,
    authoredSunSky: true,
    effectMood: "full",
    waterDetail: "full",
  };
  const world = new Feed({
    terrain: { input: {}, mesh: new Float32Array(0) },
    grass: new Map(),
  } as unknown as WorldLayers);
  const { advance, view, props, Viewport } = await mount({ quality, world });
  await building();
  const originalRender = vi.fn(() => true),
    originalDispose = vi.fn();
  await act(async () => gpu.resolve!({ render: originalRender, dispose: originalDispose }));
  act(() => advance(1));
  originalRender.mockClear();
  const pending = { ...quality, grassDensity: 0.5, shadowReach: 600 };
  await act(async () => view.rerender(<Viewport {...props} quality={pending} />));
  expect(gpu.options).toHaveLength(2);
  const latest = { ...pending, grassDensity: 0.9, shadowReach: 900, shadowMapSize: 2048 as const };
  act(() => view.rerender(<Viewport {...props} quality={latest} />));
  const staleDispose = vi.fn(),
    staleRender = vi.fn(() => true);
  await act(async () => gpu.resolve!({ dispose: staleDispose, render: staleRender }));
  expect(staleDispose).toHaveBeenCalledTimes(1);
  expect(staleRender).not.toHaveBeenCalled();
  expect(originalDispose).not.toHaveBeenCalled();
  expect(gpu.options).toHaveLength(3);
  expect(gpu.options[2].settings.grassDensity).toBe(0.9);
  expect(gpu.options[2].settings.light.cascades).toMatchObject({ map_size: 2048, max_far_m: 900 });
  const latestRender = vi.fn(() => true);
  await act(async () => gpu.resolve!({ render: latestRender }));
  act(() => advance(2));
  expect(latestRender).toHaveBeenCalledTimes(1);
  expect(originalDispose).toHaveBeenCalledTimes(1);
  view.unmount();
});

test("a superseded admission rejection retries the newest request", async () => {
  const quality: GraphicsQualityProfile = {
    unitLodScale: 1,
    buildingLodScale: 1,
    treeLodScale: 1,
    grassDensity: 0.2,
    renderScale: 1,
    forestDressing: true,
    understorey: true,
    shadowMapSize: 512 as const,
    shadowReach: 300,
    bloom: true,
    castLights: true,
    authoredSunSky: true,
    effectMood: "full" as const,
    waterDetail: "full" as const,
  };
  const { advance, view, props, Viewport } = await mount({ quality });
  await building();
  await act(async () => gpu.resolve!());
  act(() => advance(1));
  const newer = { ...quality, grassDensity: 0.9, shadowMapSize: 2048 as const, shadowReach: 900 };
  await act(async () => view.rerender(<Viewport {...props} quality={newer} />));
  expect(gpu.options).toHaveLength(2);
  const latest = { ...newer, grassDensity: 0.7, shadowReach: 1200 };
  await act(async () => view.rerender(<Viewport {...props} quality={latest} />));
  await act(async () => gpu.reject!(new Error("retired candidate")));
  expect(window.__lab?.error).toBeNull();
  expect(gpu.options).toHaveLength(3);
  expect(gpu.options[2].settings.light.cascades.max_far_m).toBe(1200);
  await act(async () => gpu.resolve!());
  act(() => advance(2));
  view.unmount();
});

test("failed admission of a retired observer yields to the latest session", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  const before = new ScenePresentation(),
    after = new ScenePresentation();
  before.setBattle({ world: {}, tables: new Map() });
  after.setBattle({ world: {}, tables: new Map() });
  const observer = new Feed<ScenePresentation | null>(before);
  const { advance, view } = await mount({ presentation: observer, frame: () => null });
  await building();
  act(() => observer.set(after));
  await act(async () => gpu.reject!(new Error("retired presentation handle")));
  expect(gpu.refused).toEqual([]);
  expect(window.__lab?.error).toBeNull();
  expect(gpu.options).toHaveLength(2);
  expect(gpu.options[1].presentation).toBe(after);
  const render = vi.fn(() => true);
  await act(async () => gpu.resolve!({ render }));
  await act(async () => advance(1));
  expect(render).toHaveBeenCalledTimes(1);
  expect(window.__lab?.ready).toBe(true);
  view.unmount();
});

test("proxy instances changed during admission become the installed poses", async () => {
  const first = [
    { kind: "infantry" as const, x: 1, y: 0, z: 0, yaw: 0, color: [1, 1, 1] as const },
  ];
  const latest = [{ ...first[0], x: 2 }];
  const { view, props, Viewport } = await mount({ instances: first });
  await building();
  act(() => view.rerender(<Viewport {...props} instances={latest} />));
  let installed = gpu.options[0].instances;
  await act(async () =>
    gpu.resolve!({
      setInstances(instances: typeof latest) {
        installed = instances;
      },
    }),
  );
  expect(installed).toEqual(latest);
  view.unmount();
});

test("changing route proxies after the first draw keeps the viewport live through departure", async () => {
  const first = [
    { kind: "infantry" as const, x: 1, y: 0, z: 0, yaw: 0, color: [1, 1, 1] as const },
  ];
  const { view, props, Viewport, advance } = await mount({ instances: first });
  await building();
  await act(async () => gpu.resolve!());
  await act(async () => advance(1));
  const latest = [{ ...first[0], x: 2 }];
  act(() => view.rerender(<Viewport {...props} instances={latest} />));
  await act(async () => advance(2));
  expect(window.__lab!.instances!()[0].x).toBe(2);
  view.unmount();
  expect(window.__lab).toBeUndefined();
});

test("a pose-only update is framed before its draw completes", async () => {
  const { ScenePresentation } = await import("@web/battle/sim/presentation");
  let posed = 1,
    framed = 0,
    shown = 0;
  const previousFrame = ScenePresentation.prototype.frame;
  vi.spyOn(ScenePresentation.prototype, "frame").mockImplementation(function (
    this: InstanceType<typeof ScenePresentation>,
    ...args
  ) {
    framed = posed;
    return previousFrame.apply(this, args);
  });
  const first = [
    { kind: "infantry" as const, x: 1, y: 0, z: 0, yaw: 0, color: [1, 1, 1] as const },
  ];
  const { view, props, Viewport, advance } = await mount({ instances: first });
  await building();
  await act(async () =>
    gpu.resolve!({
      updateSources(
        _world: WorldLayers,
        _art: InstalledAppearances,
        _models: unknown,
        instances: typeof first,
      ) {
        if (posed === instances[0].x) return "current";
        posed = instances[0].x;
        return "reframe";
      },
      render() {
        shown = framed;
        return true;
      },
    }),
  );
  await act(async () => advance(1));
  expect(shown).toBe(1);
  act(() => view.rerender(<Viewport {...props} instances={[{ ...first[0], x: 2 }]} />));
  const completed = window.__lab!.frame!();
  await act(async () => advance(2));
  await completed;
  expect(shown).toBe(2);
  view.unmount();
});

test("a failed topology replacement ends readiness without retrying every frame", async () => {
  const { advance, view } = await mount();
  await building();
  let current = true;
  await act(async () => gpu.resolve!({ worldCurrent: () => current }));
  act(() => advance(1));
  current = false;
  await act(async () => advance(2));
  await act(async () => gpu.reject!(new Error("replacement failed")));
  await act(async () => {
    advance(3);
    advance(4);
  });
  expect(window.__lab?.ready).toBe(false);
  expect(window.__lab?.error).toContain("replacement failed");
  expect(gpu.options).toHaveLength(2);
  view.unmount();
});

test("render failures clear readiness and report to loading instead of escaping RAF", async () => {
  const reported: (string | null | undefined)[] = [];
  const ready = vi.fn();
  const { view, advance } = await mount({ onReady: ready }, (...args) =>
    reported.push(args[1]?.error),
  );
  await building();
  await act(async () =>
    gpu.resolve!({
      render() {
        throw new Error("draw failed");
      },
    }),
  );
  act(() => advance(1));
  expect(window.__lab?.ready).toBe(false);
  expect(ready).not.toHaveBeenCalled();
  expect(reported).toContainEqual(expect.stringContaining("draw failed"));
  view.unmount();
});

test("a covered viewport ignores camera keys and resumes steering when uncovered", async () => {
  const { view, advance, props, Viewport } = await mount({ inputEnabled: false });
  await building();
  await act(async () => gpu.resolve!());
  act(() => advance(performance.now() + 100));
  const before = [...window.__lab!.camera!().target];
  fireEvent.keyDown(window, { code: "KeyW" });
  act(() => advance(performance.now() + 200));
  expect(window.__lab!.camera!().target).toEqual(before);
  fireEvent.keyUp(window, { code: "KeyW" });
  view.rerender(<Viewport {...props} inputEnabled />);
  fireEvent.keyDown(window, { code: "KeyW" });
  act(() => advance(performance.now() + 300));
  expect(window.__lab!.camera!().target).not.toEqual(before);
  fireEvent.keyUp(window, { code: "KeyW" });
  view.unmount();
});

test("N steps the camera through the side's contacts and Shift+N steps back", async () => {
  const focusTargets = [
    { id: 9, target: [1200, 300] as const, evidenceTick: 9 },
    { id: 4, target: [200, 900] as const, evidenceTick: 4 },
  ];
  const { view, advance } = await mount({ focusTargets });
  await building();
  await act(async () => gpu.resolve!());
  act(() => advance(performance.now() + 100));
  fireEvent.keyDown(window, { code: "KeyN" });
  act(() => advance(performance.now() + 200));
  expect(window.__lab!.camera!().target.slice(0, 2)).toEqual([1200, 300]);
  fireEvent.keyDown(window, { code: "KeyN" });
  act(() => advance(performance.now() + 300));
  expect(window.__lab!.camera!().target.slice(0, 2)).toEqual([200, 900]);
  fireEvent.keyDown(window, { code: "KeyN", shiftKey: true });
  act(() => advance(performance.now() + 400));
  expect(window.__lab!.camera!().target.slice(0, 2)).toEqual([1200, 300]);
  view.unmount();
});

test("a reused contact id starts a new cycle after the old contact leaves", async () => {
  const initial = [
    { id: 9, target: [1200, 300] as const, evidenceTick: 9 },
    { id: 4, target: [200, 900] as const, evidenceTick: 4 },
  ];
  const { view, advance, props, Viewport } = await mount({ focusTargets: initial });
  await building();
  await act(async () => gpu.resolve!());
  act(() => advance(performance.now() + 100));
  fireEvent.keyDown(window, { code: "KeyN" });
  act(() => advance(performance.now() + 200));
  expect(window.__lab!.camera!().target.slice(0, 2)).toEqual([1200, 300]);

  view.rerender(<Viewport {...props} focusTargets={[initial[1]]} />);
  view.rerender(
    <Viewport
      {...props}
      focusTargets={[{ id: 9, target: [700, 700] as const, evidenceTick: 10 }, initial[1]]}
    />,
  );
  fireEvent.keyDown(window, { code: "KeyN" });
  act(() => advance(performance.now() + 300));
  expect(window.__lab!.camera!().target.slice(0, 2)).toEqual([700, 700]);
  view.unmount();
});

test("a covered viewport ignores N like every other camera key", async () => {
  const focusTargets = [{ id: 9, target: [1200, 300] as const, evidenceTick: 9 }];
  const { view, advance, props, Viewport } = await mount({ inputEnabled: false, focusTargets });
  await building();
  await act(async () => gpu.resolve!());
  act(() => advance(performance.now() + 100));
  const before = [...window.__lab!.camera!().target];
  fireEvent.keyDown(window, { code: "KeyN" });
  act(() => advance(performance.now() + 200));
  expect(window.__lab!.camera!().target).toEqual(before);
  view.rerender(<Viewport {...props} inputEnabled />);
  fireEvent.keyDown(window, { code: "KeyN" });
  act(() => advance(performance.now() + 300));
  expect(window.__lab!.camera!().target.slice(0, 2)).toEqual([1200, 300]);
  view.unmount();
});
