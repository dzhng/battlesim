// Map resolution sweep for candidate A (and B once): edge stair-stepping at
// ground framing (single street observer), agreement with the sim's field
// (all street eyes and all endurance eyes), memory and build time.
import { edgeMetric } from "../edgeMetric.mjs";
import { GROUND } from "./edge.mjs";
const BAND = { x0: 0, x1: 1920, y0: 250, y1: 800 };
const median = (v) => v.slice().sort((a, b) => a - b)[Math.floor(v.length / 2)];
export default async function ({ spike, shot }) {
  const rows = [];
  const configs = [[512, 64], [1024, 64], [1024, 128], [2048, 64], [2048, 128], [4096, 128]];
  for (const [AZ, R] of configs) {
    const row = { AZ, R };
    // Edge.
    await spike.load("street");
    const eyes = await spike.eyes();
    await spike.setEyes([eyes.find((e) => e[8] === 2)]);
    await spike.set({ AZ, R, cam: GROUND, shadowHalf: 150, directional: 0, tech: 1, debug: 1 });
    await spike.buildA();
    const { raw } = await shot(`sweep-edge-A-${AZ}x${R}`);
    Object.assign(row, Object.fromEntries(Object.entries(edgeMetric(raw, BAND)).map(([k, v]) => ["edge." + k, v])));
    // Agreement, street and endurance, isotropic vs the sim and directional vs the CPU reference.
    for (const name of ["street", "endurance"]) {
      await spike.load(name);
      const m = await spike.set({ AZ, R, tech: 1, directional: 0 });
      const b = [];
      for (let i = 0; i < 5; i++) b.push((await spike.buildA()).buildA);
      const st = await spike.state();
      row[`${name}.eyes`] = st.eyeCount;
      row[`${name}.MiB`] = m.mapBytes / 2 ** 20;
      row[`${name}.buildMs`] = median(b);
      const iso = await spike.agreement("sim");
      row[`${name}.isoPct`] = iso.pctOutside;
      row[`${name}.isoFalseSeen`] = iso.falseSeen; row[`${name}.isoFalseHidden`] = iso.falseHidden;
      await spike.set({ directional: 1 });
      const dir = await spike.agreement("dir");
      row[`${name}.dirPct`] = dir.pctOutside;
    }
    console.log(JSON.stringify(row));
    rows.push(row);
  }
  // Candidate B once.
  const rowB = { tech: "B 1m" };
  await spike.load("street");
  const eyes = await spike.eyes();
  await spike.setEyes([eyes.find((e) => e[8] === 2)]);
  await spike.set({ cam: GROUND, shadowHalf: 150, directional: 0, tech: 2, debug: 1 });
  await spike.sweepB();
  const { raw } = await shot("sweep-edge-B");
  Object.assign(rowB, Object.fromEntries(Object.entries(edgeMetric(raw, BAND)).map(([k, v]) => ["edge." + k, v])));
  for (const name of ["street", "endurance"]) {
    await spike.load(name);
    await spike.set({ tech: 2, directional: 0 });
    const b = [];
    for (let i = 0; i < 5; i++) b.push((await spike.sweepB()).sweepB);
    rowB[`${name}.sweepMs`] = median(b);
    rowB[`${name}.isoPct`] = (await spike.agreement("sim")).pctOutside;
    await spike.set({ directional: 1 });
    await spike.sweepB();
    rowB[`${name}.dirPct`] = (await spike.agreement("dir")).pctOutside;
    const by = await spike.bytes();
    rowB[`${name}.MiB`] = (by.bits + by.raster) / 2 ** 20;
  }
  console.log(JSON.stringify(rowB));
}
