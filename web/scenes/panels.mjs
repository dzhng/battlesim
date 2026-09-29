// The panel workbench, scripted: a contact sheet of every specimen at 2×
// zoom (so it reads like the game at a close look). At 1×, 1.5× and 2× it
// measures every row's icon against the centre of its slot, and checks every
// panel is titled, that unlimited ammunition is the drawn ∞, and that no own
// panel draws a warm (amber) colour outside a warning.

/** Every mark's icon ink centre against its slot's centre, in CSS pixels. */
const offCentre = (page) =>
  page.evaluate(() => {
    const worst = { d: 0, at: null };
    let marks = 0;
    let rings = 0;
    let worstRing = 0;
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
      // A timer's ring round the icon shares its centre, and the slot's.
      const ring = mark.querySelector(".ro-ring");
      for (const r of ring ? [ring] : []) {
        const b = r.getBoundingClientRect();
        const m = mark.getBoundingClientRect();
        const off = Math.hypot(
          b.left + b.width / 2 - (m.left + m.width / 2),
          b.top + b.height / 2 - (m.top + m.height / 2),
        );
        rings++;
        worstRing = Math.max(
          worstRing,
          off / (Number(getComputedStyle(document.querySelector(".pw")).zoom) || 1),
        );
      }
      const c = mark.getBoundingClientRect();
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
    return {
      marks,
      worst: Number(worst.d.toFixed(3)),
      at: worst.at,
      rings,
      worstRing: Number(worstRing.toFixed(3)),
    };
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
        if (el.closest("[data-tone], .ro-badge")) continue;
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
  for (const scale of [1, 1.5, 2]) {
    await ctx.openLab(page, `${ctx.url}?scale=${scale}`);
    await page.evaluate(() => document.fonts.ready);
    const c = await offCentre(page);
    ctx.check(
      `at ${scale}×: every row's icon, and every timer's ring, sits at its slot's centre (within 0.5 px)`,
      c.marks > 50 && c.worst <= 0.5 && c.rings > 5 && c.worstRing <= 0.5,
      JSON.stringify(c),
    );
  }
  // The rest reads the page at 2×, its last zoom, which the sheet shoots.
  const shown = await panels(page);
  const untitled = shown.filter((p) => !p.name || !p.firstIsName);
  ctx.check(
    "every panel is titled with its unit's name, first",
    shown.length > 60 && untitled.length === 0,
    JSON.stringify({ panels: shown.length, untitled: untitled.map((p) => p.id) }),
  );
  const order = shown.filter((p) => p.sections.join() !== [...p.sections].sort().reverse().join());
  ctx.check(
    "weapons come before states on every panel",
    order.length === 0,
    JSON.stringify(order.map((p) => p.id)),
  );
  const ammo = shown.flatMap((p) => p.ammo);
  ctx.check(
    "unlimited ammunition is the drawn ∞, never a font glyph",
    ammo.some((a) => a.ammo.includes("∞")) &&
      ammo.every((a) => a.drawnInfinity === a.ammo.includes("∞") && !a.textInfinity),
    JSON.stringify(ammo.filter((a) => a.textInfinity || a.drawnInfinity !== a.ammo.includes("∞"))),
  );
  const ringless = await page.evaluate(
    () =>
      [...document.querySelectorAll(".ro-row")].filter((r) => {
        const timing =
          (r.dataset.aim ?? "") !== "" ||
          (r.dataset.reload ?? "") !== "" ||
          (r.dataset.progress ?? "") !== "";
        return timing !== !!r.querySelector(".ro-ring");
      }).length,
  );
  ctx.check(
    "a ring shows round a row's icon exactly while its timer runs",
    ringless === 0,
    String(ringless),
  );
  const warm = await warmInOwn(page);
  ctx.check("no own panel draws amber outside a warning", warm.length === 0, warm.join(" | "));
  await sheet(ctx, page, "sheet.png");
  // The key cases alone, for a close look.
  await page.evaluate(() =>
    document.querySelectorAll(".pw > section:not(:first-of-type)").forEach((e) => e.remove()),
  );
  await sheet(ctx, page, "key-cases.png");
  await page.close();
}
