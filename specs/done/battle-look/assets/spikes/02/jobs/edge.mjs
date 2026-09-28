// Single observer (the street recon) and building B's sight shadow at
// Broken Arrow ground framing: debug masks for both candidates.
export const GROUND = { eye: [1150, 858, 12], target: [1150, 824, 0], fov: 40 };
export default async function ({ spike, shot }) {
  await spike.load("street");
  const eyes = await spike.eyes();
  const recon = eyes.find((e) => e[8] === 2);
  console.log("recon eye", recon);
  await spike.setEyes([recon]);
  await spike.set({ cam: GROUND, shadowHalf: 150, directional: 0 });
  await spike.shadowPass();
  await spike.buildA();
  await spike.sweepB();
  for (const tech of [1, 2]) {
    await spike.set({ tech, debug: 1 });
    await shot(`edge-mask-${tech}`);
    await spike.set({ debug: 0 });
    await shot(`edge-look-${tech}`);
  }
  await spike.set({ tech: 1, debug: 1, cam: { eye: [1060, 810, 400], target: [1060, 811, 0], fov: 40 } });
  await shot("edge-top-1");
}
