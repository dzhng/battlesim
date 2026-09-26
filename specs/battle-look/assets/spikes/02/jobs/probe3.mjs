import { orbit } from "./perf.mjs";
export default async function ({ spike, shot }) {
  await spike.load("endurance");
  const cam = orbit([1500, 1000, 0], 260, 50, 60, 46);
  await spike.set({ R: Number(process.env.R ?? 64) });
  for (const directional of [1]) {
    await spike.set({ cam, shadowHalf: 400, sunElevation: 24, sunAzimuth: 215, tech: 1, debug: 0, directional });
    await spike.shadowPass();
    await spike.buildA();
    await shot(`probe3-endurance-R${process.env.R ?? 64}`);
  }
}
