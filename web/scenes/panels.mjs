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

/** Each row's items (its icon, words, counts, ∞, marks and pips) by the
 *  vertical centre of their ink, in the page's own pixels: the worst spread
 *  across one row. Text ink is measured from the glyphs themselves (the
 *  font's ink above and below the baseline, found with a zero-height probe),
 *  not from the text's box, so no CSS box trick can pass it. */
const rowCentres = (page) =>
  page.evaluate(() => {
    const zoom = Number(getComputedStyle(document.querySelector(".pw")).zoom) || 1;
    const ctx = document.createElement("canvas").getContext("2d");
    const inkBox = (els) => {
      const rs = els
        .map((e) => e.getBoundingClientRect())
        .filter((r) => r.width > 0 || r.height > 0);
      return rs.length
        ? (Math.min(...rs.map((r) => r.top)) + Math.max(...rs.map((r) => r.bottom))) / 2
        : null;
    };
    const textCentre = (el) => {
      const text = el.textContent.trim();
      if (!text) return null;
      const cs = getComputedStyle(el);
      if (cs.display === "none") return null;
      const probe = document.createElement("span");
      probe.style.cssText = "display:inline-block;width:0;height:0;padding:0;margin:0;border:0";
      el.append(probe);
      const baseline = probe.getBoundingClientRect().bottom;
      probe.remove();
      ctx.font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
      const m = ctx.measureText(cs.textTransform === "uppercase" ? text.toUpperCase() : text);
      // measureText is in CSS px of the font size as declared; the page zoom
      // scales the rendered glyphs by `zoom`.
      return baseline - ((m.actualBoundingBoxAscent - m.actualBoundingBoxDescent) / 2) * zoom;
    };
    let worst = { spread: 0, at: null, items: null };
    let rows = 0;
    for (const row of document.querySelectorAll(".ro-row")) {
      if (getComputedStyle(row).display === "none") continue;
      const items = [];
      const mark = row.querySelector(".ro-mark-icon svg");
      if (mark) items.push(["icon", inkBox([...mark.querySelectorAll("path, rect, circle")])]);
      for (const t of row.querySelectorAll(
        ".ro-word, .ro-kind-label, .ro-ammo:not(.ro-unlimited), .ro-badge, .ro-guide",
      ))
        items.push([t.className, textCentre(t)]);
      for (const u of row.querySelectorAll(".ro-unlimited svg"))
        items.push(["∞", inkBox([...u.querySelectorAll("path, rect, circle")])]);
      for (const p of row.querySelectorAll(".ro-pips:not(.ro-pips-none)"))
        items.push(["pips", inkBox([p])]);
      const ys = items.filter(([, y]) => y !== null).map(([, y]) => y);
      if (ys.length < 2) continue;
      rows++;
      const spread = (Math.max(...ys) - Math.min(...ys)) / zoom;
      if (spread > worst.spread)
        worst = {
          spread,
          at: row.closest("[data-specimen]")?.dataset.specimen ?? "?",
          items: items.map(([k, y]) => [k, y === null ? null : Number((y / zoom).toFixed(2))]),
        };
    }
    return { rows, ...worst, spread: Number(worst.spread.toFixed(3)) };
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
    const line = await rowCentres(page);
    ctx.check(
      `at ${scale}×: every row's icon, words, counts and pips share one centre line (within 0.5 px)`,
      line.rows > 50 && line.spread <= 0.5,
      JSON.stringify(line),
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
  const progressRings = await page.evaluate(() =>
    [...document.querySelectorAll(".ro-ring")].map((ring) => ({
      circles: ring.querySelectorAll(".ro-track").length,
      arcs: ring.querySelectorAll(".ro-arc").length,
      reloading: ring.classList.contains("ro-reload"),
      radius: ring.querySelector(".ro-track")?.getAttribute("r"),
      dash: getComputedStyle(ring.querySelector(".ro-arc")).strokeDasharray,
    })),
  );
  ctx.check(
    "every progress display uses one radius; aiming is solid and reloading dashed",
    progressRings.length > 0 &&
      progressRings.every((r) => r.circles === 1 && r.arcs === 1) &&
      new Set(progressRings.map((r) => r.radius)).size === 1 &&
      progressRings.every((r) => (r.reloading ? r.dash !== "none" : r.dash === "none")),
    JSON.stringify(progressRings),
  );
  const warm = await warmInOwn(page);
  ctx.check("no own panel draws amber outside a warning", warm.length === 0, warm.join(" | "));
  const farWeapons = await page
    .locator('[data-specimen="far out/two launchers selected"]')
    .evaluate((card) => {
      const rows = [...card.querySelectorAll(".ro-weapon")];
      return rows.map((r) => ({
        name: r.querySelector(".ro-word").textContent,
        visible: getComputedStyle(r.querySelector(".ro-word")).display !== "none",
        y: r.getBoundingClientRect().top,
      }));
    });
  ctx.check(
    "far zoom retains separate named rows for identical weapon icons",
    farWeapons.map((w) => w.name).join("|") === "RIFLE|ATGM 1|ATGM 2" &&
      farWeapons.every((w) => w.visible) &&
      farWeapons[1].y < farWeapons[2].y,
    JSON.stringify(farWeapons),
  );
  await sheet(ctx, page, "sheet.png");
  for (const [id, file] of [
    ["key cases/two launchers, separate reloads", "twin-launchers.png"],
    ["key cases/two launchers, enemy equipment", "twin-enemy.png"],
    ["key cases/turret and hull HMG", "twin-hmg.png"],
    ["far out/two launchers selected", "twin-far.png"],
  ]) {
    await page
      .locator(`[data-specimen="${id}"]`)
      .first()
      .screenshot({ path: ctx.evidencePath(file) });
  }
  await page.locator(".pw-grass").screenshot({ path: ctx.evidencePath("over-grass.png") });
  await page
    .locator('.pw-grass [data-specimen="own, busiest/squad in a fight"]')
    .screenshot({ path: ctx.evidencePath("rifle-and-grenade-progress.png") });
  await page
    .locator('[data-specimen="own weapons/aiming and reloading at once"]')
    .screenshot({ path: ctx.evidencePath("aim-priority.png") });
  await page
    .locator('[data-specimen="own weapons/aim complete, reload continues"]')
    .screenshot({ path: ctx.evidencePath("reload-after-aim.png") });
  // The key cases alone, for a close look.
  await page.evaluate(() =>
    document.querySelectorAll(".pw > section:not(:first-of-type)").forEach((e) => e.remove()),
  );
  await sheet(ctx, page, "key-cases.png");
  await page.close();
}
