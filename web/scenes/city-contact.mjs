// The benchmark opened by its named address (`?preset=city-contact`), the
// same workload the menu's Benchmark opens; it shares the recording gate.
// BENCHMARK_LENGTH=full selects its 300 s final frame-cost window.
import { run as benchmark } from "./benchmark.mjs";

export async function run(ctx) {
  await benchmark(ctx, true);
}
