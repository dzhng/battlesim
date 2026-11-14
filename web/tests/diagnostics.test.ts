import { afterEach, expect, test } from "vitest";
import {
  diagnostics,
  diagnosticsEvent,
  diagnosticsReport,
  diagnosticsSource,
} from "../src/diagnostics";

afterEach(() => diagnostics.set(true));
const report = () => JSON.parse(diagnosticsReport({ commit: "abc" }));

test("the report names the build, the page and the machine, and carries each page's live section until it leaves", () => {
  diagnostics.start();
  const remove = diagnosticsSource("battle view", () => ({ canvas: [800, 600] }));
  const broken = diagnosticsSource("broken", () => {
    throw new Error("device lost");
  });
  const r = report();
  expect(r.build).toEqual({ commit: "abc" });
  expect(r.page).toBe(location.pathname + location.search);
  expect(r.machine.devicePixelRatio).toBe(devicePixelRatio);
  expect(r.live["battle view"]).toEqual({ canvas: [800, 600] });
  expect(r.live.broken).toEqual({ error: "Error: device lost" });
  remove();
  broken();
  expect(report().live).toEqual({});
});

test("collection is on unless the player turns it off, and off keeps nothing", () => {
  diagnostics.start();
  console.error("battle resources aborted");
  expect(report().collecting).toBe(true);
  for (let i = 0; i < 100; i++) console.error("device lost");
  const kept = report().errors;
  expect(kept[0]).toMatchObject({ text: "battle resources aborted", count: 1 });
  expect(kept[1]).toMatchObject({ text: "device lost", count: 100 });
  diagnostics.set(false);
  expect(JSON.parse(localStorage.getItem("battle.diagnostics")!)).toEqual({ enabled: false });
  console.error("after opting out");
  const off = report();
  expect([off.collecting, off.frames, off.pointerInputDelayMs, off.errors]).toEqual([
    false,
    false,
    false,
    [],
  ]);
});

test("moments the page marks land on the report's timeline in order, and off keeps none", () => {
  diagnostics.start();
  diagnosticsEvent("scene rebuild", { reason: "graphics quality" });
  diagnosticsEvent("scene installed");
  const { now, events } = report().timeline;
  expect(events.map((e: { kind: string }) => e.kind)).toEqual(["scene rebuild", "scene installed"]);
  expect(events[0].detail).toEqual({ reason: "graphics quality" });
  expect(events[0].at).toBeLessThanOrEqual(events[1].at);
  expect(events[1].at).toBeLessThanOrEqual(now);
  diagnostics.set(false);
  diagnosticsEvent("canvas blank");
  expect(report().timeline).toBe(false);
  diagnostics.set(true);
  expect(report().timeline.events).toEqual([]);
});

test("a failure the player is shown is in the report: what they read, and why", async () => {
  const { render } = await import("@testing-library/react");
  const { createElement } = await import("react");
  const { MemoryRouter } = await import("react-router");
  const { LoadingScreen } = await import("@apps/battle-lab/src/LoadingScreen");
  diagnostics.start();
  const view = render(
    createElement(
      MemoryRouter,
      null,
      createElement(LoadingScreen, {
        title: "Deploying",
        stages: [],
        current: "",
        failure: {
          message: "The battle cannot continue.",
          details: ["effect flipbook cloud01_8x8.png is not an image"],
        },
      }),
    ),
  );
  const shown = report().errors.filter((e: { text: string }) =>
    e.text.includes("The battle cannot continue."),
  );
  expect(shown).toHaveLength(1);
  expect(shown[0].text).toContain("effect flipbook cloud01_8x8.png is not an image");
  view.unmount();
});
