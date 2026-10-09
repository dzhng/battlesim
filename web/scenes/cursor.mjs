// Native-size presentation contract, using the component's real moving-pointer surface.
import { pointerAway } from "./_baseline.mjs";
import { decode, writeCrop, mostChanged } from "./_png.mjs";

const geometry = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("[data-specimen]")].map((sample) => {
      const cursor = sample.querySelector(".game-cursor");
      const svg = cursor.querySelector(".game-cursor-arrow svg");
      const point = svg.createSVGPoint();
      point.x = 2;
      point.y = 2;
      const tip = point.matrixTransform(svg.getScreenCTM());
      const box = sample.getBoundingClientRect();
      const badge = cursor.querySelector(".game-cursor-badge");
      const b = badge.getBoundingClientRect();
      const icon = svg.getBoundingClientRect();
      return {
        id: sample.dataset.specimen,
        action: sample.dataset.action,
        tip: [tip.x, tip.y],
        tipError: Math.hypot(tip.x - (box.left + box.width / 2 - 16), tip.y - (box.top + 14)),
        arrowSize: [icon.width, icon.height],
        badge: badge.hidden ? null : [b.x - tip.x, b.y - tip.y, b.width, b.height],
        icons: [...badge.children]
          .filter((node) => node.getClientRects().length)
          .map((node) => node.dataset.action),
      };
    }),
  );

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);
  const desktop = await geometry(page);
  ctx.check(
    "every action keeps the arrow tip at the same pointer anchor",
    desktop.length === 24 &&
      desktop.every(
        (s) => s.tipError < 0.01 && s.arrowSize.every((v) => Math.abs(v - 22.4) < 0.01),
      ),
    JSON.stringify(desktop),
  );
  ctx.check(
    "default has no badge; all other actions show exactly their one icon at the approved offset",
    desktop.every((s) =>
      s.action === "default"
        ? s.badge === null && !s.icons.length
        : s.badge.every((v, i) => Math.abs(v - [15.4, 13.3, 13.3, 13.3][i]) < 0.01) &&
          s.icons.join() === s.action,
    ),
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
  const before = decode(await page.screenshot({ path: ctx.evidencePath("pointer-absent.png") }));
  await page.mouse.move(at.x, at.y);
  // The app cursor is mounted after the page, so it is the last one.
  const moving = page.getByTestId("game-cursor").last();
  ctx.check(
    "pointer motion places the real default cursor",
    (await moving.isVisible()) && (await moving.locator(".game-cursor-badge").isHidden()),
  );
  const after = decode(await page.screenshot({ path: ctx.evidencePath("pointer-default.png") }));
  ctx.check(
    "the cursor actually changes the captured frame at the pointer",
    mostChanged(before, after, [at.x + 10, at.y + 10], 24) > 100,
  );
  await page.getByRole("button", { name: "Garrison", exact: true }).click();
  await page.mouse.move(at.x + 80, at.y);
  ctx.check(
    "changing action and moving updates the one mounted cursor",
    await moving.locator('[data-action="garrison"]').isVisible(),
  );
  await ctx.matchBaseline(page, "pointer-garrison");
  await page.mouse.move(30, 25);
  ctx.check(
    "leaving the playfield keeps the arrow and drops its action",
    (await moving.isVisible()) && (await moving.locator(".game-cursor-badge").isHidden()),
  );
  await page.setViewportSize({ width: 430, height: 1000 });
  await page.evaluate(() => window.__lab.frame());
  const narrow = await geometry(page);
  ctx.check(
    "narrow layout preserves native scale and has no horizontal overflow",
    narrow.every((s) => s.tipError < 0.01 && s.arrowSize.every((v) => Math.abs(v - 22.4) < 0.01)) &&
      (await page.evaluate(() => document.querySelector(".cursor-lab").scrollWidth <= innerWidth)),
  );
  await pointerAway(page);
  await ctx.matchBaseline(page, "matrix-native-430x1000");
  await ctx.writeEvidence("geometry.json", { desktop, narrow, viewport: [1280, 800], dpr: 1 });
}
