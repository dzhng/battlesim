// @vitest-environment node
import { expect, test } from "vitest";
import {
  BENCHMARK_PRESETS,
  CITY_CONTACT,
  benchmarkFingerprint,
  benchmarkPreset,
} from "../src/battle/benchmark/presets";

// Each released version's workload. A change to a scenario or its camera tour
// fails here until the scenario gets a new version and a new line.
const RELEASED: Record<string, string> = {
  "city-contact@1": "968aad93",
  "city-contact@2": "2da54b2a",
  "city-contact@3": "c166ad91",
  "city-contact@4": "5d4eaef8",
  "city-contact@5": "0d8e57f5",
};

test("a scenario's workload is pinned to its version", () => {
  for (const s of BENCHMARK_PRESETS)
    expect(benchmarkFingerprint(s), `${s.id}@${s.version}: bump the version`).toBe(
      RELEASED[`${s.id}@${s.version}`],
    );
});

test("the fingerprint sees the tour and the start, not the version", () => {
  const moved = { ...CITY_CONTACT, startTick: CITY_CONTACT.startTick + 1 };
  const reanchored = { ...CITY_CONTACT, tour: "another-tour" };
  const renumbered = { ...CITY_CONTACT, version: 99 };
  const base = benchmarkFingerprint(CITY_CONTACT);
  expect(benchmarkFingerprint(moved)).not.toBe(base);
  expect(benchmarkFingerprint(reanchored)).not.toBe(base);
  expect(benchmarkFingerprint(renumbered)).toBe(base);
});

test("the default benchmark is city contact, and an unknown preset is refused", () => {
  expect(benchmarkPreset(null)).toBe(CITY_CONTACT);
  expect(benchmarkPreset("city-contact")).toBe(CITY_CONTACT);
  expect(benchmarkPreset("village-contact")).toBeUndefined();
});
