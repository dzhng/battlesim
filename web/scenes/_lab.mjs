// Helpers every scene shares: the lab's route probes, stepping the paused
// authority until an observation holds, and saving a drawn frame.
import { writeFile } from "node:fs/promises";

/** Run `fn(arg)` in the page. */
export const lab = (page, fn, arg) => page.evaluate(fn, arg);

/** The newest decoded observation. */
export const obs = (page) => lab(page, () => window.__lab.route.observation());

/** Call `method` of the lab route's diagnostics with `args`. */
export const route = (page, method, ...args) =>
  lab(page, ([method, args]) => window.__lab.route[method](...args), [method, args]);

/** The WebGPU validation warnings the page logs from now on: a list that
 *  fills as they arrive. Any one is a failed render. */
export function gpuWarnings(page) {
  const warnings = [];
  page.on("console", (m) => {
    if (m.type() === "warning" && /webgpu|validation|gpu\w*error/i.test(m.text()))
      warnings.push(m.text().slice(0, 200));
  });
  return warnings;
}

/** Advance the paused authority `n` ticks. React's development build records
 *  a performance measure per component render; thousands of fast-forwarded
 *  ticks would exhaust that buffer, so the ticks go in chunks with the buffer
 *  cleared before each. */
export async function advance(page, n) {
  const CHUNK = 60;
  for (let done = 0; done < n; done += CHUNK) {
    await lab(
      page,
      (k) => (performance.clearMeasures(), window.__lab.route.advance(k)),
      Math.min(CHUNK, n - done),
    );
  }
}

/** Wait until the paused authority's latest observation is drawn at its settled
 *  clock. Death-animation starts still depend on earlier presented frames. */
export async function presented(page, timeout = 30000) {
  await page.waitForFunction(
    () => {
      const tick = window.__lab.route.tick();
      const p = window.__lab.route.presented();
      return !!p && p.tick === tick && p.clock >= tick - 1e-6;
    },
    undefined,
    { timeout, polling: 50 },
  );
}

/** Advance in `step`-tick steps until `test(observation)` holds, within
 *  `limit` ticks; returns that observation or null. `each` sees every one. */
export async function until(page, test, limit, step = 15, each = () => {}) {
  for (let t = 0; t < limit; t += step) {
    await advance(page, step);
    const o = await obs(page);
    each(o);
    if (test(o)) return o;
  }
  return null;
}

/** What the building layer drew in the last frame (`BuildingStats`). */
export const buildingStats = (page) => lab(page, () => window.__lab.stats().buildings);

/** The page as the frame's ground-classes view shows it, a PNG: black where a
 *  pixel is not wholly bare ground (something stands over it), the ground's
 *  class bytes where it is. The frame is back at its final view after. */
export async function groundClasses(page) {
  await lab(page, () => window.__lab.setFrameView("ground-classes"));
  const png = await page.screenshot();
  await lab(page, () => window.__lab.setFrameView("final"));
  return png;
}

/** The frame's GPU time over `frames` forced redraws from now (a lab draws on
 *  demand, and setting the frame view starts the timer's window again). */
export function gpuMs(page, frames = 120) {
  return lab(
    page,
    async (frames) => {
      await window.__lab.setFrameView("final");
      for (let i = 0; i < frames; i++) await window.__lab.frame();
      return window.__lab.stats().gpu;
    },
    frames,
  );
}

/** Draw frames until every building near the camera is expanded into the
 *  pool: a view change expands a budget of rows a frame, and a chunk waiting
 *  its turn draws at the coarsest tier meanwhile. */
export async function buildingsSettled(page, timeout = 30000) {
  const deadline = Date.now() + timeout;
  for (;;) {
    await lab(page, () => window.__lab.frame());
    if (await lab(page, () => !window.__lab.stats().buildings.pending)) return;
    if (Date.now() >= deadline) throw new Error("building expansion did not settle");
  }
}

/** Open a battle's pause menu by its HUD button. */
export async function openMenu(page) {
  await page.getByRole("button", { name: "Menu", exact: true }).click();
  await page.getByRole("dialog", { name: "Paused" }).waitFor();
}

/** Close the pause menu (Resume). */
export async function closeMenu(page) {
  await page.getByRole("button", { name: "Resume", exact: true }).click();
  await page.getByRole("dialog", { name: "Paused" }).waitFor({ state: "detached" });
}

/** Restart a battle from its pause menu. */
export async function restart(page) {
  await openMenu(page);
  await page.getByRole("button", { name: "Restart", exact: true }).click();
}

/** The village's pause menu: the variant chosen, and the menu's whole text
 *  (which names no seed). */
export async function chosenVariant(page) {
  await openMenu(page);
  const menu = page.getByRole("dialog", { name: "Paused" });
  const variant = await menu.getByRole("radio", { checked: true }).textContent();
  const text = await menu.textContent();
  await closeMenu(page);
  return { variant, text };
}

/** Choose the village's variant in its pause menu: the battle remounts on
 *  it (the menu goes with the old view); resolves once the new battle runs. */
export async function chooseVariant(page, name) {
  await openMenu(page);
  await page.getByRole("radio", { name }).click();
  await page.getByRole("dialog", { name: "Paused" }).waitFor({ state: "detached" });
  await page.waitForFunction(
    () => window.__lab?.ready && window.__lab.route?.tick() > 3,
    undefined,
    {
      timeout: 60000,
    },
  );
}

/** Draw a fresh frame, save the page screenshot as evidence `file`, and
 *  return the PNG. */
export async function snapshot(ctx, page, file) {
  await page.evaluate(() => window.__lab.frame());
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath(file), shot);
  return shot;
}

/** A new page on the lab at `url` (the fixture's route by default), its
 *  battle running (past tick 3), then paused and, with `tick`, stepped to
 *  that tick. `grass` also waits for the grass kinds to install first (the
 *  catalog loads after the battle starts). */
export async function openBattle(
  ctx,
  { viewport, url, tick, timeout = 30000, grass = false, allowErrors } = {},
) {
  const page = await ctx.newPage({ viewport, allowErrors });
  await ctx.openLab(page, url, timeout);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout });
  if (grass)
    await page.waitForFunction(() => window.__lab.stats?.().grass.enabled, undefined, {
      timeout: 30000,
    });
  await lab(page, () => window.__lab.route.pause());
  if (tick !== undefined)
    await advance(page, tick - (await lab(page, () => window.__lab.route.tick())));
  return page;
}

/** The camera on `at`, keeping what `view` (distance, pitch, yaw) leaves
 *  out: its target on the walkable surface under `at`, or at `at[2]` with
 *  `onGround` false. */
export const aim = (page, at, view = {}, { onGround = true } = {}) =>
  lab(
    page,
    ({ at, view, onGround }) =>
      window.__lab.setCamera({
        ...window.__lab.camera(),
        ...view,
        target: [at[0], at[1], onGround ? window.__lab.route.surfaceZ(at[0], at[1]) : at[2]],
      }),
    {
      at,
      view: Object.fromEntries(Object.entries(view).filter(([, v]) => v !== undefined)),
      onGround,
    },
  );

/** The page point of the walkable surface under world point `p`. */
export const groundCss = (page, p) =>
  lab(
    page,
    (q) => window.__lab.projectToCss(q[0], q[1], window.__lab.route.surfaceZ(q[0], q[1])),
    p,
  );

/** The pointer onto the HUD's menu button: off the canvas (no range ruler,
 *  no edge pan), whatever the selection. */
export async function pointerOffCanvas(page) {
  const box = await page.getByRole("button", { name: "Menu", exact: true }).boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
}
