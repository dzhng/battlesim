export default async function ({ spike, shot }) {
  console.log(await spike.load("street"));
  console.log(await spike.state().then((s) => ({ eyes: s.eyeCount })));
  await spike.set({ cam: { eye: [1000, 800, 900], target: [1000, 801, 0], fov: 40 }, shadowHalf: 600 });
  console.log("shadow", await spike.shadowPass());
  console.log("buildA", await spike.buildA());
  console.log("sweepB", await spike.sweepB());
  for (const tech of [1, 2]) {
    await spike.set({ tech, debug: 1, directional: 0 });
    console.log("tech", tech, "frame", await spike.frame());
    await shot(`smoke-top-${tech}`);
    console.log("agree iso vs sim", await spike.agreement("sim"));
  }
  await spike.set({ tech: 1, debug: 0 });
  await shot("smoke-top-look");
}
