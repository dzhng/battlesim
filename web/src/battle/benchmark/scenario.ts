// The benchmark's scenario: a real village battle,
// both sides scripted, warm-started to heavy contact, then timed while the
// camera flies its tour. The version pins the workload: the scenario fields
// and the tour. Fixture rule tuning is not pinned; frame-cost rows compare
// rule changes on purpose.
//
// Another preset (the endurance battle, say) is another entry here.
//
// Ported by technique from ~/dev/game/web/src/battle/benchmark/
// benchmarkScenario.ts.
import { BENCHMARK_TOUR, type BenchmarkTour } from "./camera";

export type BenchmarkLength = "full" | "short";

export interface BenchmarkScenario {
  id: string;
  version: number;
  /** The village fixture's variant and the battle seed. */
  variant: "ordinary" | "prepared_crossfire";
  seed: number;
  /** Blue's comparison script (`village_report`'s name). Red is always the
   *  fixture's defender policy. */
  blue: "scout-suppress-flank";
  /** Timing starts here, after stepping the real simulation to it. */
  startTick: number;
  durationMs: Readonly<Record<BenchmarkLength, number>>;
  tour: BenchmarkTour;
}

export const VILLAGE_CONTACT: BenchmarkScenario = {
  id: "village-contact",
  version: 1,
  variant: "ordinary",
  seed: 20260925,
  blue: "scout-suppress-flank",
  // 210 s: blue's line and the village garrison exchange 30–50 rounds a tick
  // from here to beyond 500 s.
  startTick: 6300,
  durationMs: { full: 300_000, short: 60_000 },
  tour: BENCHMARK_TOUR,
};

export const BENCHMARK_SCENARIOS: readonly BenchmarkScenario[] = [VILLAGE_CONTACT];

/** A stable hash of everything that shapes the workload (FNV-1a, 32 bit). */
export function benchmarkFingerprint(scenario: BenchmarkScenario): string {
  const { version: _version, ...workload } = scenario;
  const text = JSON.stringify(workload);
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}
