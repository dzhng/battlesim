// @vitest-environment jsdom
// The Armory's stage in the real menu: a failure stays on the stage, with
// the list, the sheet and the menu still usable and the page's resources
// untouched; a first-launch visit waits for graphics detection, which is
// never aborted, before it builds; a planned card never asks for art; and
// leaving stops the stage's loops while the menu bed plays on; and an open
// by address keeps the loading cover while the roster's catalog loads. The
// 3D stage and its art are stand-ins here: `web/scenes/armory.mjs` judges
// the drawn page.
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { useEffect } from "react";
import { BrowserRouter } from "react-router";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { AppAudio } from "@packages/battle-audio/src/appAudio";
import { catalogSet, type SessionCatalog } from "@web/battle/catalog/sets";
import { appResources } from "@apps/battle-lab/src/appResources";
import {
  graphicsDetectionState,
  registerGraphicsDetectionRunner,
  runAutoDetect,
} from "@apps/battle-lab/src/graphicsDetection";
import {
  graphicsSettings,
  graphicsSettingsStorageKey,
} from "@apps/battle-lab/src/graphicsSettings";
import { useLabLoading } from "@apps/battle-lab/src/LabLoading";
import { menuPage } from "@apps/battle-lab/src/menu/pages";
import { PreviewWorker } from "./support/previewWorker";
import { visitShell } from "./support/router";

/** The stand-ins' record: stages built, units shown, art asked for, and the
 *  failure the next art request meets. */
const stand = vi.hoisted(() => ({
  stages: 0,
  shown: [] as (string | null)[],
  asked: 0,
  /** Each art load's unit, null for the yard alone. */
  kinds: [] as (string | null)[],
  failure: null as Error | null,
}));

/** The backdrop is ready at once, and releases the detection cover once
 *  detection completes, as its reset scene does. */
vi.mock("@apps/battle-lab/src/MenuBackdrop", async () => {
  const { useSyncExternalStore } = await import("react");
  const detection = await import("@apps/battle-lab/src/graphicsDetection");
  return {
    MenuBackdrop: function MenuBackdrop(props: { onDetectionSceneReady?: () => void }) {
      const state = useSyncExternalStore(
        detection.subscribeGraphicsDetection,
        detection.graphicsDetectionState,
      );
      useLabLoading("renderer", true);
      useEffect(() => {
        if (state.status === "complete") props.onDetectionSceneReady?.();
      }, [state.status, props]);
      return null;
    },
  };
});

/** The game's catalog, held back while `held` is set, as a first visit's
 *  download holds it. */
const catalogGate = vi.hoisted(() => ({
  held: null as Promise<void> | null,
  game: null as Promise<unknown> | null,
}));
vi.mock("@web/battle/catalog/sets", async (importOriginal) => {
  const real = await importOriginal<typeof import("@web/battle/catalog/sets")>();
  return {
    ...real,
    catalogSet: (set: Parameters<typeof real.catalogSet>[0]) => {
      if (set !== "game" || !catalogGate.held) return real.catalogSet(set);
      catalogGate.game ??= catalogGate.held.then(() => real.catalogSet(set));
      return catalogGate.game as Promise<SessionCatalog>;
    },
  };
});

vi.mock("@web/battle/render/showcaseStage", () => ({
  ShowcaseStage: class {
    constructor() {
      stand.stages += 1;
    }
    async show(unit: { kind: string } | null) {
      stand.shown.push(unit?.kind ?? null);
      return unit ? [{ slot: "engine", sound: "engine_diesel", gain: 0.06, rate: 0.8 }] : [];
    }
    frame() {
      return true;
    }
    dispose() {}
  },
}));

vi.mock("@apps/battle-lab/src/armory/armoryAppearances", () => ({
  ArmoryAppearances: class {
    async catalog() {
      stand.asked += 1;
      return { appearances: {} };
    }
    async show(selection: { kind: string | null }) {
      stand.asked += 1;
      stand.kinds.push(selection.kind);
      if (stand.failure) throw stand.failure;
      return { appearances: { appearances: new Map() }, runtime: {} };
    }
    dispose() {}
  },
}));

