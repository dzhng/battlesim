// Exported/query geometry agreement, camera picking, and traversal overlay rendering.
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
    "exported terrain heights match queries, and the renderer reports at least that vertex count",
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
  // The same viewer inspects another catalogue map; the default fixture
  // above retains every geometry check and the same scene identity.
  const saved = await ctx.newPage();
  await ctx.openLab(saved, `${ctx.url}?map=river`);
  const selected = await saved.evaluate(() => ({
    id: window.__lab.route.mapId,
    size: window.__lab.route.mapSize,
  }));
  ctx.check(
    "the generic viewer loads the selected catalogue map",
    selected.id === "river" && selected.size.join() === "640,480",
    JSON.stringify(selected),
  );
  ctx.check(
    "the viewer names the catalogue map",
    (await saved.getByTestId("geometry-panel").locator("strong").textContent()) === "River lab",
  );
  await snapshot(ctx, saved, "catalogue-river-1280x800.png");
  await saved.setViewportSize({ width: 360, height: 800 });
  await snapshot(ctx, saved, "catalogue-river-360x800.png");
  const large = await ctx.newPage();
  await ctx.openLab(large, `${ctx.url}?map=market-town-test`);
  const centre = await large.evaluate(() => {
    const [w, h] = window.__lab.route.mapSize;
    return [w / 2, h / 2, window.__lab.route.heightAt(w / 2, h / 2)];
  });
  const hit = await probeAt(large, ...centre);
  ctx.check(
    "a large map can be probed from its opening overview",
    hit !== null,
    JSON.stringify(hit),
  );
  const point = await large.evaluate(([x, y, z]) => window.__lab.projectToCss(x, y, z), centre);
  await large.mouse.click(point[0], point[1]);
  await large.evaluate(() => window.__lab.frame());
  ctx.check(
    "the large-map overview click reports a hit",
    /Hit/.test(await large.getByTestId("probe").textContent()),
  );
  const overview = await snapshot(ctx, large, "catalogue-market-town-test-1280x800.png");
  await writeCrop(
    decode(overview),
    ctx.evidencePath("catalogue-market-town-test-panel-2x.png"),
    204,
    160,
    400,
    300,
    2,
  );
  await large.setViewportSize({ width: 360, height: 800 });
  await snapshot(ctx, large, "catalogue-market-town-test-360x800.png");
}
