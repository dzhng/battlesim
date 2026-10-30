import { lab, obs, advance } from "./_lab.mjs";
/** A scene journey is ready only after every destination was admitted and
 *  the authority published through the acknowledged application tick. Its
 *  caller still observes the travel or sighting the picture requires. An
 *  order given as `{ order, queued: true }` joins its units' queues. */
export async function stageStreetMoves(ctx, page, orders, name) {
  const before = await lab(page, () => window.__lab.route.acks()[0]?.seq ?? 0);
  await lab(
    page,
    (orders) => {
      for (const o of orders) window.__lab.route.command(o.order ?? o, o.queued ?? false);
    },
    orders,
  );
  await page.waitForFunction(
    ({ before, count }) => window.__lab.route.acks().filter((a) => a.seq > before).length === count,
    { before, count: orders.length },
  );
  const staging = await lab(
    page,
    (seq) => window.__lab.route.acks().filter((a) => a.seq > seq),
    before,
  );
  const admitted = staging.every(
    ({ ack, order }) =>
      ack.error === null &&
      ack.placement?.destinations.length === order.units.length &&
      ack.placement.destinations.every((d) => d.placed && order.units.includes(d.unit)),
  );
  ctx.check(`${name}: every destination is admitted`, admitted, JSON.stringify(staging));
  if (!admitted) throw new Error(`${name}: refused staging command`);
  const applied = Math.max(...staging.map(({ ack }) => ack.applied_tick));
  await advance(page, Math.max(0, applied - (await obs(page)).tick));
  ctx.check(`${name}: the acknowledged tick is published`, (await obs(page)).tick >= applied);
}
