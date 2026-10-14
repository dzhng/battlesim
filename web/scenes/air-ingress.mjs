// Ingress, fuel, afterburner and withdrawal (slice 06), in an empty
// skirmish on the air field. Blue buys two F-16s, one for the middle of the
// field and one near its north edge. Each is paid at once and appears as
// its own unit, traversing: off the map edge nearest its destination (the
// south edge for the first, the north for the second), its army card
// greyed, its ghost's card shown with Space held, and its fuel untouched.
// After a seeded 15–30 s it enters at that edge point at the air height,
// without a jump, and burns fuel from then on; afterburners double the
// burn. Run dry, it turns for blue's entry on its own and is refunded there:
// some but not all of its price.
//
// Its price is the card's. Not shown: a destroyed aircraft's lack of refund (the skirmish has no
// enemy fire), and circling at the destination (aircraft hover there).
import { writeFile } from "node:fs/promises";
import { gpuWarnings, obs, advance } from "./_lab.mjs";
import { openAir, play, FIELD, command, surface, CRUISE, shoot } from "./_air.mjs";
import { unitType } from "./_units.mjs";
import { nodeCatalogSet } from "../src/battle/catalog/node.ts";

const { cards } = (await nodeCatalogSet("test")).units.view;

const JET = "f_16c_block_50";
const PRICE = cards.find((c) => c.id === JET).cost;
/** The checkpoint skirmish's income (`airWar.tsx`). */
const INCOME_PER_MINUTE = 200;
const FUEL = unitType(JET).capabilities.air.fuel_capacity_s;
const AB = unitType(JET).capabilities.air.afterburner.fuel_multiplier;
const ORDERS = [
  { destination: [300, 200], edge: (p) => p[1] },
  { destination: [300, 480], edge: (p) => FIELD[1] - p[1] },
];

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  let o = await obs(page);
  ctx.check("the checkpoint is a skirmish", !!o.skirmish, JSON.stringify(o.skirmish));
  await command(page, { kind: "ready" });
  await advance(page, 2);
  const before = (await obs(page)).skirmish.credits;
  for (const { destination } of ORDERS)
    await command(page, { kind: "confirm_purchase", variant: JET, destination });
  await advance(page, 1);
  o = await obs(page);
  const cost = before - o.skirmish.credits;
  ctx.check(
    "both purchases are paid at once",
    Math.abs(cost - 2 * PRICE) < 1,
    JSON.stringify({ cost, price: PRICE }),
  );
  // The first is dispatched at once, the next a dispatch interval later.
  const bought = o.tick;
  await advance(page, 45);
  o = await obs(page);
  const jets = o.own.filter((u) => u.kind === JET).sort((a, b) => a.id - b.id);
  ctx.check(
    "each appears as its own unit, traversing",
    jets.length === 2 && jets.every((u) => u.traversing),
    JSON.stringify(jets.map((u) => [u.id, u.position, u.traversing])),
  );
  if (jets.length !== 2) return;
  ORDERS.forEach(({ edge }, i) =>
    ctx.check(
      `aircraft ${i + 1} waits on the map edge nearest its destination`,
      edge(jets[i].position) < 1,
      JSON.stringify(jets[i].position),
    ),
  );
  ctx.check(
    "a traversing aircraft's fuel is untouched",
    jets.every((u) => u.fuelS === FUEL),
    JSON.stringify(jets.map((u) => u.fuelS)),
  );
  const greyed = await page.locator(".hud-army-traversing").count();
  ctx.check("its army card is greyed as traversing", greyed === 2, `${greyed}`);
  await page.keyboard.down("Space");
  await page.evaluate(() => window.__lab.frame());
  const ghosts = await page.locator("[data-testid=readouts] .ro-callout[data-ghost]").count();
  await shoot(ctx, page, "traversing-ghosts");
  await page.keyboard.up("Space");
  ctx.check(
    "with Space held each traversing aircraft's ghost shows its card",
    ghosts === 2,
    `${ghosts}`,
  );

  // Entry, after the seeded delay, at the edge it waited on.
  const [first] = jets;
  let entered = null;
  o = await play(page, 31 * 30, (o) => {
    const u = o.own.find((u) => u.id === first.id);
    if (u && !u.traversing) entered = { tick: o.tick, position: u.position, fuel: u.fuelS };
    return !!entered;
  });
  const delayS = entered ? (entered.tick - bought) / 30 : null;
  ctx.check(
    "it enters after a 15–30 s traversal",
    delayS !== null && delayS >= 15 && delayS <= 30.1,
    `${delayS}`,
  );
  if (!entered) return;
  const lift = entered.position[2] - (await surface(page, entered.position));
  ctx.check(
    "it enters where it waited, at the air height, fuel full",
    Math.hypot(entered.position[0] - first.position[0], entered.position[1] - first.position[1]) <
      5 &&
      Math.abs(lift - CRUISE) < 1 &&
      entered.fuel >= FUEL - 0.1,
    JSON.stringify({ entered, waited: first.position, lift }),
  );

  // Burn: plain, then with afterburners, over the same 90 ticks.
  const fuel = async () => (await obs(page)).own.find((u) => u.id === first.id).fuelS;
  let f0 = await fuel();
  await advance(page, 90);
  const plain = f0 - (await fuel());
  await command(page, { kind: "set_afterburner", units: [first.id], enabled: true });
  await advance(page, 2);
  ctx.check(
    "the afterburner toggle is published",
    (await obs(page)).own.find((u) => u.id === first.id).afterburner,
  );
  f0 = await fuel();
  await advance(page, 90);
  const burner = f0 - (await fuel());
  ctx.check(
    `afterburners burn fuel ${AB}× as fast`,
    plain > 2.9 && Math.abs(burner / plain - AB) < 0.05,
    JSON.stringify({ plain, burner }),
  );

  // Run dry: it turns for home by itself and is refunded at the entry. The
  // wallet also earns the skirmish's income, steadily, each step.
  const step = 30;
  const income = (INCOME_PER_MINUTE / 60 / 30) * step;
  let home = null;
  let retired = null;
  let wallet = (await obs(page)).skirmish.credits;
  for (let t = 0; t < 3600 && !retired; t += step) {
    await advance(page, step);
    const now = await obs(page);
    const u = now.own.find((u) => u.id === first.id);
    if (u?.withdrawing && !home) home = { tick: now.tick, fuel: u.fuelS, position: u.position };
    if (!u) retired = { tick: now.tick, refund: now.skirmish.credits - wallet - income };
    wallet = now.skirmish.credits;
  }
  ctx.check(
    "near empty, it turns for home on its own",
    !!home && home.fuel > 0 && home.fuel < 15,
    JSON.stringify(home),
  );
  ctx.check(
    "it leaves the field at the entry and is refunded part of its price",
    !!retired && retired.refund > 1 && retired.refund < PRICE,
    JSON.stringify({ retired, cost }),
  );
  await writeFile(
    ctx.evidencePath("ingress.json"),
    JSON.stringify({ cost, delayS, plain, burner, home, retired }, null, 2),
  );
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
