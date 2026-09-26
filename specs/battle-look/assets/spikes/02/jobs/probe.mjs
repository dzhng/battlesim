export default async function ({ spike, shot }) {
  await spike.load("village");
  const eyes = await spike.eyes();
  console.log(eyes.map((e) => e.map((v) => +v.toFixed(1)).join(",")).join("\n"));
  await spike.set({ split: Number(process.env.SPLIT ?? 1), cam: { eye: [950, 690, 22], target: [980, 750, 4], fov: 40 }, shadowHalf: 200, directional: 1, tech: 1, debug: 0 });
  await spike.shadowPass();
  await spike.buildA();
  await shot(`probe-A-split${process.env.SPLIT ?? 1}`);
  await spike.set({ debug: 1 });
  await shot("probe-A-mask");
  await spike.setEyes([eyes.find((e) => e[8] === 2)]);
  await spike.buildA();
  await shot("probe-A-mask-recon");
}
