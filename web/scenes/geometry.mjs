// Slice 02: the rendered world is the authoritative geometry, probed through it.
import { writeFile } from "node:fs/promises";
import { decode, writeCrop } from "./_png.mjs";
import { snapshot } from "./_lab.mjs";

const probeAt = (page, x, y, z) =>
  page.evaluate(
    ([x, y, z]) => {
      const lab = window.__lab;
      const px = lab.projectToCss(x, y, z);
      return lab.route.probeRay(lab.rayAt(px[0], px[1]));
    },
    [x, y, z],
  );

export async function run(ctx) {
  const page = await ctx.newPage();
  await ctx.openLab(page);

  const identity = await page.evaluate(() => {
    const { exports, heightAt } = window.__lab.route;
    const p = exports.positions;
    let worst = 0;
    for (let i = 0; i < p.length; i += 3 * 17)
      worst = Math.max(worst, Math.abs(heightAt(p[i], p[i + 1]) - p[i + 2]));
    return { worst, drawn: window.__lab.stats().worldVertices, triangles: exports.indices.length };
  });
  ctx.check(
    "rendered terrain vertices are the queried heights",
    identity.worst < 1e-5 && identity.drawn >= identity.triangles,
    JSON.stringify(identity),
  );

  // Probes go through the camera ray and the exported query surface.
  const ridge = await page.evaluate(() => window.__lab.route.heightAt(100, 208));
  const onRidge = await probeAt(page, 100, 208, ridge);
  ctx.check(
    "probing the ridge lands on the query surface",
    onRidge?.collider === "terrain" &&
      Math.abs(
        onRidge.point[2] -
          (await page.evaluate(([x, y]) => window.__lab.route.heightAt(x, y), onRidge.point)),
      ) < 1e-4,
    JSON.stringify(onRidge),
  );
  const deck = await probeAt(page, 192, 160, 0.1);
  ctx.check(
    "the bridge deck is a solid walkable surface",
    deck?.collider.startsWith("bridge_deck") && deck.surface === "bridge" && deck.traversable,
    JSON.stringify(deck),
  );
  const water = await probeAt(page, 192, 125, -0.5);
  ctx.check(
    "water beside the bridge blocks ground units",
    water?.surface === "water" && !water.traversable,
    JSON.stringify(water),
  );
  const steep = await probeAt(page, 315, 50, 5);
  const gentle = await probeAt(page, 215, 50, 4.5);
  ctx.check(
    "the steep ramp is blocked and the gentle one is not",
    steep && !steep.traversable && gentle?.traversable,
    JSON.stringify({ steep, gentle }),
  );

  // Clicking shows the probe in the panel and drops the marker.
  const px = await page.evaluate(([x, y, z]) => window.__lab.projectToCss(x, y, z), [215, 50, 4.5]);
  await page.mouse.click(px[0], px[1]);
  await page.evaluate(() => window.__lab.frame());
  const panel = await page.getByTestId("probe").textContent();
  ctx.check(
    "click probe reports in the panel",
    /Hit terrain/.test(panel) && /traversable/.test(panel),
    panel,
  );

  const frame = await page.screenshot();
  await writeFile(ctx.evidencePath("frame-1280x800.png"), frame);
  const bridgePx = await page.evaluate(() => window.__lab.projectToCss(192, 160, 0.1));
  await writeCrop(
    decode(frame),
    ctx.evidencePath("crop-bridge-3x.png"),
    bridgePx[0],
    bridgePx[1],
    80,
    50,
    3,
  );

  // Traversal overlay changes the frame where ground is blocked.
  await page.evaluate(() => window.__lab.route.setOverlay("traversal"));
  const traversal = await snapshot(ctx, page, "frame-traversal.png");
  const steepPx = await page.evaluate(() => window.__lab.projectToCss(315, 50, 5));
  const [r, g, b] = (() => {
    const png = decode(traversal);
    const i = (Math.round(steepPx[1]) * png.width + Math.round(steepPx[0])) * 4;
    return [png.data[i], png.data[i + 1], png.data[i + 2]];
  })();
  ctx.check(
    "traversal overlay marks the steep ramp",
    r > g + 40 && r > b + 40,
    `rgb ${r},${g},${b}`,
  );
  await page.evaluate(() => window.__lab.route.setOverlay("surface"));

  // Ridge silhouette from a low camera across it.
  await page.evaluate(() =>
    window.__lab.setCamera({
      ...window.__lab.camera(),
      target: [100, 210, 8],
      distance: 190,
      pitch: 0.08,
      yaw: -Math.PI / 2,
    }),
  );
  const low = await snapshot(ctx, page, "frame-ridge-low.png");
  const peak = await page.evaluate(() =>
    window.__lab.projectToCss(100, 208, window.__lab.route.heightAt(100, 208)),
  );
  await writeCrop(decode(low), ctx.evidencePath("crop-ridge-2x.png"), peak[0], peak[1], 200, 90, 2);
  await ctx.writeEvidence("meta.json", {
    browser: ctx.browser,
    adapter: await page.evaluate(() => window.__lab.adapter),
    viewport: [1280, 800],
    dpr: 1,
  });
}
