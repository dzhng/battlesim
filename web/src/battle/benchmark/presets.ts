// The released benchmark workload: explicitly synthetic central city contact
// on a complete generated world. Its version pins scenario and tour inputs.
// Rule changes are compared on purpose; reports additionally carry the
// checked request and resolved map identity.
import { CITY_CONTACT_TOUR, type BenchmarkTour } from "./camera";
import type { MapChoice } from "../../maps/source";

export type BenchmarkLength = "full" | "short";

export interface BenchmarkScenario {
  id: string;
  version: number;
  /** The simulation's stress scene. */
  variant: "city-arena-2";
  /** The generated map the production preparation owner resolves. */
  generated: MapChoice;
  seed: number;
  /** Both sides' seeded orders are carried by the simulation-owned scenario. */
  blue: "scenario-orders";
  /** Timing starts here, after stepping the real simulation to it. */
  startTick: number;
  durationMs: Readonly<Record<BenchmarkLength, number>>;
  tour: BenchmarkTour;
}

/** A preset names its camera policy; coordinates only exist after
 * preparation has returned the actual extent. Recording takes a resolved scenario. */
export type BenchmarkPreset = Omit<BenchmarkScenario, "tour"> & { tour: typeof CITY_CONTACT_TOUR };

export const CITY_CONTACT = {
  id: "city-contact",
  version: 5,
  variant: "city-arena-2",
  generated: { type: "metro", size: "xl", seed: "4" },
  seed: 4,
  blue: "scenario-orders",
  startTick: 150,
  durationMs: { full: 300_000, short: 60_000 },
  tour: CITY_CONTACT_TOUR,
} satisfies BenchmarkPreset;

export const BENCHMARK_PRESETS: readonly BenchmarkPreset[] = [CITY_CONTACT];

/** The preset `preset` names, `city-contact` when none is named; a name
 *  that is not a preset is refused (undefined), never silently replaced. */
export function benchmarkPreset(preset: string | null): BenchmarkPreset | undefined {
  return preset === null ? CITY_CONTACT : BENCHMARK_PRESETS.find((s) => s.id === preset);
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
