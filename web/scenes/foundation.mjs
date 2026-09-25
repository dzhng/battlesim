// Slice 01: honest 3D frame — camera/depth/picking/lifecycle on the real adapter.
import { writeFile } from "node:fs/promises";
import { decode, pixel, writeCrop } from "./_png.mjs";

// Hue, not brightness: the crate is warm (r > b), the tank cool (b > r), whatever the light.
const isTan = ([r, g, b]) => r > b + 10 && g > b;
const isBlue = ([r, , b]) => b > r + 10;

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  const info = await page.evaluate(() => ({
    adapter: window.__lab.adapter,
    stats: window.__lab.stats(),
    camera: window.__lab.camera(),
    instances: window.__lab.instances(),
  }));
  ctx.check(
    "hardware adapter reported",
    Boolean(info.adapter?.vendor),
    JSON.stringify(info.adapter),
  );
  const depth = info.stats.depth;
  ctx.check(
    "installed depth state is reverse-Z depth32float",
    depth.format === "depth32float" && depth.clearValue === 0 && depth.compare === "greater",
    JSON.stringify(depth),
  );

  // Primary evidence frame and the tank-nose/prop overlap crop.
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath("frame-1280x800.png"), shot);
  const png = decode(shot);
  const tankIndex = info.instances.findIndex((i) => i.kind === "tank");
  const boxIndex = info.instances.findIndex((i) => i.kind === "box");
  const tank = info.instances[tankIndex];
  const box = info.instances[boxIndex];
  const nose = await page.evaluate(
    ([x, y, z]) => window.__lab.projectToCss(x, y, z),
    [box.x, box.y, box.z + 0.6],
  );
  await writeCrop(png, ctx.evidencePath("crop-nose-prop-3x.png"), nose[0], nose[1], 90, 60, 3);
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: info.adapter,
    viewport: [1280, 800],
    dpr: 1,
    camera: info.camera,
    depth,
  });

  // Picking: a visible proxy's anchor selects it; open sky selects nothing.
  const tankPx = await page.evaluate(
    ([x, y, z]) => window.__lab.projectToCss(x, y, z + 1),
    [tank.x, tank.y, tank.z],
  );
  const picked = await page.evaluate(([x, y]) => window.__lab.pickAt(x, y), tankPx);
  ctx.check("clicking the tank picks the tank", picked === tankIndex, `picked ${picked}`);
  await page.mouse.click(tankPx[0], tankPx[1]);
  await page.evaluate(() => window.__lab.frame());
  const selectedText = await page.getByTestId("foundation-panel").textContent();
  ctx.check("selection reaches the panel", selectedText.includes("tank"), selectedText);
  const sky = await page.evaluate(() => window.__lab.pickAt(640, 5));
  ctx.check("sky pick selects nothing", sky === -1, `picked ${sky}`);

  // Depth: along one line of sight the nearer of crate/tank must win, both ways.
  await page.mouse.click(640, 5); // deselect so the tank shows its base colour
  const lineOfSight = async (from, to, name) => {
    const yaw = Math.atan2(from.y - to.y, from.x - to.x);
    await page.evaluate((camera) => window.__lab.setCamera(camera), {
      ...info.camera,
      target: [from.x, from.y, from.z + 0.6],
      yaw,
      pitch: 0.12,
      distance: 14,
    });
    await page.evaluate(() => window.__lab.frame());
    const frame = await page.screenshot();
    await writeFile(ctx.evidencePath(name), frame);
    return pixel(decode(frame), 640, 400);
  };
  const crateFirst = await lineOfSight(box, tank, "depth-crate-first.png");
  ctx.check("crate in front of tank occludes it", isTan(crateFirst), `rgb ${crateFirst}`);
  const tankFirst = await lineOfSight(tank, box, "depth-tank-first.png");
  ctx.check("tank in front of crate occludes it", isBlue(tankFirst), `rgb ${tankFirst}`);

  // Camera controls: wheel zoom changes distance; reset restores the fixture camera.
  await page.evaluate(() => window.__lab.reset());
  await page.mouse.move(640, 400);
  await page.mouse.wheel(0, 400);
  const zoomed = await page.evaluate(() => window.__lab.camera().distance);
  ctx.check(
    "wheel zooms out",
    zoomed > info.camera.distance,
    `${info.camera.distance} → ${zoomed}`,
  );
  const yaw0 = await page.evaluate(() => window.__lab.camera().yaw);
  await page.mouse.move(640, 400);
  await page.mouse.down({ button: "middle" });
  await page.mouse.move(760, 400, { steps: 4 });
  await page.mouse.up({ button: "middle" });
  const yaw1 = await page.evaluate(() => window.__lab.camera().yaw);
  ctx.check("middle drag orbits", Math.abs(yaw1 - yaw0) > 0.1, `${yaw0} → ${yaw1}`);
  const target0 = await page.evaluate(() => window.__lab.camera().target);
  await page.keyboard.press("d");
  const target1 = await page.evaluate(() => window.__lab.camera().target);
  ctx.check("WASD pans", target0.join() !== target1.join(), `${target0} → ${target1}`);
  await page.getByRole("button", { name: "Reset camera" }).click();
  const reset = await page.evaluate(() => window.__lab.camera().distance);
  ctx.check("reset restores the fixture camera", reset === info.camera.distance, `${reset}`);

  // Lifecycle: resize and rebuild cycles return owned GPU resources to baseline.
  const baseline = await page.evaluate(() => window.__lab.allocations());
  for (const [w, h] of [
    [900, 600],
    [1600, 1000],
    [640, 480],
    [1280, 800],
  ]) {
    await page.setViewportSize({ width: w, height: h });
    await page.evaluate(() => window.__lab.frame());
  }
  for (let i = 0; i < 3; i++) await page.evaluate(() => window.__lab.rebuild());
  const after = await page.evaluate(() => window.__lab.allocations());
  ctx.check(
    "resize + rebuild cycles return live GPU buffers/textures to baseline",
    after.buffers === baseline.buffers && after.textures === baseline.textures,
    `baseline ${JSON.stringify(baseline)} after ${JSON.stringify(after)}`,
  );
  const size = await page.evaluate(() => window.__lab.stats());
  ctx.check(
    "render target follows the viewport",
    size.width === 1280 && size.height === 800,
    `${size.width}×${size.height}`,
  );

  // Unsupported GPU is reported, not an endless loading state.
  const bare = await ctx.newPage({ allowErrors: true });
  await bare.addInitScript(() =>
    Object.defineProperty(Navigator.prototype, "gpu", { get: () => undefined }),
  );
  await bare.goto(ctx.url);
  await bare.waitForFunction(() => window.__lab?.error, undefined, { timeout: 15000 });
  const alert = await bare.getByRole("alert").textContent();
  ctx.check("missing WebGPU shows an actionable message", /WebGPU/.test(alert), alert);
}
