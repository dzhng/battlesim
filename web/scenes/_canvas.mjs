import { writeFile } from "node:fs/promises";
import { snapshot } from "./_lab.mjs";
import { decode } from "./_png.mjs";

/** Read render pixels before CSS scaling mixes neighbouring surfaces. */
export async function captureCanvas(page) {
  const { url, width, height, rect } = await page.evaluate(async () => {
    await window.__lab.frame();
    const canvas = document.querySelector("canvas");
    if (!canvas) throw new Error("native capture: no battle canvas");
    const r = canvas.getBoundingClientRect();
    return {
      url: canvas.toDataURL("image/png"),
      width: canvas.width,
      height: canvas.height,
      rect: { x: r.x, y: r.y, width: r.width, height: r.height },
    };
  });
  const bytes = Buffer.from(url.split(",")[1], "base64");
  const png = decode(bytes);
  if (png.width !== width || png.height !== height)
    throw new Error(`native capture: expected ${width}×${height} render pixels`);
  return { bytes, png, rect };
}

/** Preserve the CSS picture while judging a renderer's pixel-domain contract. */
export async function snapshotCanvas(ctx, page, file) {
  await snapshot(ctx, page, file);
  const { bytes, png } = await captureCanvas(page);
  await writeFile(ctx.evidencePath(file.replace(/\.png$/, "-native.png")), bytes);
  return png;
}
