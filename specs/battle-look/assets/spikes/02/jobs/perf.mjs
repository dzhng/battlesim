// GPU cost at 1920x1080: per pass medians for no fog, sight lights (A) and
// the 1 m viewshed (B), at ground / default / strategic framing, for the
// village street (8 eyes), endurance (96) and stress (both sides, 193).
export const orbit = (target, distance, pitchDeg, yawDeg, fov) => {
  const p = (pitchDeg * Math.PI) / 180, y = (yawDeg * Math.PI) / 180;
  return { eye: [target[0] - distance * Math.cos(p) * Math.cos(y), target[1] - distance * Math.cos(p) * Math.sin(y),
    target[2] + distance * Math.sin(p)], target, fov };
};
const med = (v) => v.slice().sort((a, b) => a - b)[Math.floor(v.length / 2)];
const p95 = (v) => v.slice().sort((a, b) => a - b)[Math.floor(v.length * 0.95)];
export default async function ({ spike, shot }) {
  const [AZ, R] = (process.env.MAP ?? "4096x64").split("x").map(Number);
  const N = Number(process.env.FRAMES ?? 150);
  const scen = { street: [1000, 800, 0], endurance: [1500, 1000, 0], stress: [1500, 1000, 0] };
  const out = [];
  for (const [name, t] of Object.entries(scen)) {
    const info = await spike.load(name);
    await spike.set({ AZ, R, directional: 1 });
    const cams = {
      ground: orbit([t[0] + 10, t[1] + 5, 0], 32, 22, 60, 40),
      default: orbit(t, 260, 50, 60, 46),
      strategic: orbit(t, name === "street" ? 1150 : 2400, 54, 60, 46),
    };
    const bytes = await spike.bytes();
    const row0 = { name, eyes: info.eyes, AZ, R, mapsMiB: bytes.maps / 2 ** 20, tileListsMiB: (bytes.lists + bytes.counts) / 2 ** 20,
      viewshedMiB: bytes.bits / 2 ** 20, rasterMiB: bytes.raster / 2 ** 20, terrMiB: bytes.terr / 2 ** 20 };
    out.push(row0);
    for (const [cname, cam] of Object.entries(cams)) {
      await spike.set({ cam, shadowHalf: cname === "ground" ? 150 : cname === "default" ? 400 : 1500 });
      await spike.shadowPass();
      const row = { name, cam: cname };
      await spike.buildA(); await spike.sweepB();
      await spike.batch(30, [0, 1, 2]);
      const r = await spike.batch(N, [0, 1, 2], { build: cname === "ground" });
      for (const [tech, passes] of Object.entries(r)) for (const [pass, v] of Object.entries(passes)) {
        row[`${tech}.${pass}`] = med(v); row[`${tech}.${pass}.p95`] = p95(v);
      }
      const pairA = r[1].main.map((m, i) => m + r[1].cull[i] - r[0].main[i]);
      const pairB = r[2].main.map((m, i) => m - r[0].main[i]);
      row["A.fog_ms"] = med(pairA); row["A.fog_ms.p95"] = p95(pairA);
      row["B.fog_ms"] = med(pairB); row["B.fog_ms.p95"] = p95(pairB);
      await spike.set({ tech: 1, debug: 0 });
      Object.assign(row, Object.fromEntries(Object.entries(await spike.tileStats()).map(([k, v]) => [`tiles.${k}`, v])));
      for (const [tech, label] of [[0, "none"], [1, "A"], [2, "B"]]) {
        await spike.set({ tech, debug: 0 });
        await shot(`perf-${name}-${cname}-${label}`);
      }
      if (r.update) { row0.buildA_full_ms = (row["update.buildT"] ?? 0) + (row["update.merge"] ?? 0) + (row["update.buildA"] ?? 0); row0.buildT_ms = row["update.buildT"]; row0.merge_ms = row["update.merge"]; row0.buildA_per_eye_ms = row0.buildA_full_ms / info.eyes; row0.sweepB_full_ms = row["update.sweepB"]; console.log(JSON.stringify(row0)); }
      console.log(JSON.stringify(row));
      out.push(row);
    }
  }
  const fs = await import("node:fs");
  fs.writeFileSync(new URL(`../perf-${AZ}x${R}.json`, import.meta.url), JSON.stringify(out, null, 1));
}
