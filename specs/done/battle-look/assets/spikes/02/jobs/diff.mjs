import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
const { PNG } = createRequire("/Users/david/dev/battlegame/web/package.json")("pngjs");
// Where the GPU fog (at 8 m cell centres) and the simulation's field disagree:
// red = GPU seen / sim unseen, blue = GPU unseen / sim seen. North up, 4x.
export default async function ({ spike, out }) {
  for (const name of ["village", "street", "endurance"]) {
    await spike.load(name);
    await spike.set({ tech: 1, directional: 0 });
    await spike.buildA();
    console.log(name, JSON.stringify(await spike.agreement("sim")));
    const d = await spike.diff();
    const [nx, ny] = await spike.fogDims();
    const S = 4, png = new PNG({ width: nx * S, height: ny * S });
    for (let y = 0; y < ny * S; y++) for (let x = 0; x < nx * S; x++) {
      const i = Math.floor(x / S), j = ny - 1 - Math.floor(y / S), v = d[j * nx + i];
      png.data.set(v === 2 ? [255, 60, 60, 255] : v === 1 ? [60, 120, 255, 255] : [40, 40, 40, 255], (y * nx * S + x) * 4);
    }
    writeFileSync(`${out}diff-${name}.png`, PNG.sync.write(png));
  }
}
