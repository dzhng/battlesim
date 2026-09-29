// The panel workbench, scripted: one contact sheet per layout variant (every
// specimen, at 2× zoom so the sheet reads like the game at a close look),
// and a side-by-side sheet of a representative few in every variant. At 1×,
// 1.5× and 2× it measures every row's icon against the centre of its slot
// or ring, checks every panel is titled, that unlimited ammunition is the
// drawn ∞, and that no own panel draws a warm (amber) colour outside a
// warning.
import { copyFile, mkdir } from "node:fs/promises";

const VARIANTS = ["line", "ring", "ledger", "ledger-plain"];
const SCRATCH = process.env.PANELS_COPY_TO;

/** Every mark's icon ink centre against its slot's centre, in CSS pixels. */
const offCentre = (page) =>
  page.evaluate(() => {
    const worst = { d: 0, at: null };
    let marks = 0;
    for (const mark of document.querySelectorAll(".ro-mark")) {
      const icon = mark.querySelector(".ro-mark-icon svg");
      if (!icon) continue;
      const ink = [...icon.querySelectorAll("path, rect, circle")].map((e) =>
        e.getBoundingClientRect(),
      );
      const box = {
        x0: Math.min(...ink.map((r) => r.left)),
        x1: Math.max(...ink.map((r) => r.right)),
        y0: Math.min(...ink.map((r) => r.top)),
        y1: Math.max(...ink.map((r) => r.bottom)),
      };
      // A ringed mark's centre is its ring's; a bare one's is its slot's.
      const ring = mark.querySelector(".ro-track");
      const c = (ring ?? mark).getBoundingClientRect();
      const d = Math.hypot(
        (box.x0 + box.x1) / 2 - (c.left + c.width / 2),
        (box.y0 + box.y1) / 2 - (c.top + c.height / 2),
      );
      // In the page's own pixels, whatever its zoom.
      const zoom = Number(getComputedStyle(document.querySelector(".pw")).zoom) || 1;
      marks++;
      if (d / zoom > worst.d) worst.at = mark.closest("[data-specimen]")?.dataset.specimen ?? "?";
      worst.d = Math.max(worst.d, d / zoom);
    }
    return { marks, worst: Number(worst.d.toFixed(3)), at: worst.at };
  });

/** What the page shows, per panel. */
const panels = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("[data-specimen]")].map((card) => ({
      id: card.dataset.specimen,
      owner: card.querySelector(".ro-unit").dataset.owner,
      name: card.querySelector(".ro-name")?.textContent.trim() ?? "",
      firstIsName: card.querySelector(".ro-body")?.firstElementChild?.classList.contains("ro-name"),
      sections: [...card.querySelectorAll(".ro-section")].map((s) =>
        s.classList.contains("ro-weapons") ? "weapons" : "states",
      ),
      ammo: [...card.querySelectorAll("[data-ammo]")].map((r) => ({
        ammo: r.dataset.ammo,
        drawnInfinity: !!r.querySelector(".ro-unlimited svg"),
        textInfinity: r.textContent.includes("∞"),
      })),
    })),
  );

/** Warm colours (red over blue by more than a little, and not the enemy's
 *  or a warning's) drawn in an own panel. */
const warmInOwn = (page) =>
  page.evaluate(() => {
    const warm = [];
    const props = ["color", "stroke", "fill", "background-color", "border-top-color"];
    for (const panel of document.querySelectorAll('.ro-unit[data-owner="own"]'))
      for (const el of panel.querySelectorAll("*")) {
        if (el.closest("[data-warn], .ro-badge")) continue;
        const cs = getComputedStyle(el);
        if (cs.display === "none") continue;
        for (const p of props) {
          const m = cs
            .getPropertyValue(p)
            .match(/rgba?\((\d+),?\s*(\d+),?\s*(\d+)(?:[,/]\s*([\d.]+))?/);
          if (!m) continue;
          const [r, , b, a] = [Number(m[1]), Number(m[2]), Number(m[3]), m[4] ?? "1"];
          if (Number(a) > 0 && r - b > 24)
            warm.push(
              `${el.closest("[data-specimen]")?.dataset.specimen} ${el.className?.baseVal ?? el.className} ${p} ${cs.getPropertyValue(p)}`,
            );
        }
      }
    return warm.slice(0, 8);
  });

/** The whole page in one frame: the viewport grown to the page's size. */
async function sheet(ctx, page, file) {
  const [w, h] = await page.evaluate(() => {
    const pw = document.querySelector(".pw");
    const zoom = Number(getComputedStyle(pw).zoom) || 1;
    return [Math.ceil(pw.scrollWidth * zoom), Math.ceil(pw.scrollHeight * zoom)];
  });
  await page.setViewportSize({ width: Math.max(1500, w), height: h });
  await page.screenshot({ path: ctx.evidencePath(file) });
  await page.setViewportSize({ width: 1500, height: 900 });
}

export async function run(ctx) {
  const page = await ctx.newPage({ viewport: { width: 1500, height: 900 } });
  const sheets = [];
  for (const variant of VARIANTS) {
    for (const scale of [1, 1.5, 2]) {
      await ctx.openLab(page, `${ctx.url}?variant=${variant}&scale=${scale}`);
      await page.evaluate(() => document.fonts.ready);
      const c = await offCentre(page);
      ctx.check(
        `${variant} at ${scale}×: every row's icon sits at its slot's or ring's centre (within 0.5 px)`,
        c.marks > 50 && c.worst <= 0.5,
        JSON.stringify(c),
      );
    }
    // The sheet is shot at 2×, the page's last zoom.
    const shown = await panels(page);
    const untitled = shown.filter((p) => !p.name || !p.firstIsName);
    ctx.check(
      `${variant}: every panel is titled with its unit's name, first`,
      shown.length > 60 && untitled.length === 0,
      JSON.stringify({ panels: shown.length, untitled: untitled.map((p) => p.id) }),
    );
    const order = shown.filter(
      (p) => p.sections.join() !== [...p.sections].sort().reverse().join(),
    );
    ctx.check(
      `${variant}: weapons come before states on every panel`,
      order.length === 0,
      JSON.stringify(order.map((p) => p.id)),
    );
    const ammo = shown.flatMap((p) => p.ammo);
    ctx.check(
      `${variant}: unlimited ammunition is the drawn ∞, never a font glyph`,
      ammo.some((a) => a.ammo.includes("∞")) &&
        ammo.every((a) => a.drawnInfinity === a.ammo.includes("∞") && !a.textInfinity),
      JSON.stringify(
        ammo.filter((a) => a.textInfinity || a.drawnInfinity !== a.ammo.includes("∞")),
      ),
    );
    const warm = await warmInOwn(page);
    ctx.check(
      `${variant}: no own panel draws amber outside a warning`,
      warm.length === 0,
      warm.join(" | "),
    );
    const file = `sheet-${variant}.png`;
    await sheet(ctx, page, file);
    sheets.push(file);
  }
  await ctx.openLab(page, `${ctx.url}?compare&scale=2`);
  await page.evaluate(() => document.fonts.ready);
  await sheet(ctx, page, "compare.png");
  sheets.push("compare.png");
  if (SCRATCH) {
    await mkdir(SCRATCH, { recursive: true });
    for (const f of sheets) await copyFile(ctx.evidencePath(f), `${SCRATCH}/${f}`);
  }
  await page.close();
}
