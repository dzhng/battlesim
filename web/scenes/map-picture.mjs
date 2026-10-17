// The lobby map picture, one cell at a time at lobby size: a pairing at or
// above its size floor (fixtures/skirmish-sizes.json) shows its admitted map
// with both sides' entries and every objective drawn, and one below the floor
// shows the preview's refusal. The first cell shows loading before its map.
// Shots go to the evidence folder, one per cell.
import { readFile } from "node:fs/promises";

const SIZES = ["small", "medium", "large", "xl"];
const PAIRINGS = { "1v1": [1, 1], "3v4": [3, 4], "5v5": [5, 5] };

export async function run(ctx) {
  const tiers = JSON.parse(
    await readFile(new URL("../../fixtures/skirmish-sizes.json", import.meta.url), "utf8"),
  ).tiers;
  const floor = (players) =>
    tiers.find((t) => players >= t.min_players && (t.max_players ?? Infinity) >= players).size;
  const page = await ctx.newPage({ viewport: { width: 640, height: 640 } });
  let first = true;
  for (const [pairing, sides] of Object.entries(PAIRINGS)) {
    const largest = Math.max(...sides);
    const entries = largest <= 1 ? 1 : largest === 2 ? 2 : 3;
    for (const size of SIZES) {
      const cell = `${pairing}-${size}`;
      await page.goto(ctx.urlFor({ cell }));
      if (first) {
        await page.locator(".mp-frame").waitFor();
        ctx.check(
          "the preview shows loading from its first frame",
          (await page.locator(".mp-frame").getAttribute("data-status")) === "loading",
        );
        await page.locator(".mp-frame").screenshot({ path: ctx.evidencePath("loading.png") });
        first = false;
      }
      await ctx.openLab(page, ctx.urlFor({ cell }), 90000);
      const status = await page.locator(".mp-frame").getAttribute("data-status");
      const admitted = SIZES.indexOf(size) >= SIZES.indexOf(floor(largest));
      ctx.check(
        `${cell} is ${admitted ? "drawn" : "refused below its size floor"}`,
        status === (admitted ? "ready" : "failed"),
        status,
      );
      if (status === "ready") {
        const blue = await page.locator('[data-entry="blue"]').count();
        const red = await page.locator('[data-entry="red"]').count();
        const objectives = await page.locator("[data-objective]").count();
        ctx.check(
          `${cell} draws ${entries} entries a side and its objectives`,
          blue === entries && red === entries && objectives > 0,
          `blue ${blue}, red ${red}, objectives ${objectives}`,
        );
      }
      await page.locator(".mp-frame").screenshot({ path: ctx.evidencePath(`${cell}.png`) });
    }
  }
  await page.close();
}
