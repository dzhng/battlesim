// @vitest-environment node
import { expect, test } from "vitest";
import {
  DEFAULT_LIGHTING,
  LIGHTING_RANDOM,
  lightWithMapSeed,
  lightingChoices,
  lightingLabel,
  lightingPresetNames,
  parseLighting,
  pickSunAzimuth,
  resolveLighting,
} from "../../apps/battle-lab/src/lobby/lighting";
import { lightForPreset, presentationSettings } from "@web/presentationSettings";

test("Daylight is the default match lighting, labelled from the fixture", () => {
  expect(DEFAULT_LIGHTING).toBe("day");
  expect(lightingLabel("day")).toBe("Daylight");
  expect(lightingLabel("golden_hour")).toBe("Golden hour");
  expect(lightingLabel(LIGHTING_RANDOM)).toBe("Random");
  expect(lightingChoices()).toEqual([LIGHTING_RANDOM, ...lightingPresetNames()]);
  expect(lightingPresetNames()).toEqual(["day", "golden_hour"]);
});

test("parseLighting accepts random and every fixture look, and names a bad value", () => {
  expect(parseLighting(null)).toBe("day");
  expect(parseLighting("")).toBe("day");
  expect(parseLighting("random")).toBe("random");
  expect(parseLighting("day")).toBe("day");
  expect(parseLighting("golden_hour")).toBe("golden_hour");
  expect(parseLighting("dusk")).toEqual({ error: expect.stringMatching(/dusk|random/) });
});

test("Random is a stable index of the sorted looks from the map seed, and every look is reachable", () => {
  expect(resolveLighting("golden_hour", "99")).toBe("golden_hour");
  expect(resolveLighting("day", "99")).toBe("day");
  const names = lightingPresetNames();
  const hits = new Set<string>();
  const first = resolveLighting(LIGHTING_RANDOM, "0");
  for (let i = 0; i < 64; i++) {
    const resolved = resolveLighting(LIGHTING_RANDOM, String(i));
    expect(resolved).toBe(names[i % names.length]);
    expect(resolveLighting(LIGHTING_RANDOM, String(i))).toBe(resolved);
    hits.add(resolved);
  }
  expect(hits).toEqual(new Set(names));
  expect(first).toBe("day");
  expect(resolveLighting(LIGHTING_RANDOM, "1")).toBe("golden_hour");
});

test("golden hour picks due east or west from the map seed, agreeing with native", () => {
  const golden = lightForPreset("golden_hour");
  expect(golden.sun_azimuths).toEqual([0, Math.PI]);
  const table: [string, number][] = [
    ["0", 0],
    ["1", Math.PI],
    ["2", 0],
    ["8", 0],
    ["9", Math.PI],
    ["99", Math.PI],
  ];
  for (const [seed, want] of table) {
    expect(pickSunAzimuth(golden.sun_azimuths ?? [], seed)).toBe(want);
    expect(lightWithMapSeed(golden, seed).sun_azimuth).toBe(want);
  }
  // Day is unchanged by the seed.
  const day = presentationSettings.light;
  expect(day.sun_azimuth).toBe(-0.35);
  expect(lightWithMapSeed(day, "1").sun_azimuth).toBe(day.sun_azimuth);
});
