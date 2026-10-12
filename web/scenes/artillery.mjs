// The artillery lab shares the flight bench's deterministic checks; its route
// is a focused catalog entry so the shipped indirect rows get their own gate.
import { run as runFlightBench } from "./ballistics.mjs";

export async function run(ctx) {
  await runFlightBench(ctx, { artilleryOnly: true });
}
