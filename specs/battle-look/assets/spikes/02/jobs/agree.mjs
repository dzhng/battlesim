// Final agreement table: GPU fog at 8 m cell centres vs the simulation's field
// (isotropic, today's rule) and vs the directional CPU reference, outside a
// one-cell band around the simulation's own boundary.
export default async function ({ spike }) {
  const r = (a) => ({ pct: +a.pctOutside.toFixed(3), ofSeen: +a.pctOfSeen.toFixed(3), fs: a.falseSeen, fh: a.falseHidden,
    outside: a.outside, simSeen: a.simSeen, forestPct: +a.forestPct.toFixed(3), forestCells: a.forestCells });
  await spike.set({ simFoliage: Number(process.env.SIMFOL ?? 0), split: Number(process.env.SPLIT ?? 1), AZT: Number(process.env.AZT ?? 512) });
  for (const [tech, label] of (process.env.TECHS === "A" ? [[1, "A"]] : [[1, "A"], [2, "B"]])) {
    for (const name of ["village", "street", "endurance", "stress"]) {
      await spike.load(name);
      await spike.set({ tech, directional: 0 });
      if (tech === 1) await spike.buildA(); else await spike.sweepB();
      const iso = r(await spike.agreement("sim"));
      await spike.set({ directional: 1 });
      if (tech === 1) await spike.buildA(); else await spike.sweepB();
      const dir = r(await spike.agreement("dir"));
      let veh = null;
      if (name === "endurance") {
        const v = (await spike.eyes()).filter((e) => e[8] === 3 || e[8] === 4);
        await spike.setEyes(v);
        await spike.set({ tech, directional: 1 });
        if (tech === 1) await spike.buildA(); else await spike.sweepB();
        veh = r(await spike.agreement("dirVeh"));
      }
      console.log(JSON.stringify({ label, name, iso, dir, veh }));
    }
  }
}
