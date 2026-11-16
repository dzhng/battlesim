import {
  LIGHTING_RANDOM,
  lightingChoices,
  lightingPresetNames,
} from "../../packages/skirmish-setup/src/lighting";
// @vitest-environment node
import { expect, test } from "vitest";
import {
  DEFAULT_LIGHTING,
  lightWithMapSeed,
  lightingLabel,
  parseLighting,
  resolveLighting,
} from "../../apps/battle-lab/src/lobby/lighting";
import { lightForPreset } from "@web/presentationSettings";

test("the lobby offers Random then every fixture look, each with its label, the default among them", () => {
  const names = lightingPresetNames();
  expect(names).toContain(DEFAULT_LIGHTING);
  expect(lightingChoices()).toEqual([LIGHTING_RANDOM, ...names]);
  for (const choice of lightingChoices()) expect(lightingLabel(choice).trim()).not.toBe("");
});

test("parseLighting accepts random and every fixture look, and names a bad value", () => {
  expect(parseLighting(null)).toBe(DEFAULT_LIGHTING);
  expect(parseLighting("")).toBe(DEFAULT_LIGHTING);
  expect(parseLighting(LIGHTING_RANDOM)).toBe(LIGHTING_RANDOM);
  for (const name of lightingPresetNames()) expect(parseLighting(name)).toBe(name);
  expect(parseLighting("no_such_look")).toEqual({
    error: expect.stringMatching(/random/),
  });
});

test("Random is a stable index of the sorted looks from the map seed, and every look is reachable", () => {
  const names = lightingPresetNames();
  for (const name of names) expect(resolveLighting(name, "99")).toBe(name);
  const hits = new Set<string>();
  for (let i = 0; i < 64; i++) {
    const resolved = resolveLighting(LIGHTING_RANDOM, String(i));
    expect(resolved).toBe(names[i % names.length]);
    expect(resolveLighting(LIGHTING_RANDOM, String(i))).toBe(resolved);
    hits.add(resolved);
  }
  expect(hits).toEqual(new Set(names));
  // A seed that is no non-negative integer is refused, not wrapped.
  expect(() => resolveLighting(LIGHTING_RANDOM, "-3")).toThrow();
});

test("a look with candidate sun azimuths takes one by the map seed, each reachable; one without keeps its own", () => {
  for (const name of lightingPresetNames()) {
    const light = lightForPreset(name);
    const candidates = [...(light.sun_azimuths ?? [])].sort((a, b) => a - b);
    if (candidates.length === 0) {
      expect(lightWithMapSeed(light, "7").sun_azimuth).toBe(light.sun_azimuth);
      continue;
    }
    const picked = new Set<number>();
    for (let seed = 0; seed < 2 * candidates.length; seed++) {
      const azimuth = lightWithMapSeed(light, String(seed)).sun_azimuth;
      expect(azimuth).toBe(candidates[seed % candidates.length]);
      expect(lightWithMapSeed(light, String(seed)).sun_azimuth).toBe(azimuth);
      picked.add(azimuth);
    }
    expect(picked).toEqual(new Set(candidates));
  }
});
