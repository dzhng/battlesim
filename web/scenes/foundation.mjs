// Slice 01: honest 3D frame — camera/depth/picking/lifecycle on the real adapter.
// The tank and truck are their appearances (models), the
// crate and soldiers proxies; picking reads the simulation's boxes.
import { writeFile } from "node:fs/promises";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { menuShown, openMenuPage } from "./_lab.mjs";

// The crate is a pale warm proxy (r > b); the tank's camouflage is dark
// (retuned when the tank was a blue proxy, told apart by hue).
const isTan = ([r, g, b]) => r > b + 10 && g > b;
const luminance = ([r, g, b]) => 0.2126 * r + 0.7152 * g + 0.0722 * b;

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  const info = await page.evaluate(() => ({
    adapter: window.__lab.adapter,
    stats: window.__lab.stats(),
    camera: window.__lab.camera(),
    placed: window.__lab.route.placed,
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
  const tankIndex = info.placed.findIndex((i) => i.kind === "tank");
  const boxIndex = info.placed.findIndex((i) => i.kind === "box");
  const tank = info.placed[tankIndex];
  const box = info.placed[boxIndex];
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

  // Picking: a point on the tank's hull selects it; open sky selects nothing.
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
  ctx.check(
    "tank in front of crate occludes it",
    !isTan(tankFirst) || luminance(tankFirst) < 0.6 * luminance(crateFirst),
    `rgb ${tankFirst} against the crate's ${crateFirst}`,
  );

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
  // Held keys (CameraController): D and the arrows pan, Q turns; letting go
  // or losing focus stops them.
  const camera = () => page.evaluate(() => window.__lab.camera());
  const hold = async (key, ms = 300) => {
    const before = await camera();
    await page.keyboard.down(key);
    await page.waitForTimeout(ms);
    await page.keyboard.up(key);
    return [before, await camera()];
  };
  const centreMoves = async (key) => {
    const [before, after] = await hold(key);
    const was = await page.evaluate((t) => window.__lab.projectToCss(...t), before.target);
    return { dx: was[0] - 640, dy: was[1] - 400, after };
  };
  // The old centre slides left when D pans right, and so on for each key.
  const d = await centreMoves("d");
  const w = await centreMoves("w");
  const right = await centreMoves("ArrowRight");
  ctx.check(
    "held D and ArrowRight pan right, W pans forward",
    d.dx < -20 && right.dx < -20 && w.dy > 20,
    JSON.stringify({ d, w, right }),
  );
  const [q0, q1] = await hold("q");
  ctx.check("held Q turns the view", q1.yaw - q0.yaw > 0.1, `${q0.yaw} → ${q1.yaw}`);
  await page.keyboard.down("a");
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  const released = await camera();
  await page.waitForTimeout(200);
  const still = await camera();
  await page.keyboard.up("a");
  ctx.check(
    "losing focus releases held keys",
    released.target.join() === still.target.join(),
    `${released.target} → ${still.target}`,
  );
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
    "resize + rebuild cycles return live GPU buffers/textures, and their bytes, to baseline",
    after.buffers === baseline.buffers &&
      after.textures === baseline.textures &&
      after.bufferBytes === baseline.bufferBytes &&
      after.textureBytes === baseline.textureBytes,
    `baseline ${JSON.stringify(baseline)} after ${JSON.stringify(after)}`,
  );
  const size = await page.evaluate(() => window.__lab.stats());
  ctx.check(
    "render target follows the viewport",
    size.width === 1280 && size.height === 800,
    `${size.width}×${size.height}`,
  );

  // Menu warm-up and repeated visits share one device; each viewport releases
  // its buffers/textures without destroying that page's admission.
  const visits = await ctx.newPage();
  await visits.addInitScript(() => {
    const probe = { devices: [], destroyed: 0, allocations: null };
    window.__pageGpuProbe = probe;
    const gpu = navigator.gpu;
    const requestAdapter = gpu.requestAdapter.bind(gpu);
    gpu.requestAdapter = async (...args) => {
      const adapter = await requestAdapter(...args);
      if (!adapter) return adapter;
      const requestDevice = adapter.requestDevice.bind(adapter);
      adapter.requestDevice = async (...args) => {
        const device = await requestDevice(...args);
        probe.devices.push(device);
        const destroy = device.destroy.bind(device);
        device.destroy = () => {
          probe.destroyed++;
          destroy();
        };
        return device;
      };
      return adapter;
    };
  });
  await visits.goto(new URL(ctx.url).origin);
  await menuShown(visits);
  await visits.waitForFunction(() => window.__pageGpuProbe.devices.length === 1);
  ctx.check(
    "menu warm-up starts one battle worker, its backdrop's",
    visits.workers().length === 1,
    `${visits.workers().length} workers`,
  );
  const origin = await visits.evaluate(() => performance.timeOrigin);
  const visitCounts = [];
  for (let i = 0; i < 2; i++) {
    await openMenuPage(visits, "Developer");
    await visits.getByRole("link", { name: "Labs", exact: true }).click();
    await visits.getByRole("link", { name: "foundation", exact: true }).click();
    await visits.waitForFunction(() => window.__lab?.ready);
    await visits.evaluate(() => {
      window.__pageGpuProbe.allocations = window.__lab.allocations;
    });
    await visits.goBack();
    await visits.getByRole("link", { name: "Main menu", exact: true }).click();
    // Read in the same moment the visit's allocations have all returned:
    // the menu's backdrop allocates its own scene soon after.
    const released = await visits.waitForFunction(() => {
      const n = window.__pageGpuProbe.allocations();
      return (
        n.buffers === 0 &&
        n.textures === 0 && {
          devices: window.__pageGpuProbe.devices.length,
          destroyed: window.__pageGpuProbe.destroyed,
          allocations: n,
          sameDocument: performance.timeOrigin,
        }
      );
    });
    visitCounts.push(await released.jsonValue());
    await menuShown(visits);
  }
  ctx.check(
    "client visit disposal returns allocations to zero and retains one live page GPU",
    visitCounts.every(
      (n) =>
        n.devices === 1 &&
        n.destroyed === 0 &&
        n.allocations.buffers === 0 &&
        n.allocations.textures === 0 &&
        n.sameDocument === origin,
    ),
    JSON.stringify(visitCounts),
  );
  await ctx.writeEvidence("app-resource-visits.json", visitCounts);

  // Unsupported GPU is reported, not an endless loading state.
  const bare = await ctx.newPage({ allowErrors: true });
  await bare.addInitScript(() =>
    Object.defineProperty(Navigator.prototype, "gpu", { get: () => undefined }),
  );
  await bare.goto(ctx.url);
  const refusal = bare.getByRole("alert");
  await refusal.waitFor({ timeout: 15000 });
  await bare.getByRole("button", { name: "Details", exact: true }).click();
  const alert = await bare.getByTestId("error-details").textContent();
  ctx.check("missing WebGPU shows an actionable message", /WebGPU/.test(alert), alert);
}