// The camera frames the yard alone: the stand-in art draws no yard to frame.
vi.mock("@apps/battle-lab/src/armory/stageFraming", () => ({
  framingOf: (
    _stage: unknown,
    _art: unknown,
    _units: unknown,
    _rules: unknown,
    spot: { camera: unknown },
  ) => ({
    bounds: null,
    pose: spot.camera,
    boxes: [],
    ground: () => 0,
  }),
}));

vi.mock("@apps/battle-lab/src/armory/stageInputs", async (importOriginal) => {
  const real = await importOriginal<typeof import("@apps/battle-lab/src/armory/stageInputs")>();
  const showground = {};
  return { ...real, useShowground: () => showground };
});

const GAME = await catalogSet("game");
const typed = GAME.units.cards.filter((c) => GAME.units.has(c.id));
const planned = GAME.units.cards.find((c) => !GAME.units.has(c.id))!;

beforeEach(() => {
  Object.assign(stand, { stages: 0, shown: [], asked: 0, kinds: [], failure: null });
  Object.assign(catalogGate, { held: null, game: null });
  graphicsSettings.set({ detectedMode: "medium" }, { persist: false });
  vi.stubGlobal("Worker", PreviewWorker);
  vi.stubGlobal("requestAnimationFrame", (f: FrameRequestCallback) =>
    setTimeout(() => f(performance.now()), 16),
  );
  vi.stubGlobal("cancelAnimationFrame", (id: number) => clearTimeout(id));
  vi.spyOn(appResources, "gpu").mockResolvedValue({} as never);
  // jsdom draws no canvases: the frame a switch holds is nothing here.
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
});

/** `navigator.gpu` as the page found it: a test that defines it restores it. */
const gpuAtStart = Object.getOwnPropertyDescriptor(navigator, "gpu");

afterEach(() => {
  cleanup();
  if (gpuAtStart) Object.defineProperty(navigator, "gpu", gpuAtStart);
  else delete (navigator as { gpu?: unknown }).gpu;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  graphicsSettings.set({ mode: "auto", detectedMode: null }, { persist: false });
  window.history.replaceState(null, "", "/");
});

const armoryAt = (card: string) => `${menuPage("armory").path}?unit=${card}`;
const uncovered = () => waitFor(() => expect(screen.queryByTestId("loading")).toBeNull());
const stage = () => document.querySelector<HTMLElement>(".armory-stage")!;
const unit = () => new URLSearchParams(location.search).get("unit");
const tiles = () =>
  within(screen.getByRole("region", { name: "Units" }))
    .getByRole("tabpanel")
    .querySelectorAll<HTMLButtonElement>("button[data-family]");

test("a stage failure stays on the Armory", async () => {
  stand.failure = new Error("bundle 404: the unit's art");
  await visitShell(armoryAt(typed[0].id));
  await uncovered();
  const alert = await within(stage()).findByRole("alert");
  expect(alert.textContent).toContain("bundle 404: the unit's art");
  expect(appResources.error()).toBeNull();
  // The list and the sheet still answer: another pick opens its sheet.
  const other = [...tiles()].find((t) => t.getAttribute("aria-pressed") !== "true")!;
  stand.failure = null;
  await act(async () => fireEvent.click(other));
  expect(unit()).not.toBe(typed[0].id);
  await waitFor(() => expect(within(stage()).queryByRole("alert")).toBeNull());
  expect(stand.shown.at(-1)).toBe(unit());
  // And Back leaves for the menu.
  fireEvent.click(screen.getByRole("button", { name: "Back" }));
  await screen.findByRole("navigation", { name: "Main menu" });
  expect(appResources.error()).toBeNull();
});

