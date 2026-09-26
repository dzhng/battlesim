// Critique follow-ups: (1) building C's south wall in village-start with the
// recon alone, foliage on/off; (2) endurance wedge-end notches, mask crop.
export default async function ({ spike, shot }) {
  await spike.load("village");
  const eyes = await spike.eyes();
  const recon = eyes.find((e) => e[8] === 2);
  await spike.setEyes([recon]);
  const cam = { eye: [985, 820, 12], target: [985, 862, 4], fov: 40 };
  await spike.set({ cam, shadowHalf: 150, directional: 1, tech: 1, debug: 1 });
  await spike.shadowPass();
  await spike.buildA();
  await shot("probe2-wallC-recon-mask");
  await spike.set({ tech: 2 });
  await spike.sweepB();
  await shot("probe2-wallC-recon-mask-B");
  await spike.load("endurance");
  await spike.set({ cam: { eye: [1500 - 260 * Math.cos(0.87) * Math.cos(1.05), 1000 - 260 * Math.cos(0.87) * Math.sin(1.05), 260 * Math.sin(0.87)], target: [1500, 1000, 0], fov: 46 }, tech: 1, debug: 1 });
  await spike.buildA();
  await shot("probe2-endurance-mask");
}
