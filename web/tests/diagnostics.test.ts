import { afterEach, expect, test } from "vitest";
import { diagnostics, diagnosticsReport, diagnosticsSource } from "../src/diagnostics";

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
  expect(report().errors.some((e: string) => e.includes("battle resources aborted"))).toBe(true);
  diagnostics.set(false);
  expect(JSON.parse(localStorage.getItem("battle.diagnostics")!)).toEqual({ enabled: false });
  console.error("after opting out");
  const off = report();
  expect([off.collecting, off.frames, off.pointerInputDelayMs, off.errors]).toEqual([false, false, false, []]);
});
