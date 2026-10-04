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
      if (row.getClientRects().length === 0) continue;
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
  const titledIcon = page.locator(".ro-icon[title]").first();
  ctx.check(
    "informative panel icons retain hover hit testing",
    (await titledIcon.evaluate((e) => getComputedStyle(e).pointerEvents)) !== "none",
  );
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
    ["own states/hidden in foliage", "hidden-expanded.png"],
    ["own states/hidden, compressed", "hidden-compact.png"],
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
  await ctx.openLab(page, ctx.url);
  await page.getByRole("button", { name: "Review battle deck" }).click();
  for (const width of [1600, 1024]) {
    await page.setViewportSize({ width, height: 900 });
    for (const name of [
      "Rifle squad",
      "Tank ammunition",
      "Suppressed and resupplying",
      "Deploying supply truck",
      "Mixed capabilities",
      "Garrison exit",
      "Large selection",
      "Army roster",
      "Entire force",
      "No selection",
      "Replay",
    ]) {
      await page.getByRole("button", { name, exact: true }).click();
      await page.mouse.move(0, 500);
      const layout = await page.locator(".hud-army-deck").evaluate((deck) => {
        const rect = deck.getBoundingClientRect();
        const cards = [...deck.querySelectorAll(".hud-army-card")].map((e) =>
          e.getBoundingClientRect(),
        );
        const commands = deck.querySelector('[role="toolbar"]')?.getBoundingClientRect();
        return {
          left: rect.left,
          right: rect.right,
          top: rect.top,
          bottom: rect.bottom,
          height: rect.height,
          cardBottom: Math.max(...cards.map((r) => r.bottom)),
          cardCenter: (cards[0].left + cards.at(-1).right) / 2,
          singleRow: cards.every((r) => Math.abs(r.top - cards[0].top) < 1),
          commandTop: commands?.top ?? null,
          commandHeight: commands?.height ?? null,
          captionBottom: document.querySelector('[data-testid="captions"]').getBoundingClientRect()
            .bottom,
          text: deck.textContent,
        };
      });
      ctx.check(
        `${width}px ${name}: cards stay in one compact row above commands and clear captions`,
        layout.singleRow &&
          layout.height <= 155 &&
          layout.left >= 0 &&
          layout.right <= width &&
          layout.bottom <= 900 &&
          layout.captionBottom < layout.top &&
          (layout.commandTop === null ||
            (layout.commandTop >= layout.cardBottom && layout.commandHeight <= 48)),
        JSON.stringify(layout),
      );
      if (name !== "Entire force")
        ctx.check(
          `${width}px ${name}: unit cards are centered above the command row`,
          Math.abs(layout.cardCenter - (layout.left + layout.right) / 2) <= 1,
          JSON.stringify(layout),
        );
      ctx.check(
        `${width}px ${name}: no-selection and replay keep the roster without commands or a selection label`,
        (name === "No selection" || name === "Replay"
          ? layout.commandTop === null
          : layout.commandTop !== null) && !/\d+ selected/i.test(layout.text),
        JSON.stringify(layout),
      );
      const slug = name.toLowerCase().replaceAll(" ", "-");
      await page.screenshot({ path: ctx.evidencePath(`army-${width}-${slug}.png`) });
      const cards = page.locator(".hud-army-card");
      await cards.first().hover();
      const detail = await page.getByRole("tooltip").evaluate((e) => {
        const r = e.getBoundingClientRect();
        const words = [...e.querySelectorAll(".ro-name-word, .ro-row")];
        return {
          left: r.left,
          right: r.right,
          top: r.top,
          bottom: r.bottom,
          unit: e.dataset.unit,
          text: e.textContent,
          complete: words.every(
            (w) => getComputedStyle(w).display !== "none" && w.getBoundingClientRect().width > 0,
          ),
        };
      });
      const first = await cards.first().getAttribute("data-unit");
      ctx.check(
        `${width}px ${name}: hover shows complete facts owned by the card above the roster`,
        detail.unit === first &&
          detail.complete &&
          detail.top >= 0 &&
          detail.bottom < layout.top &&
          detail.left >= 0 &&
          detail.right <= width,
        JSON.stringify(detail),
      );
      if (name === "Tank ammunition" || name === "Replay")
        ctx.check(
          `${width}px ${name}: tank hover retains both weapons`,
          /CANNON/.test(detail.text) && /HMG/.test(detail.text),
          detail.text,
        );
      if (name === "Suppressed and resupplying")
        ctx.check(
          `${width}px: busy hover retains suppression and resupply`,
          /SUPPRESSED/.test(detail.text) && /SUPPLY/.test(detail.text),
          detail.text,
        );
      await page.screenshot({ path: ctx.evidencePath(`army-${width}-${slug}-hover.png`) });
      await page.mouse.move(0, 500);
      if (name === "Entire force") {
        const overflow = await page
          .locator(".hud-army")
          .evaluate((e) => e.scrollWidth > e.clientWidth);
        ctx.check(`${width}px: whole army overflows horizontally`, overflow);
        await cards.first().focus();
        for (let i = 1; i < (await cards.count()); i++) await page.keyboard.press("Tab");
        const reach = await cards.last().evaluate((e) => {
          const r = e.getBoundingClientRect(),
            parent = e.parentElement.getBoundingClientRect();
          return {
            focused: document.activeElement === e,
            left: r.left,
            right: r.right,
            parentLeft: parent.left,
            parentRight: parent.right,
            scroll: e.parentElement.scrollLeft,
          };
        });
        ctx.check(
          `${width}px: Tab reaches and reveals the final unit in the horizontal row`,
          reach.focused &&
            reach.scroll > 0 &&
            reach.left >= reach.parentLeft &&
            reach.right <= reach.parentRight,
          JSON.stringify(reach),
        );
        const lastId = await cards.last().getAttribute("data-unit");
        ctx.check(
          `${width}px: final focused card owns its facts`,
          (await page.getByRole("tooltip").getAttribute("data-unit")) === lastId,
        );
        await page.screenshot({
          path: ctx.evidencePath(`army-${width}-entire-force-focus-end.png`),
        });
        await page.keyboard.press("Escape");
        ctx.check(
          `${width}px: Escape dismisses card details`,
          (await page.getByRole("tooltip").count()) === 0,
        );
        await page.getByRole("button", { name: "Info panels", exact: true }).focus();
      }
      if (name === "Mixed capabilities") {
        const attack = page.getByRole("button", { name: /^Attack-move / });
        await attack.click();
        ctx.check(
          `${width}px: real pointer click arms the command`,
          (await attack.getAttribute("aria-pressed")) === "true",
        );
        await page.mouse.move(0, 500);
        const deploy = page.getByRole("button", { name: /^Deploy / });
        await cards.last().focus();
        for (let i = 0; i < 7; i++) await page.keyboard.press("Tab");
        ctx.check(
          `${width}px: keyboard deployment focus is distinct from the armed attack-move`,
          await deploy.evaluate(
            (e) => document.activeElement === e && parseFloat(getComputedStyle(e).outlineWidth) > 0,
          ),
        );
        ctx.check(
          `${width}px: deployment focus retains its binding without counts`,
          (await page.getByRole("tooltip").textContent()) === "Deploy (T)",
        );
        await page.screenshot({ path: ctx.evidencePath(`army-${width}-mixed-command-focus.png`) });
        await page.getByRole("button", { name: "Info panels", exact: true }).focus();
      }
    }
    await page.getByRole("button", { name: "Rifle squad", exact: true }).click();
    await page.getByRole("button", { name: /^Attack ground / }).focus();
    const focus = await page.getByRole("button", { name: /^Attack ground / }).evaluate((e) => ({
      active: document.activeElement === e,
      outline: parseFloat(getComputedStyle(e).outlineWidth),
      icon: e.querySelector("svg").getBoundingClientRect().height,
    }));
    ctx.check(
      `${width}px: compact commands retain legible icons and strong keyboard focus`,
      focus.active && focus.outline >= 2 && focus.icon >= 24,
      JSON.stringify(focus),
    );
    const tip = await page.getByRole("tooltip").boundingBox();
    const commands = await page.getByRole("toolbar", { name: "Commands" }).boundingBox();
    const deck = await page.locator(".hud-army-deck").boundingBox();
    ctx.check(
      `${width}px: command row is centered below the cards`,
      Math.abs(commands.x + commands.width / 2 - deck.x - deck.width / 2) <= 1,
    );
    const heard = await page.getByTestId("captions").boundingBox();
    ctx.check(
      `${width}px: command hint stays above its owner and below captions`,

      heard.y + heard.height < tip.y && tip.x >= 0 && tip.x + tip.width <= width,
    );
    await page.screenshot({ path: ctx.evidencePath(`army-${width}-focus-tooltip.png`) });
  }
  await page.close();
}
