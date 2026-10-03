// Released benchmark workloads: the village control and explicitly synthetic
// central city contact on a complete generated world. Versions pin scenario
// and tour inputs. Rule changes are compared on purpose; generated reports
// additionally carry the checked request and resolved map identity.
import { BENCHMARK_TOUR, CITY_CONTACT_TOUR, type BenchmarkTour } from "./camera";
import type { MapChoice } from "../../maps/source";

export type BenchmarkLength = "full" | "short";

export interface BenchmarkScenario {
  id: string;
  version: number;
  /** The authoritative scenario builder's variant. */
  variant: "ordinary" | "prepared_crossfire" | "city-arena-2";
  /** Generated local-contact stress; the production preparation owner
   * resolves this choice on the complete map. */
  generated?: MapChoice;
  seed: number;
  /** A village comparison commander, or both sides' seeded orders already
   * carried by the simulation-owned scenario. */
  blue: "scout-suppress-flank" | "scenario-orders";
  /** Timing starts here, after stepping the real simulation to it. */
  startTick: number;
  durationMs: Readonly<Record<BenchmarkLength, number>>;
  tour: BenchmarkTour;
}

/** A generated preset names its camera policy; coordinates only exist after
 * preparation has returned the actual extent. Recording takes a resolved scenario. */
export type BenchmarkPreset = Omit<BenchmarkScenario, "tour" | "generated" | "variant" | "blue"> &
  (
    | {
        variant: "ordinary" | "prepared_crossfire";
        generated?: never;
        blue: "scout-suppress-flank";
        tour: BenchmarkTour;
      }
    | {
        variant: "city-arena-2";
        generated: MapChoice;
        blue: "scenario-orders";
        tour: typeof CITY_CONTACT_TOUR;
      }
  );

export const VILLAGE_CONTACT = {
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
} satisfies BenchmarkPreset;

export const CITY_CONTACT = {
  id: "city-contact",
  version: 4,
  variant: "city-arena-2",
  generated: { type: "metro", size: "large", seed: "4" },
  seed: 4,
  blue: "scenario-orders",
  startTick: 150,
  durationMs: { full: 300_000, short: 60_000 },
  tour: CITY_CONTACT_TOUR,
} satisfies BenchmarkPreset;

export const BENCHMARK_PRESETS: readonly BenchmarkPreset[] = [VILLAGE_CONTACT, CITY_CONTACT];

/** No named selection may silently fall back to the village control. */
export function benchmarkPreset(preset: string | null): BenchmarkPreset | undefined {
  return preset === null ? VILLAGE_CONTACT : BENCHMARK_PRESETS.find((s) => s.id === preset);
}

/** A stable hash of everything that shapes the workload (FNV-1a, 32 bit). */
export function benchmarkFingerprint(scenario: BenchmarkPreset | BenchmarkScenario): string {
  const { version: _version, ...workload } = scenario;
  const text = JSON.stringify(workload);
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}
