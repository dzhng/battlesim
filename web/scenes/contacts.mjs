// Slice 06: react to uncertain evidence without learning hidden truth.
import { writeFile } from "node:fs/promises";
import { decode, pixel, writeCrop } from "./_png.mjs";
import { lab, obs, snapshot, openBattle } from "./_lab.mjs";
import { checkOverlayIsolation } from "./_overlays.mjs";

export async function run(ctx) {
  const page = await openBattle(ctx);

  // The hidden squad fires at tick 60: a firing area, no identification.
  await lab(page, () => window.__lab.route.advance(62 - window.__lab.route.tick()));
  let o = await obs(page);
  const firing = o.contacts.find((c) => c.source === "firing");
  ctx.check(
    "a shot from behind the ridge shows an area, not the shooter",
    !!firing && !o.identified.some((e) => e.kind === "test_rifle"),
    JSON.stringify({ contacts: o.contacts, identified: o.identified.map((e) => e.kind) }),
  );
  if (!firing) throw new Error("contact fixture did not publish firing evidence");
  // React takes the observation on the next animation frame, so its callout
  // can lag the observation just read: wait for it (a missing one still fails).
  await page
    .locator(`[data-contact="${firing.id}"]`)
    .first()
    .waitFor({ state: "attached", timeout: 5000 })
    .catch(() => {});
  const labels = await page.locator(`[data-contact="${firing.id}"]`).allTextContents();
  ctx.check(
    "unidentified firing has a truthful info label",
    labels.length === 1 && /UNKNOWN/.test(labels[0]),
    JSON.stringify(labels),
  );
  const shot = await snapshot(ctx, page, "frame-firing-1280x800.png");
  const captions = await page.getByTestId("captions").locator("li").allTextContents();
  const at = await lab(page, (c) => window.__lab.projectToCss(c[0], c[1], 0), firing.center);
  const [r, g, b] = pixel(decode(shot), at[0], at[1]);
  ctx.check(
    "the firing area is drawn as a red glow",
    r > g + 15 && r > b + 15,
    `rgb ${r},${g},${b}`,
  );
  // The translucent firing area lies over the finished frame, unfogged.
  const isolation = await checkOverlayIsolation(ctx, page, "overlay-firing");
  ctx.check(
    "the firing area composites over the finished frame",
    isolation.isolated && isolation.coveredChannels > 0,
    JSON.stringify(isolation),
  );
  await writeCrop(
    decode(shot),
    ctx.evidencePath("crop-firing-area-2x.png"),
    at[0],
    at[1],
    160,
    100,
    2,
  );

  // Two more shots (ticks 150 and 240): the same report, same place.
  await lab(page, () => window.__lab.route.advance(245 - window.__lab.route.tick()));
  o = await obs(page);
  const again = o.contacts.find((c) => c.id === firing.id);
  ctx.check(
    "repeated shots refresh one report without a new position",
    !!again && again.center.join() === firing.center.join() && again.evidenceTick >= 240,
    JSON.stringify(again),
  );

  // The tank drives into view and back behind the hill: a fixed last-seen area.
  let seenAt = null;
  let lastSeen = null;
  for (let i = 0; i < 200 && !lastSeen; i++) {
    await lab(page, () => window.__lab.route.advance(6));
    o = await obs(page);
    const tank = o.identified.find((e) => e.kind === "test_tank");
    if (tank) seenAt = tank.position;
    const ls = o.contacts.find((c) => c.source === "last_seen");
    if (ls && seenAt) lastSeen = ls;
  }
  ctx.check(
    "losing sight of the tank leaves an area centred where it was last seen",
    // The scene samples every 6 ticks; Rust pins exact centring on the last sighting.
    !!lastSeen && Math.hypot(lastSeen.center[0] - seenAt[0], lastSeen.center[1] - seenAt[1]) < 2,
    JSON.stringify({ seenAt, lastSeen }),
  );
  await lab(page, () => window.__lab.route.advance(60));
  o = await obs(page);
  const still = o.contacts.find((c) => c.id === lastSeen?.id);
  ctx.check(
    "the last-seen area does not follow the hidden tank",
    !still || still.center.join() === lastSeen.center.join(),
    JSON.stringify(still),
  );
  await snapshot(ctx, page, "frame-last-seen-1280x800.png");

  // Sound: cues become captions (and sounds from their direction only,
  // pinned by `soundFrame.test.ts`); the camera changes nothing heard.
  ctx.check(
    "unseen enemies are heard as captioned cues",
    captions.some((text) => /Heard (gunfire|engine)/.test(text)),
    captions.join(" | "),
  );
  await writeFile(ctx.evidencePath("audio-captions.txt"), captions.join("\n"));
  const before = JSON.stringify((await obs(page)).contacts);
  await lab(page, () =>
    window.__lab.setCamera({ ...window.__lab.camera(), target: [300, 100, 0], yaw: 0.7 }),
  );
  await page.evaluate(() => window.__lab.frame());
  ctx.check(
    "moving the camera changes no evidence",
    JSON.stringify((await obs(page)).contacts) === before,
  );
}
