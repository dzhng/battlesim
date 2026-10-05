// @vitest-environment node
import { expect, test } from "vitest";
import {
  BENCHMARK_PRESETS,
  VILLAGE_CONTACT,
  benchmarkFingerprint,
} from "../src/battle/benchmark/presets";

// Each released version's workload. A change to a scenario or its camera tour
// fails here until the scenario gets a new version and a new line.
const RELEASED: Record<string, string> = {
  "village-contact@1": "e8f82e7e",
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
  const moved = { ...VILLAGE_CONTACT, startTick: VILLAGE_CONTACT.startTick + 1 };
  const reanchored = {
    ...VILLAGE_CONTACT,
    tour: { ...VILLAGE_CONTACT.tour, keyframes: VILLAGE_CONTACT.tour.keyframes.slice(1) },
  };
  const renumbered = { ...VILLAGE_CONTACT, version: 99 };
  const base = benchmarkFingerprint(VILLAGE_CONTACT);
  expect(benchmarkFingerprint(moved)).not.toBe(base);
  expect(benchmarkFingerprint(reanchored)).not.toBe(base);
  expect(benchmarkFingerprint(renumbered)).toBe(base);
});
