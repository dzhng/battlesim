// Correctness of a map configuration (no timing): edge stair and position at
// ground framing, agreement with the simulation's field and the directional
// CPU reference (all eyes, and vehicle eyes alone).
import { edgeMetric } from "../edgeMetric.mjs";
import { positionError } from "../posMetric.mjs";
import { GROUND } from "./edge.mjs";
import { NEAR } from "./position.mjs";
export default async function ({ spike, shot }) {
  const configs = (process.env.CONFIGS ?? "4096x64x512,4096x64x0").split(",");
  for (const c of configs) {
    const [AZ, R, AZT] = c.split("x").map(Number);
    const cfg = { AZ, R, AZT: AZT || 512, split: AZT ? 1 : 0 };
    const row = { c };
    await spike.load("street");
    const eyes = await spike.eyes();
    const recon = eyes.find((e) => e[8] === 2);
    for (const [name, cam, band] of [["far", GROUND, { y0: 300, y1: 800 }], ["near", NEAR, { y0: 300, y1: 1080, x0: 300, x1: 1300 }]]) {
      await spike.load("street");
      await spike.setEyes([recon]);
      await spike.set({ ...cfg, cam, shadowHalf: 150, directional: 0, tech: 1, debug: 1 });
      await spike.buildA();
      const { raw } = await shot(`check-${name}-${c}`);
      const e = edgeMetric(raw, { x0: 0, x1: 1920, ...band });
      const pe = positionError(e.pts ?? [], cam, recon, [1064, 800], 0);
      row[`${name}.stairPx`] = e.localMaxPx; row[`${name}.posMaxPx`] = pe.maxPx; row[`${name}.posMeanPx`] = pe.meanPx;
    }
    for (const name of ["street", "endurance"]) {
      await spike.load(name);
      await spike.set({ ...cfg, tech: 1, directional: 0 });
      await spike.buildA();
      const iso = await spike.agreement("sim");
      row[`${name}.iso%`] = iso.pctOutside; row[`${name}.iso.fs/fh`] = `${iso.falseSeen}/${iso.falseHidden} of ${iso.outside}`;
      await spike.set({ directional: 1 });
      row[`${name}.dir%`] = (await spike.agreement("dir")).pctOutside;
      if (name === "endurance") {
        const veh = (await spike.eyes()).filter((e) => e[8] === 3 || e[8] === 4);
        await spike.setEyes(veh);
        await spike.set({ ...cfg, tech: 1, directional: 1 });
        await spike.buildA();
        const v = await spike.agreement("dirVeh");
        row["vehicles.eyes"] = veh.length; row["vehicles.dir%"] = v.pctOutside; row["vehicles.dir.seenCells"] = v.outside;
      }
    }
    console.log(JSON.stringify(row));
  }
}
