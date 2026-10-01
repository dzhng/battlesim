// Slice 13: service eligibility, finite stock and replacements in the browser.
import { decode, writeCrop } from "./_png.mjs";
import { lab, obs, advance, snapshot, openBattle } from "./_lab.mjs";

const unit = (o, id) => o.own.find((u) => u.id === id);

/** The supply rows a recipient's panel can carry. */
const SUPPLY_ROWS = ["resupplying"];

/** Each own panel's unit, its supply row's word (null without one) and every
 *  state row's word, as drawn. */
const supplyRows = (page) =>
  page.evaluate(
    (kinds) =>
      [...document.querySelectorAll("[data-testid=readouts] .ro-unit[data-owner=own]")].map((n) => {
        const states = [...n.querySelectorAll(".ro-state")];
        const supply = states.find((e) => kinds.includes(e.dataset.state));
        return {
          unit: Number(n.dataset.unit),
          row: supply?.querySelector(".ro-state-word").textContent ?? null,
          words: states.map((e) => e.querySelector(".ro-state-word").textContent),
        };
      }),
    SUPPLY_ROWS,
  );
const rowOf = (rows, id) => rows.find((r) => r.unit === id);

async function select(page, ids) {
  await lab(page, (x) => window.__lab.route.select(x), ids);
  await page.waitForFunction((n) => window.__lab.route.selected().length === n, ids.length);
  await page.evaluate(() => window.__lab.frame());
}

async function frame(ctx, page, name, crop) {
  const shot = await snapshot(ctx, page, `frame-${name}-1280x800.png`);
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
  const page = await openBattle(ctx);
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
  await page.evaluate(() => window.__lab.frame());
  let rows = await supplyRows(page);
  ctx.check(
    "before the truck is set up, nobody shows a supply row and the truck shows DEPLOYING",
    rows.every((r) => r.row === null) && rowOf(rows, 0)?.words.includes("DEPLOYING"),
    JSON.stringify(rows),
  );
  await frame(ctx, page, "setting-up", [205, 210]);

  // Set up, then serve long enough to restore the recipients.
  await advance(page, 90 + 600);
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
  // Recipients show active service only; trucks show stock and supplying.
  await page.evaluate(() => window.__lab.frame());
  rows = await supplyRows(page);
  const WORD = {
    serving: "RESUPPLYING",
  };
  const serving = o.own.filter((u) => u.service === "serving").map((u) => u.id);
  ctx.check(
    "only actively resupplying recipients show a supply row",
    serving.length > 0 &&
      o.own.every((u) => (rowOf(rows, u.id)?.row ?? null) === (WORD[u.service] ?? null)),
    JSON.stringify({ services: o.own.map((u) => [u.id, u.service]), rows }),
  );
  ctx.check(
    "the scouts beside the empty truck have no redundant supply row",
    rowOf(rows, 5)?.row === null,
    JSON.stringify(rowOf(rows, 5)),
  );
  ctx.check(
    "each truck's panel shows its stock, and the serving one SUPPLYING",
    rowOf(rows, 0)?.words.includes(`SUPPLY ${truck.stock}`) &&
      rowOf(rows, 0)?.words.includes("SUPPLYING") &&
      rowOf(rows, 4)?.words.includes("SUPPLY 0") &&
      !rowOf(rows, 4)?.words.includes("SUPPLYING"),
    JSON.stringify([rowOf(rows, 0), rowOf(rows, 4)]),
  );
  await frame(ctx, page, "empty-truck", [345, 325]);
  // The truck's reach shows only while it is selected.
  await frame(ctx, page, "serving-unselected", [205, 210]);
  await select(page, [0]);
  await frame(ctx, page, "serving", [205, 210]);

  // A unit that moves off waits; the stock never regrows.
  await lab(page, () => window.__lab.route.demo("Tank: move off"));
  // A vehicle turns on the spot first (still served), then drives.
  await advance(page, 75);
  await select(page, [0, 1]);
  await frame(ctx, page, "waiting", [230, 170]);
  o = await obs(page);
  rows = await supplyRows(page);
  ctx.check(
    "a moving recipient has no redundant supply row",
    unit(o, 1).service === "moving"
      ? rowOf(rows, 1)?.row === null
      : unit(o, 1).service === "out_of_range" && rowOf(rows, 1)?.row === null,
    JSON.stringify({ service: unit(o, 1).service, rows }),
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
