import { edgeMetric } from "../edgeMetric.mjs";
import { positionError } from "../posMetric.mjs";
import { GROUND } from "./edge.mjs";
export const NEAR = { eye: [1090, 836, 10], target: [1078, 806, 0], fov: 40 };
export default async function ({ spike, shot }) {
  await spike.load("street");
  const eyes = await spike.eyes();
  const recon = eyes.find((e) => e[8] === 2);
  const configs = (process.env.CONFIGS ?? "1024x64,2048x64,4096x64,8192x64,B").split(",");
  for (const [name, cam, band] of [["far", GROUND, { y0: 300, y1: 800 }], ["near", NEAR, { y0: 300, y1: 1080, x0: 300, x1: 1300 }]]) {
    for (const c of configs) {
      const tech = c === "B" ? 2 : 1;
      const [AZ, R] = c.split("x").map(Number);
      await spike.load("street");
      await spike.setEyes([recon]);
      if (tech === 1) await spike.set({ AZ, R });
      await spike.set({ cam, shadowHalf: 150, directional: 0, tech, debug: 1 });
      if (tech === 1) await spike.buildA(); else await spike.sweepB();
      const { raw } = await shot(`pos-${name}-${c}`);
      const e = edgeMetric(raw, { x0: 0, x1: 1920, ...band });
      const pe = positionError(e.pts ?? [], cam, recon, [1064, 800], 0);
      console.log(name, c, JSON.stringify({ stairMaxPx: e.localMaxPx, lineMaxPx: e.lineMaxPx, posMaxPx: pe.maxPx, posMeanPx: pe.meanPx, n: pe.n }));
    }
  }
}