test("opening the Armory mid-detection waits and saves the measured tier", async () => {
  graphicsSettings.set({ mode: "auto", detectedMode: null }, { persist: false });
  Object.defineProperty(navigator, "gpu", { configurable: true, value: {} });
  localStorage.removeItem(graphicsSettingsStorageKey);
  await visitShell(armoryAt(typed[0].id));
  await screen.findByRole("region", { name: "Units" });
  // Detection is still to run: the cover holds and no stage is built.
  expect(screen.queryByTestId("loading")).not.toBeNull();
  expect(stand.stages).toBe(0);
  const tiers: string[] = [];
  const unregister = registerGraphicsDetectionRunner(async (tier) => {
    tiers.push(tier);
    return { averageFps: tier === "high" ? 30 : 60, onePercentLowFps: 30 };
  });
  try {
    let detected: string | undefined;
    await act(async () => {
      detected = await runAutoDetect();
    });
    expect(tiers).toEqual(["low", "medium", "high"]);
    expect(detected).toBe("medium");
  } finally {
    unregister();
  }
  expect(graphicsDetectionState().status).toBe("complete");
  expect(JSON.parse(localStorage.getItem(graphicsSettingsStorageKey)!)).toMatchObject({
    detectedMode: "medium",
  });
  await uncovered();
  await waitFor(() => expect(stand.shown).toEqual([typed[0].id]));
  expect(stand.stages).toBe(1);
});

test("a planned card never asks for a unit's art", async () => {
  // Opened on a planned card, the yard stands empty: only the yard's art is
  // asked for, never the planned unit's.
  await visitShell(armoryAt(planned.id));
  await uncovered();
  expect(unit()).toBe(planned.id);
  await waitFor(() => expect(stand.shown).toEqual([null]));
  expect(stand.kinds).toEqual([null]);
  // A typed tile in the planned card's tab: its art is asked for.
  const typedTile = [...tiles()].find((t) => !t.classList.contains("unavailable"));
  expect(typedTile, "a typed family beside the planned one").toBeDefined();
  await act(async () => fireEvent.click(typedTile!));
  await waitFor(() => expect(stand.shown).toHaveLength(2));
  expect(stand.kinds.at(-1)).toBe(stand.shown.at(-1));
  const asked = stand.asked;
  // Back to the planned card: the yard stands empty, and nothing is asked.
  const plannedTile = [...tiles()].find((t) => t.dataset.family === planned.family)!;
  await act(async () => fireEvent.click(plannedTile));
  await waitFor(() => expect(stand.shown.at(-1)).toBeNull());
  expect(stand.asked).toBe(asked);
  expect(stand.kinds).not.toContain(planned.id);
});

test("leaving the Armory stops its loops and keeps the bed", async () => {
  const loops = vi.spyOn(AppAudio.prototype, "showLoops");
  const screens = vi.spyOn(AppAudio.prototype, "setScreen");
  await visitShell(armoryAt(typed[0].id));
  await uncovered();
  await waitFor(() => expect(loops.mock.calls.at(-1)?.[0].length).toBeGreaterThan(0));
  fireEvent.click(screen.getByRole("button", { name: "Back" }));
  await screen.findByRole("navigation", { name: "Main menu" });
  expect(loops.mock.calls.at(-1)?.[0]).toEqual([]);
  // The menu screen never ended, so its bed plays on.
  expect(screens.mock.calls.every(([s]) => s === "menu")).toBe(true);
});

test("an open by address keeps the cover while the roster's catalog loads", async () => {
  // The page's code is in: only the catalog keeps it waiting.
  await import("@apps/battle-lab/src/armory/ArmoryPage");
  const { AppShell } = await import("@apps/battle-lab/src/AppShell");
  let release!: () => void;
  catalogGate.held = new Promise<void>((resolve) => (release = resolve));
  window.history.replaceState(null, "", armoryAt(typed[0].id));
  await act(async () => {
    render(
      <BrowserRouter unstable_useTransitions={false}>
        <AppShell />
      </BrowserRouter>,
    );
    await new Promise((resolve) => setTimeout(resolve, 100));
  });
  // The backdrop is ready, but the page is not: the cover holds.
  expect(screen.queryByRole("region", { name: "Units" })).toBeNull();
  expect(screen.queryByTestId("loading")).not.toBeNull();
  await act(async () => {
    release();
    await catalogGate.game;
  });
  await uncovered();
  await waitFor(() => expect(stand.shown).toEqual([typed[0].id]));
});
