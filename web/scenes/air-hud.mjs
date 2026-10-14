// Aircraft in the HUD (slice 10). The F-16 (HighAir) has no ground marker
// and no drop line: in the overlay pass the Apache (LowAir) beside it has
// its line, the jet none, and selecting the jet changes nothing on the
// ground under it while selecting the Apache recolours its marker. The
// jet's card shows its fuel and flares; ordered away with Space held, its
// ghost carries a compact card. The EA-18G's radar contact reads RADAR
// CONTACT with its bearing and range.
import { decode, mostChanged, pixel, around } from "./_png.mjs";
import { gpuWarnings, lab, obs, hideHud, presented, aim } from "./_lab.mjs";
import { openAir, own, play, surface, command, cardText } from "./_air.mjs";
import { unitType } from "./_units.mjs";

const JET = "f_16c_block_50";
const HELI = "ah_64e_guardian";
const sum = (c) => c[0] + c[1] + c[2];

/** The brightest pixel within 3 px of the screen segment `a`–`b`. */
function brightestAlong(png, a, b) {
  let best = 0;
  const steps = Math.max(1, Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1])));
  for (let i = 0; i <= steps; i++) {
    const p = [a[0] + ((b[0] - a[0]) * i) / steps, a[1] + ((b[1] - a[1]) * i) / steps];
    for (const [x, y] of around(png, p, 3)) best = Math.max(best, sum(pixel(png, x, y)));
  }
  return best;
}

const css = (page, p) => lab(page, (q) => window.__lab.projectToCss(q[0], q[1], q[2]), p);

export async function run(ctx) {
  const page = await openAir(ctx);
  const warnings = gpuWarnings(page);
  const o = await play(page, 30);
  const jet = own(o, JET)[0];
  const heli = own(o, HELI)[0];
  await lab(page, () =>
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [200, 265, 0],
      distance: 260,
      pitch: 0.5,
    }),
  );
  const style = await hideHud(page);

  // The overlay pass: drop lines from the ground up toward each airframe.
  const segment = async (u) => {
    const ground = await surface(page, u.position);
    const lift = u.position[2] - ground;
    return [
      await css(page, [u.position[0], u.position[1], ground + lift * 0.1]),
      await css(page, [u.position[0], u.position[1], ground + lift * 0.5]),
    ];
  };
  await presented(page);
  await page.evaluate(() => window.__lab.setFrameView("overlays-on-black"));
  await page.evaluate(() => window.__lab.frame());
  const overlays = decode(await page.screenshot());
  await page.evaluate(() => window.__lab.setFrameView("final"));
  const heliLine = brightestAlong(overlays, ...(await segment(heli)));
  const jetLine = brightestAlong(overlays, ...(await segment(jet)));
  ctx.check(
    "the Apache has a drop line, the jet none",
    heliLine > 150 && jetLine < 60,
    JSON.stringify({ heliLine, jetLine }),
  );

  // Selection recolours a ground marker: the Apache's, never the jet's.
  const under = async (u) =>
    css(page, [u.position[0], u.position[1], await surface(page, u.position)]);
  const frame = async () => (
    await page.evaluate(() => window.__lab.frame()), decode(await page.screenshot())
  );
  const plain = await frame();
  await lab(page, (ids) => window.__lab.route.select(ids), [jet.id, heli.id]);
  const selected = await frame();
  const jetChange = mostChanged(plain, selected, await under(jet), 14);
  const heliChange = mostChanged(plain, selected, await under(heli), 14);
  ctx.check(
    "selected, the Apache's ground marker changes and nothing changes under the jet",
    heliChange > 60 && jetChange < 30,
    JSON.stringify({ heliChange, jetChange }),
  );
  await style.evaluate((el) => el.remove());

  // The jet's card: fuel and flares.
  await lab(page, (id) => window.__lab.route.select([id]), jet.id);
  const card = await cardText(page, `[data-unit="${jet.id}"]`);
  const flares = unitType(JET).capabilities.air.flares.capacity;
  ctx.check(
    "the jet's card shows its fuel and flares",
    !!card && /FUEL \d+ s/.test(card) && card.includes(`FLARES ${flares}`),
    card ?? "no card",
  );

  // Ordered away, with Space held its ghost carries a compact card.
  await command(page, {
    kind: "move",
    units: [jet.id],
    gesture: 1,
    goal: [320, 330],
    route: "fastest",
  });
  await play(page, 2);
  await page.keyboard.down("Space");
  const ghost = await cardText(page, ".ro-callout[data-ghost]");
  await page.screenshot({ path: ctx.evidencePath("space-ghost.png") });
  await page.keyboard.up("Space");
  ctx.check(
    "with Space held the jet's ghost carries its card",
    ghost !== null,
    ghost ?? "no ghost card",
  );

  // The radar contact's card.
  const radar = (await obs(page)).contacts.find((c) => c.source === "radar");
  if (radar) await aim(page, radar.center, { distance: 140, pitch: 0.8 });
  const text = radar ? await cardText(page, "") : null;
  ctx.check(
    "the radar contact reads RADAR CONTACT with its bearing and range",
    !!text && text.includes("RADAR CONTACT") && /BEARING -?\d+° · RANGE [\d,]+ m/.test(text),
    text ?? "no radar contact card",
  );
  await page.screenshot({ path: ctx.evidencePath("hud.png") });
  ctx.check("no GPU validation warnings", warnings.length === 0, warnings.join("\n"));
}
