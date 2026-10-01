// A battle on a generated map: the map the URL asks for is the one the
// preparation worker made, a loading screen covers the wait, and the battle
// on it starts and runs.
import { writeFile } from "node:fs/promises";
import { lab, snapshot } from "./_lab.mjs";

/** Wait until the page's battle is playable: the loading screen has lifted
 *  over a running battle. */
export async function playable(page, timeout = 120000) {
  await page.waitForFunction(
    () =>
      document.querySelector("[data-testid=error]") ||
      window.__lab?.error ||
      (window.__lab?.ready &&
        window.__lab.route?.tick?.() > 3 &&
        !document.querySelector("[data-testid=loading]")),
    undefined,
    { timeout },
  );
  const error = await page.evaluate(
    () => document.querySelector("[data-testid=error]")?.textContent ?? window.__lab?.error,
  );
  if (error) throw new Error(`lab failed: ${error}`);
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1920, height: 1080 } });

  // The loading screen names the map and its stage while the worker prepares it.
  await page.goto(`${ctx.url}?type=open&size=small&seed=1`);
  await page.getByTestId("loading").waitFor();
  const loading = await page.evaluate(() => ({
    subject: document.querySelector("[data-testid=loading-subject]")?.textContent,
    stage: document.querySelector("[data-testid=loading-stage]")?.textContent,
  }));
  await writeFile(ctx.evidencePath("loading-1920x1080.png"), await page.screenshot());
  ctx.check(
    "a loading screen names the map and the stage while it is prepared",
    loading.subject === "OPEN · SMALL · SEED 1" && !!loading.stage,
    JSON.stringify(loading),
  );

  await playable(page);
  const generated = await lab(page, () => window.__lab.route.generated());
  ctx.check(
    "the battle runs on the map the URL asked for",
    generated.map.type === "open" &&
      generated.map.size === "small" &&
      generated.map.seed === "1" &&
      generated.identity.seed === "1" &&
      generated.size.join() === "6000,6000",
    JSON.stringify({ map: generated.map, identity: generated.identity, size: generated.size }),
  );
  await snapshot(ctx, page, "opening-1920x1080.png");

  // A request the generator cannot serve says so, and starts no battle.
  const refused = await ctx.newPage({ allowErrors: true });
  await refused.goto(`${ctx.url}?type=metro&size=tiny`);
  await refused.getByTestId("error").waitFor();
  const message = await refused.getByTestId("error").textContent();
  ctx.check(
    "a request for no such map is refused by name, with no battle",
    /size must be one of small, medium, large/.test(message) &&
      !(await refused.evaluate(() => window.__lab?.ready ?? false)),
    message,
  );
}
