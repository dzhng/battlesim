// A captioned grid of pictures, saved as one PNG: the review sheet a scene
// hands to whoever judges its captures. The browser lays it out and draws the
// captions, so a sheet needs no font or layout code here.

const pages = new WeakMap();

const escape = (text) =>
  String(text).replace(
    /[&<>"]/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c],
  );

/**
 * Save `cells` (each a PNG buffer under a caption) as evidence `file`, in
 * rows of `columns`, each picture `cellWidth` pixels wide. `pixelated` scales
 * a small crop up without smoothing, so a pixel stays a pixel.
 */
export async function writeSheet(
  ctx,
  file,
  { title, columns, cellWidth, cells, pixelated = false },
) {
  let page = pages.get(ctx);
  if (!page) pages.set(ctx, (page = await ctx.newPage()));
  await page.setViewportSize({ width: columns * (cellWidth + 12) + 12, height: 200 });
  await page.setContent(
    `<style>
      body { margin: 0; padding: 12px 0 0 12px; background: #14171c; color: #e6ecf5;
        font: 13px/1.4 ui-monospace, Menlo, monospace; }
      h1 { font-size: 15px; margin: 0 0 10px; }
      main { display: grid; grid-template-columns: repeat(${columns}, ${cellWidth}px); gap: 12px; }
      figure { margin: 0 0 12px; }
      img { width: ${cellWidth}px; display: block; ${pixelated ? "image-rendering: pixelated;" : ""} }
      figcaption { padding: 3px 0 0; color: #aeb7c4; }
    </style>
    <h1>${escape(title)}</h1>
    <main>${cells
      .map(
        (cell) =>
          `<figure><img src="data:image/png;base64,${cell.png.toString("base64")}"><figcaption>${escape(cell.caption)}</figcaption></figure>`,
      )
      .join("")}</main>`,
    { waitUntil: "load" },
  );
  await page.screenshot({ path: ctx.evidencePath(file), fullPage: true });
}
