// Slice 13: service eligibility, finite stock and replacements in the browser.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";

const lab = (page, fn, arg) => page.evaluate(fn, arg);
const obs = (page) => lab(page, () => window.__lab.route.observation());
const advance = (page, n) => lab(page, (k) => window.__lab.route.advance(k), n);
const unit = (o, id) => o.own.find((u) => u.id === id);

async function frame(ctx, page, name, crop) {
  await page.evaluate(() => window.__lab.frame());
  const shot = await page.screenshot();
  await writeFile(ctx.evidencePath(`frame-${name}-1280x800.png`), shot);
  if (crop) {
    const at = await lab(page, (p) => window.__lab.projectToCss(p[0], p[1], 0), crop);
    await writeCrop(
      decode(shot),
      ctx.evidencePath(`crop-${name}-2x.png`),
      at[0],
      at[1],
      170,
      130,
      2,
    );
  }
}

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  await page.waitForFunction(() => window.__lab.route?.tick() > 3, undefined, { timeout: 20000 });
  await lab(page, () => window.__lab.route.pause());
  let o = await obs(page);
  const start = {
    stock: unit(o, 0).stock,
    fallen: JSON.stringify(o.corpses.filter((c) => c.own)),
    at: unit(o, 2).mounts[1].ammo[0],
  };
  ctx.check(
    "nothing is served before the truck is set up",
    unit(o, 3).service === "source_not_deployed",
    JSON.stringify(unit(o, 3)?.service),
  );
  await frame(ctx, page, "setting-up", [205, 210]);

  // Set up (15 s), then serve for 20 s.
  await advance(page, 450 + 600);
  o = await obs(page);
  const truck = unit(o, 0);
  ctx.check(
    "the set-up truck pays from its stock for what it restores",
    truck.stock < start.stock && unit(o, 2).mounts[1].ammo[0] > start.at,
    JSON.stringify({ stock: truck.stock, atgm: unit(o, 2).mounts[1].ammo }),
  );
  ctx.check(
    "casualties are replaced and the fallen stay",
    unit(o, 3).members.length === 8 &&
      JSON.stringify(o.corpses.filter((c) => c.own)).startsWith(start.fallen.slice(0, -1)),
    JSON.stringify({ members: unit(o, 3).members.length, corpses: o.corpses.length }),
  );
  ctx.check(
    "the scouts beside the empty truck wait for stock",
    unit(o, 4).stock === 0 && unit(o, 5).service === "no_stock",
    JSON.stringify({ stock: unit(o, 4).stock, scouts: unit(o, 5).service }),
  );
  await frame(ctx, page, "serving", [205, 210]);

  // A unit that moves off waits; the stock never regrows.
  await lab(page, () => window.__lab.route.demo("Tank: move off"));
  // A vehicle turns on the spot first (still served), then drives.
  await advance(page, 75);
  await frame(ctx, page, "waiting", [230, 170]);
  o = await obs(page);
  ctx.check(
    "a moving recipient waits",
    ["moving", "out_of_range"].includes(unit(o, 1).service),
    unit(o, 1).service,
  );
  const before = unit(o, 0).stock;
  await lab(page, () => window.__lab.route.demo("Relocate the truck"));
  await advance(page, 120);
  o = await obs(page);
  ctx.check(
    "a packing, moving truck serves nobody and its stock stays put",
    unit(o, 0).stock === before && o.own.every((u) => u.stock !== null || u.service !== "serving"),
    JSON.stringify({ stock: unit(o, 0).stock, before, services: o.own.map((u) => u.service) }),
  );
  await frame(ctx, page, "relocating", [250, 230]);
}
