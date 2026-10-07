// X-ray readability in the production wood-edge firefight. The same posed
// models and camera are drawn with and without the coverage gate; grass is
// independently removed to prove it cannot trigger this unit cue.
import { openBattle, aim, presented, snapshot } from "./_lab.mjs";
import { decode } from "./_png.mjs";

const blue = (data, i) =>
  data[i + 2] > 60 && data[i + 2] > data[i] + 25 && data[i + 1] > data[i] + 10;
const bluePixels = (png) => {
  let count = 0;
  for (let i = 0; i < png.data.length; i += 4) if (blue(png.data, i)) count++;
  return count;
};

export async function xrayTour(ctx) {
  const page = await openBattle(ctx, {
    url: ctx.url.replace(/\/lab\/street.*$/, "/lab/lean"),
    viewport: { width: 1920, height: 1080 },
    tick: 331,
    grass: true,
  });
  try {
    await presented(page);
    // DOM leader lines share the cue's cyan and survive the frame inspector.
    await page.addStyleTag({
      content: ".ro-layer,[data-testid=battle-panel]{display:none!important}",
    });
    await aim(page, [874, 962, 0], { distance: 28, pitch: 0.25, yaw: 0 });
    await page.evaluate(() => window.__lab.setFrameView("final"));
    await snapshot(ctx, page, "xray-cover-final.png");
    await page.evaluate(() => window.__lab.setFrameView("overlays-on-black"));
    const enabled = decode(await snapshot(ctx, page, "xray-cover-overlay.png"));
    await page.evaluate(() => window.__lab.suppressGrass(true));
    const bare = decode(await snapshot(ctx, page, "xray-cover-no-grass.png"));
    ctx.check("grass cannot trigger own-unit x-ray pixels", enabled.data.equals(bare.data));
    await page.evaluate(() => window.__lab.suppressGrass(false));
    await page.evaluate(() => window.__lab.suppressXrayCoverage(true));
    const legacy = decode(await snapshot(ctx, page, "xray-cover-ungated.png"));
    const cue = bluePixels(enabled),
      ungated = bluePixels(legacy);
    ctx.check(
      "incidental hidden fragments are suppressed while substantial cover remains readable",
      cue > 0 && cue < ungated,
      JSON.stringify({ cue, ungated }),
    );
  } finally {
    await page.close();
  }
}
