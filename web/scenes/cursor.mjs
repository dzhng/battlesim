// Native-size presentation contract: the lab's pictures of each action's
// cursor, and the image the system draws over the real moving pointer.
import { pointerAway } from "./_baseline.mjs";
import { decode, writeCrop } from "./_png.mjs";

const geometry = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("[data-specimen]")].map((sample) => {
      const picture = sample.querySelector(".game-cursor-picture");
      const box = sample.getBoundingClientRect();
      const image = picture.getBoundingClientRect();
      // The image's hotspot, as the system places it.
      const tip = [image.left + 2, image.top + 2];
      return {
        id: sample.dataset.specimen,
        action: sample.dataset.action,
        shows: picture.dataset.action,
        loaded: picture.complete && picture.naturalWidth > 0,
        tip,
        tipError: Math.hypot(tip[0] - (box.left + box.width / 2 - 16), tip[1] - (box.top + 14)),
        size: [image.width, image.height],
      };
    }),
  );

/** What the system is asked to draw at `at`: the document's action, and the
 *  cursor the element there resolves to. */
const pointerAt = (page, at) =>
  page.evaluate(
    ({ x, y }) => ({
      action: document.documentElement.dataset.cursor,
      cursor: getComputedStyle(document.elementFromPoint(x, y)).cursor,
    }),
    at,
  );

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  const desktop = await geometry(page);
  ctx.check(
    "every action's picture loads, at native size, its tip at the same pointer anchor",
    desktop.length === 24 &&
      desktop.every(
        (s) => s.loaded && s.shows === s.action && s.tipError < 0.01 && s.size.join() === "32,32",
      ),
    JSON.stringify(desktop),
  );
  const matrix = decode(await ctx.matchBaseline(page, "matrix-native-1280x800"));
  for (const s of desktop) {
    await writeCrop(
      matrix,
      ctx.evidencePath(`${s.id}-native.png`),
      s.tip[0] + 18,
      s.tip[1] + 17,
      30,
      29,
      1,
    );
    await writeCrop(
      matrix,
      ctx.evidencePath(`${s.id}-3x.png`),
      s.tip[0] + 18,
      s.tip[1] + 17,
      30,
      29,
      3,
    );
  }
  const playfield = page.getByTestId("cursor-playfield");
  const box = await playfield.boundingBox();
  const at = { x: Math.round(box.x + 160), y: Math.round(box.y + 80) };
  await page.mouse.move(at.x, at.y);
  const plain = await pointerAt(page, at);
  ctx.check(
    "the system draws the game's default arrow over the playfield, tip on the pointer",
    plain.action === "default" &&
      /^image-set\(url\("data:image\/svg\+xml,.*"\) 1(x|dppx), url\(".*"\) 2(x|dppx)\) 2 2, default$/.test(
        plain.cursor,
      ),
    plain.cursor.slice(0, 80),
  );
  await page.getByRole("button", { name: "Garrison", exact: true }).click();
  await page.mouse.move(at.x + 80, at.y);
  const garrison = await pointerAt(page, { x: at.x + 80, y: at.y });
  ctx.check(
    "changing action and moving gives the system that action's own image",
    garrison.action === "garrison" && garrison.cursor !== plain.cursor,
  );
  await page.mouse.move(30, 25);
  const away = await pointerAt(page, { x: 30, y: 25 });
  ctx.check(
    "leaving the playfield keeps the arrow and drops its action",
    away.action === "default" && away.cursor === plain.cursor,
  );
  await page.setViewportSize({ width: 430, height: 1000 });
  await page.evaluate(() => window.__lab.frame());
  const narrow = await geometry(page);
  ctx.check(
    "narrow layout preserves native scale and has no horizontal overflow",
    narrow.every((s) => s.tipError < 0.01 && s.size.join() === "32,32") &&
      (await page.evaluate(() => document.querySelector(".cursor-lab").scrollWidth <= innerWidth)),
  );
  await pointerAway(page);
  await ctx.matchBaseline(page, "matrix-native-430x1000");
  await ctx.writeEvidence("geometry.json", { desktop, narrow, viewport: [1280, 800], dpr: 1 });
}
