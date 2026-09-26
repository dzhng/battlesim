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
// must match exactly; every pixel within 8-bit rounding.
import { writeFile } from "node:fs/promises";
import { decode } from "./_png.mjs";

/** The largest rounding the four 8-bit captures can add up to. */
const ROUNDING = 2;

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
  for (let y = 1; y < height - 1; y++) {
    for (let x = 1; x < width - 1; x++) {
      const i = (y * width + x) * 4;
      let err = 0;
      for (let c = 0; c < 3; c++) {
        const b = black.data[i + c];
        const w = white.data[i + c];
        const predicted = b + ((w - b) / 255) * world.data[i + c];
        err = Math.max(err, Math.abs(final.data[i + c] - predicted));
        if (w - b < 255) covered++;
      }
      worst = Math.max(worst, err);
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
  const result = { opaque, opaqueDiffering, coveredChannels: covered, worst };
  await ctx.writeEvidence(`${name}-isolation.json`, result);
  return {
    ...result,
    /** Opaque interiors exactly the overlay's colour; everything within rounding. */
    isolated: opaqueDiffering === 0 && worst <= ROUNDING,
  };
}
