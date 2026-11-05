// @vitest-environment node
// What the graphics test films, as the native parity scene asks for it: the
// battle view's indicators by default and the `overlays` layer that names
// them, none with PARITY_INDICATORS=0; the fog of war only with
// PARITY_FOG=1; each independently of the other.
import { expect, test } from "vitest";
import { reelDrawing } from "@apps/battle-lab/src/benchmark/reelDrawing";
import { parityUrl } from "../scenes/native-parity-capture.mjs";

const BASE = "http://localhost:5173/labs/graphics-test?plates=1";
const filmed = (env: Record<string, string>) =>
  reelDrawing(new URL(parityUrl(BASE, env)).searchParams);

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
