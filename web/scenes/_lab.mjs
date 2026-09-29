// Helpers every scene shares: the lab's route probes, stepping the paused
// authority until an observation holds, and saving a drawn frame.
import { writeFile } from "node:fs/promises";

/** Run `fn(arg)` in the page. */
export const lab = (page, fn, arg) => page.evaluate(fn, arg);

/** The newest decoded observation. */
export const obs = (page) => lab(page, () => window.__lab.route.observation());

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

/** Wait until the paused authority's last tick is presented: a drawn frame
 *  has fed the pose driver that tick's observation (React state, which lags
 *  the publications) at a presentation clock on that tick. The clock never
 *  passes the latest publication, so a paused battle's presentation comes
 *  to rest there; anything timed on it (a death playing out) has then had
 *  exactly the ticks advanced, whatever the machine's load. */
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

/** The village's scenario line (variant and seed), read in its pause menu. */
export async function scenarioLine(page) {
  await openMenu(page);
  const line = await page.getByTestId("status").innerText();
  await closeMenu(page);
  return line;
}

/** Set the village's seed in its pause menu: the battle remounts on it (the
 *  menu goes with the old view); resolves once the new battle runs. */
export async function chooseSeed(page, seed) {
  await openMenu(page);
  await page.getByLabel("Seed", { exact: true }).fill(String(seed));
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
