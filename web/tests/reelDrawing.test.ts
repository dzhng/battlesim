// @vitest-environment node
// What the graphics test films. On its own it times the menu reel without
// the battle view's indicators, recording nothing it would not use; the
// native parity scene films them by default, with the `overlays` layer that
// names them, and none with PARITY_INDICATORS=0; the fog of war only with
// PARITY_FOG=1; each independently of the other.
import { expect, test } from "vitest";
import { reelDrawing } from "@apps/battle-lab/src/benchmark/reelDrawing";
import { parityUrl } from "../scenes/native-parity-capture.mjs";

const BASE = "http://localhost:5173/labs/graphics-test?plates=1";
const filmed = (env: Record<string, string>) =>
  reelDrawing(new URL(parityUrl(BASE, env)).searchParams);

test("the graphics test alone draws and records no indicators", () => {
  const timed = reelDrawing(new URL(BASE).searchParams);
  expect(timed).toMatchObject({ indicators: false, fog: false });
  expect(timed.layers).not.toContain("overlays");
});

test("indicators are filmed by default and left out with PARITY_INDICATORS=0", () => {
  expect(filmed({})).toMatchObject({ indicators: true, fog: false });
  expect(filmed({}).layers).toContain("overlays");
  const clean = filmed({ PARITY_INDICATORS: "0" });
  expect(clean.indicators).toBe(false);
  expect(clean.layers).not.toContain("overlays");
});

test("fog and indicators are chosen independently", () => {
  for (const [env, fog, indicators] of [
    [{ PARITY_FOG: "1" }, true, true],
    [{ PARITY_FOG: "1", PARITY_INDICATORS: "0" }, true, false],
  ] as const) {
    const drawn = filmed(env);
    expect(drawn).toMatchObject({ fog, indicators });
    expect(drawn.layers.includes("fog")).toBe(fog);
    expect(drawn.layers.includes("overlays")).toBe(indicators);
  }
});
