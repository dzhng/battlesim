// The named generated contact benchmark shares the recording/camera gate.
// BENCHMARK_LENGTH=full selects its 300 s final frame-cost window.
import { run as benchmark } from "./benchmark.mjs";

export async function run(ctx) {
  await benchmark(ctx, "city-contact");
}
