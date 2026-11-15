export async function purchase(page, ticket, destination) {
  const card = await page.evaluate(async ({ side, setup, player }) => {
    const { catalogSet } = await import("/src/battle/catalog/sets.ts");
    const catalog = await catalogSet("game");
    const economy = window.__lab.route.observation().skirmish.players.find((p) => p.id === player);
    // Choose affordable mobile ground scouts from the live catalog, without
    // fixing a unit id, a price, or a tuning value in the test.
    const cards = catalog.units.cards.filter(
      (card) =>
        card.disabled_reason === null &&
        card.roster.category === "rec" &&
        card.roster.factions.includes(setup.factions[side]) &&
        card.cost <= economy.credits &&
        catalog.units.has(card.id) &&
        "wheeled" in catalog.units.type(card.id).mobility,
    );
    cards.sort(
      (a, b) =>
        catalog.units.type(b.id).sensors.ground_m - catalog.units.type(a.id).sensors.ground_m,
    );
    return cards[0];
  }, ticket);
  if (!card) throw new Error("the faction has no affordable wheeled scout");
  const start = destination ?? (await page.evaluate(() => window.__lab.route.prepared().start.at));
  await page.evaluate((at) => {
    const camera = window.__lab.camera();
    window.__lab.setCamera({ ...camera, target: [...at, 0], distance: 160 });
  }, start);
  await page.getByRole("button", { name: "Reinforcements", exact: true }).click();
  await page.getByRole("textbox", { name: "Search units" }).fill(card.name);
  // A family click chooses its first affordable variant; the test observes
  // what the real picker chooses, rather than imposing a loadout.
  await page.getByRole("tabpanel").getByRole("button").first().click();
  await page.mouse.move(640, 360);
  await page.waitForFunction(() => window.__lab.route.purchaseGhost()?.valid === true, null, {
    timeout: 30_000,
  });
  await page.mouse.click(640, 360);
  await page.waitForFunction(() =>
    window.__lab.route
      .acks()
      .some(
        ({ order, ack }) =>
          order.kind === "confirm_purchase" && !ack.error && ack.refused.length === 0,
      ),
  );
}
