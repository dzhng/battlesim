// @vitest-environment node
import { expect, test, vi } from "vitest";
import {
  loadRegistry,
  nativeToolPath,
  parseArgs,
  pinnedFixtures,
  selectFixtures,
  selectVerifyFixtures,
  fatalGpuMessage,
  openScenePage,
  runConcurrent,
  sceneUrlFor,
  mergeSceneUrl,
} from "./scene.mjs";

test("fatal GPU diagnostics are identified before readiness timeouts", () => {
  expect(fatalGpuMessage("WebGPU uncaptured error: Texture size exceeded maximum")).toBe(true);
  expect(fatalGpuMessage('WebGPU uncaptured error: [Invalid Texture "<unnamed>"]')).toBe(true);
  expect(fatalGpuMessage("GPUValidationError: bind group layout is invalid")).toBe(true);
  expect(fatalGpuMessage("WebGPU is unavailable: requestAdapter() returned null")).toBe(true);
  expect(fatalGpuMessage("GPUDevice requestDevice failed")).toBe(true);
  expect(fatalGpuMessage("the scene is still loading")).toBe(false);
});

test("native helpers live in the durable Cargo target by default", () => {
  vi.stubEnv("CARGO_TARGET_DIR", "target");
  expect(nativeToolPath("debug/examples/mechanics_validate")).toMatch(
    /[\\/]target[\\/]debug[\\/]examples[\\/]mechanics_validate$/,
  );
  vi.unstubAllEnvs();
});

test("scene args: fixture ids, the bun `--` separator, --list, --verify and --visual", () => {
  expect(parseArgs(["--", "foundation"])).toEqual({
    ids: ["foundation"],
    list: false,
    visual: false,
    verify: false,
  });
  expect(parseArgs(["--list"])).toEqual({ ids: [], list: true, visual: false, verify: false });
  expect(parseArgs(["--visual"])).toEqual({ ids: [], list: false, visual: true, verify: false });
  expect(parseArgs(["--verify"])).toEqual({ ids: [], list: false, visual: false, verify: true });
  expect(() => parseArgs(["--visual", "panels"])).toThrow("--visual");
  expect(() => parseArgs(["--bogus"])).toThrow("unknown flag");
});

test("the default verify gate keeps behavior scenes and the low visual smoke test", async () => {
  const fixtures = await loadRegistry();
  const selected = selectVerifyFixtures(fixtures).map((fixture) => fixture.id);
  expect(selected).toContain("foundation-low");
  expect(selected).not.toContain("panels");
  expect(selected).not.toContain("ui");
  expect(selected).not.toContain("unit-roster");
  expect(selected).not.toContain("city-lineup");
  expect(selected).not.toContain("generated");
  expect(selected).not.toContain("street");
  expect(selected).toContain("air-combined");
  expect(selected).toContain("garrison");
  expect(selected).not.toContain("scenario-play");
});

test("scene scheduler limits overlap and returns fixture order", async () => {
  let active = 0;
  let peak = 0;
  const seen = [];
  const values = await runConcurrent(["a", "b", "c", "d"], 2, async (id) => {
    active++;
    peak = Math.max(peak, active);
    seen.push(`${id}:start`);
    await new Promise((resolve) => setTimeout(resolve, 1));
    seen.push(`${id}:end`);
    active--;
    return id.toUpperCase();
  });
  expect(values).toEqual(["A", "B", "C", "D"]);
  expect(peak).toBe(2);
  expect(seen.filter((event) => event.endsWith(":start"))).toHaveLength(4);
  expect(seen.filter((event) => event.endsWith(":end"))).toHaveLength(4);
});

test("test graphics quality is only added to lab routes", () => {
  expect(
    sceneUrlFor("http://127.0.0.1:5173/labs/foundation", "low").searchParams.get("test-quality"),
  ).toBe("low");
  expect(
    sceneUrlFor("http://127.0.0.1:5173/battle?seed=1", "low").searchParams.has("test-quality"),
  ).toBe(false);
});

test("scene URL parameters merge with fixture queries", () => {
  expect(
    mergeSceneUrl("http://127.0.0.1:5173/labs/geometry?test-quality=low", { map: "river" }),
  ).toBe("http://127.0.0.1:5173/labs/geometry?test-quality=low&map=river");
});

test("every registered fixture has exactly one scene", async () => {
  const fixtures = await loadRegistry();
  expect(fixtures.length).toBeGreaterThan(0);
  expect(new Set(fixtures.map((fixture) => fixture.id)).size).toBe(fixtures.length);
  expect(new Set(fixtures.map((fixture) => fixture.route)).size).toBe(fixtures.length);
  for (const fixture of fixtures) expect([undefined, "production"]).toContain(fixture.build);
  expect(selectFixtures(fixtures, []).length).toBe(fixtures.length);
  expect(() => selectFixtures(fixtures, ["nope"])).toThrow("unknown fixture");
});

// Each scene page has a browser context of its own, and the context holds what
// the page stored (the appearance prefetch fills Cache Storage, which an
// incognito context keeps in the browser process's memory). Contexts left
// open after their pages grew the browser past 10 GB over street's tours,
// until it crashed.
test("a scene page takes its browser context with it when it closes", async () => {
  const { chromium } = await import("playwright");
  const browser = await chromium.launch();
  try {
    for (let i = 0; i < 3; i++) {
      const page = await openScenePage(browser);
      await page.close();
    }
    expect(browser.contexts()).toEqual([]);
  } finally {
    await browser.close();
  }
});

test("--visual selects exactly the fixtures with approved pictures", async () => {
  const fixtures = await loadRegistry();
  const pinned = (await pinnedFixtures(fixtures)).map((f) => f.id);
  expect(pinned).toEqual(expect.arrayContaining(["cursor", "panels", "ui"]));
  // A scene with no approved pictures (the GPU foundation) is not selected.
  expect(pinned).not.toContain("foundation");
});
