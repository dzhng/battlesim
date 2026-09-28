// Overlay colour isolation (battle-look slice 12): overlays composite after
// post in display space, so the finished frame must be exactly the overlay
// laid over the finished world, with no fog, grade or tone map on the
// overlay's own colours.
//
// Four frames of one paused state, through the pass inspector's views: the
// final frame, the world alone, and the overlays alone over black and over
// white. Per channel, black gives the premultiplied overlay B = a·c and white
// gives W = B + (1 − a)·255, so the final pixel must be B + (W − B)/255 · world.
// Opaque overlay pixels (W = B, and so their neighbours, to skip MSAA edges)
// must match exactly; every pixel the overlay covers within 8-bit rounding.
// Where the overlay covers nothing the final frame is the world capture, so a
// difference there is the world's own, not the overlay's: dense grass lets
// a blade tie flip a pixel between captures now and then (battle-look slice
// 18). A few such stray pixels are allowed; a grade or fog on the overlay
// path would move thousands.
import { writeFile } from "node:fs/promises";
import { decode } from "./_png.mjs";

/** The largest rounding the four 8-bit captures can add up to. */
const ROUNDING = 2;
/** Uncovered pixels that may differ from the world capture past rounding. */
const STRAY_MAX = 16;

async function capture(page, view) {
  await page.evaluate((v) => window.__lab.setFrameView(v), view);
  return page.screenshot();
}

/** Check one frame's overlay isolation; `name` prefixes the evidence files. */
export async function checkOverlayIsolation(ctx, page, name) {
  // The DOM (panels, readouts) is not the frame; hide all but the canvas.
  const hide = await page.addStyleTag({
    content: "* { visibility: hidden !important } canvas { visibility: visible !important }",
  });
  const shots = {};
  for (const view of ["final", "world", "overlays-on-black", "overlays-on-white"]) {
    shots[view] = await capture(page, view);
  }
  await page.evaluate(() => window.__lab.setFrameView("final"));
  await hide.evaluate((el) => el.remove());
  for (const [view, shot] of Object.entries(shots)) {
    await writeFile(ctx.evidencePath(`${name}-${view}.png`), shot);
  }
  const [final, world, black, white] = Object.values(shots).map(decode);
  const { width, height } = final;
  const opaqueAt = (x, y) => {
    const i = (y * width + x) * 4;
    return (
      black.data[i] === white.data[i] &&
      black.data[i + 1] === white.data[i + 1] &&
      black.data[i + 2] === white.data[i + 2]
    );
  };
  let opaque = 0;
  let opaqueDiffering = 0;
  let covered = 0;
  let worst = 0;
  let stray = 0;
  for (let y = 1; y < height - 1; y++) {
    for (let x = 1; x < width - 1; x++) {
      const i = (y * width + x) * 4;
      let err = 0;
      let touched = false;
      for (let c = 0; c < 3; c++) {
        const b = black.data[i + c];
        const w = white.data[i + c];
        const predicted = b + ((w - b) / 255) * world.data[i + c];
        err = Math.max(err, Math.abs(final.data[i + c] - predicted));
        if (w - b < 255) {
          covered++;
          touched = true;
        }
      }
      if (touched) worst = Math.max(worst, err);
      else if (err > ROUNDING) stray++;
      let interior = true;
      for (let dy = -1; dy <= 1 && interior; dy++) {
        for (let dx = -1; dx <= 1 && interior; dx++) interior = opaqueAt(x + dx, y + dy);
      }
      if (!interior) continue;
      opaque++;
      if (
        final.data[i] !== black.data[i] ||
        final.data[i + 1] !== black.data[i + 1] ||
        final.data[i + 2] !== black.data[i + 2]
      ) {
        opaqueDiffering++;
      }
    }
  }
  const result = { opaque, opaqueDiffering, coveredChannels: covered, worst, stray };
  await ctx.writeEvidence(`${name}-isolation.json`, result);
  return {
    ...result,
    /** Opaque interiors exactly the overlay's colour; every covered pixel
     *  within rounding; at most a few stray world pixels. */
    isolated: opaqueDiffering === 0 && worst <= ROUNDING && stray <= STRAY_MAX,
  };
}

/** The painted ground marks alone (27e follow-ups: they are drawn in the lit
 *  world, not the overlay): the frame with them less the frame without,
 *  each channel's rise kept (a mark lightens what it is painted on), so a
 *  pixel reads the mark's hue over black; `under` is the frame without
 *  them. Saves both frames as evidence. */
export async function paintOnly(ctx, page, name) {
  const shot = async (suffix) => {
    await page.evaluate(() => window.__lab.frame());
    const png = await page.screenshot();
    await writeFile(ctx.evidencePath(`${name}-${suffix}.png`), png);
    return decode(png);
  };
  const on = await shot("paint-on");
  await page.evaluate(() => window.__lab.suppressPaint(true));
  const off = await shot("paint-off");
  await page.evaluate(() => window.__lab.suppressPaint(false));
  const data = Buffer.alloc(on.data.length);
  for (let i = 0; i < data.length; i += 4) {
    for (let k = 0; k < 3; k++) data[i + k] = Math.max(0, on.data[i + k] - off.data[i + k]);
    data[i + 3] = 255;
  }
  // The frame without them rides along: what the marks are painted on.
  return { width: on.width, height: on.height, data, under: off };
}
