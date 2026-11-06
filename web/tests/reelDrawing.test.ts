// @vitest-environment node
// What the graphics test films. On its own it times the menu reel without
// the battle view's indicators or unit info cards, recording nothing it
// would not use; the native parity scene films its indicators by default,
// with the `overlays` layer that names them, none with PARITY_INDICATORS=0;
// its cards compact by default with the `cards` layer, open with
// PARITY_CARDS=open, none with PARITY_CARDS=0; the fog of war only with
// PARITY_FOG=1; each independently of the others.
import { expect, test } from "vitest";
import { reelDrawing } from "@apps/battle-lab/src/benchmark/reelDrawing";
import { parityUrl } from "../scenes/native-parity-capture.mjs";

const BASE = "http://localhost:5173/labs/graphics-test?plates=1";
const filmed = (env: Record<string, string>) =>
  reelDrawing(new URL(parityUrl(BASE, env)).searchParams);

test("the graphics test alone draws and records no indicators or cards", () => {
  const timed = reelDrawing(new URL(BASE).searchParams);
  expect(timed).toMatchObject({ indicators: false, cards: "off", fog: false });
  expect(timed.layers).not.toContain("overlays");
  expect(timed.layers).not.toContain("cards");
});

test("indicators are filmed by default and left out with PARITY_INDICATORS=0", () => {
  expect(filmed({})).toMatchObject({ indicators: true, fog: false });
  expect(filmed({}).layers).toContain("overlays");
  const clean = filmed({ PARITY_INDICATORS: "0" });
  expect(clean.indicators).toBe(false);
  expect(clean.layers).not.toContain("overlays");
});

test("cards are filmed compact by default, open with PARITY_CARDS=open, and left out with PARITY_CARDS=0", () => {
  expect(filmed({}).cards).toBe("compact");
  expect(filmed({}).layers).toContain("cards");
  expect(filmed({ PARITY_CARDS: "open" })).toMatchObject({ cards: "open" });
  const clean = filmed({ PARITY_CARDS: "0", PARITY_INDICATORS: "0" });
  expect(clean).toMatchObject({ cards: "off", indicators: false });
  expect(clean.layers).not.toContain("cards");
  expect(clean.layers).not.toContain("overlays");
});

test("cards, indicators and fog are chosen independently", () => {
  for (const [env, cards, indicators, fog] of [
    [{ PARITY_INDICATORS: "0" }, "compact", false, false],
    [{ PARITY_CARDS: "0" }, "off", true, false],
    [{ PARITY_CARDS: "0", PARITY_FOG: "1" }, "off", true, true],
  ] as const) {
    const drawn = filmed(env);
    expect(drawn, JSON.stringify(env)).toMatchObject({ cards, indicators, fog });
    expect(drawn.layers.includes("cards")).toBe(cards !== "off");
  }
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
