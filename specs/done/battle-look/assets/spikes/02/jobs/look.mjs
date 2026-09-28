// Look frames for both candidates (evidence for compare-screenshots and the
// unprimed critique). Directional sight on; low afternoon sun.
import { orbit } from "./perf.mjs";
import { GROUND } from "./edge.mjs";
import { NEAR } from "./position.mjs";
export default async function ({ spike, shot }) {
  const techs = [[1, "A"], [2, "B"]];
  const frame = async (label, setup, cam, shadowHalf) => {
    for (const [tech, t] of techs) {
      await setup();
      await spike.set({ cam, shadowHalf, sunElevation: 24, sunAzimuth: 215, directional: 1, tech, debug: 0 });
      await spike.shadowPass();
      if (tech === 1) await spike.buildA(); else await spike.sweepB();
      await shot(`look-${label}-${t}`);
      await spike.set({ debug: 1 });
      await shot(`look-${label}-${t}-mask`);
    }
  };
  // The village at the start: blue's eyes on the west, the recon's sight
  // reaching the village and cut by its buildings.
  await frame("village-start", () => spike.load("village"), orbit([1010, 810, 0], 230, 48, 20, 46), 350);
  // One observer (the recon) in the village street, ARMAPHRACT-like oblique.
  const single = async () => {
    await spike.load("street");
    const eyes = await spike.eyes();
    await spike.setEyes([eyes.find((e) => e[8] === 2)]);
  };
  await frame("street-oblique", single, orbit([1030, 812, 0], 150, 52, 35, 46), 250);
  await frame("street-ground", single, GROUND, 150);
  await frame("street-corner", single, NEAR, 150);
  // All blue eyes in the street.
  await frame("street-all", () => spike.load("street"), orbit([1010, 810, 0], 180, 50, 35, 46), 300);
  // Endurance, 100 a side, default framing over the middle village.
  await frame("endurance", () => spike.load("endurance"), orbit([1500, 1000, 0], 260, 50, 60, 46), 400);
}
