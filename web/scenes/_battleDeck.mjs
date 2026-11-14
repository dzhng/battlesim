// The battle deck's workbench, scripted (part of the panels scene): every
// case of the army deck and its command bar at two widths, plain and hovered,
// with the deck's layout contract and its approved pictures.
import { pointerAway } from "./_baseline.mjs";
import { spills } from "./_spills.mjs";

/** The battle deck's every case at two widths, plain and hovered: its layout
 *  contract and its approved pictures. */
export async function checkBattleDeck(ctx, page) {
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
      "Smoke ready",
      "Smoke spent",
      "Garrison exit",
      "Large selection",
      "Army roster",
      "Entire force",
      "No selection",
      "Replay",
    ]) {
      await page.getByRole("button", { name, exact: true }).click();
      await pointerAway(page);
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
          // Rows are wrapped, never scrolled: every card lies in the row's box.
          scrolls:
            deck.querySelector(".hud-army").scrollWidth >
            deck.querySelector(".hud-army").clientWidth + 1,
          cardsOnScreen: cards.every(
            (r) => r.left >= 0 && r.right <= innerWidth && r.bottom <= innerHeight,
          ),
          cardHeights: new Set(cards.map((r) => Math.round(r.height))).size,
          commandTop: commands?.top ?? null,
          commandHeight: commands?.height ?? null,
          commandsDisabled: [
            ...(deck.querySelector('[role="toolbar"]')?.querySelectorAll("button") ?? []),
          ].every((button) => button.disabled),
          captionBottom: document.querySelector('[data-testid="captions"]').getBoundingClientRect()
            .bottom,
          text: deck.textContent,
        };
      });
      ctx.check(
        `${width}px ${name}: cards wrap on screen at one size above commands and clear captions`,
        !layout.scrolls &&
          layout.cardsOnScreen &&
          layout.cardHeights === 1 &&
          layout.left >= 0 &&
          layout.right <= width &&
          layout.bottom <= 900 &&
          layout.captionBottom < layout.top &&
          (layout.commandTop === null ||
            (layout.commandTop >= layout.cardBottom + 4 && layout.commandHeight <= 48)),
        JSON.stringify(layout),
      );
      if (name !== "Entire force")
        ctx.check(
          `${width}px ${name}: unit cards are centered above the command row`,
          Math.abs(layout.cardCenter - (layout.left + layout.right) / 2) <= 1,
          JSON.stringify(layout),
        );
      // No selection keeps the command row, every action disabled, so the
      // deck does not move when units are selected; a replay has none.
      ctx.check(
        `${width}px ${name}: no-selection keeps disabled commands, replay none, and neither a selection label`,
        (name === "Replay"
          ? layout.commandTop === null
          : layout.commandTop !== null && (name !== "No selection" || layout.commandsDisabled)) &&
          !/\d+ selected/i.test(layout.text),
        JSON.stringify(layout),
      );
      const slug = name.toLowerCase().replaceAll(" ", "-");
      // A game's interface is never selected like a page's text: no word of
      // the HUD (a caption, a card, a command) takes a selection highlight.
      const selectable = await page.evaluate(() =>
        [...document.querySelectorAll(".hud *")]
          .filter((e) => [...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()))
          .filter((e) => getComputedStyle(e).userSelect !== "none")
          .map((e) => e.className || e.tagName)
          .slice(0, 6),
      );
      ctx.check(
        `${width}px ${name}: no HUD text can be selected`,
        selectable.length === 0,
        selectable.join(" | "),
      );
      const deckSpills = await spills(page, ".hud-panel");
      ctx.check(
        `${width}px ${name}: every deck panel holds its content`,
        deckSpills.length === 0,
        deckSpills.slice(0, 6).join(" | "),
      );
      await ctx.matchBaseline(page, `army-${width}-${slug}`);
      const cards = page.locator(".hud-army-card");
      await cards.first().hover();
      const detail = await page.getByRole("tooltip").evaluate((e) => {
        const r = e.getBoundingClientRect();
        const words = [...e.querySelectorAll(".ro-name-word, .ro-row")];
        const name = e.querySelector(".ro-name-word");
        return {
          nameHeight: name.getBoundingClientRect().height,
          nameFont: parseFloat(getComputedStyle(name).fontSize),
          weaponRows: e.querySelectorAll(".ro-weapon").length,
          captionBottom: document.querySelector('[data-testid="captions"]').getBoundingClientRect()
            .bottom,
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
          Math.abs(layout.top - detail.bottom - 4) < 0.5 &&
          detail.left >= 0 &&
          detail.right <= width,
        JSON.stringify(detail),
      );
      // A card collapsed to its padding still has a box on screen and every
      // word drawn: only its content against its frame shows it.
      const hoverSpills = await spills(page, ".hud-army-detail");
      ctx.check(
        `${width}px ${name}: hover card holds its contents, its name on one line, clear of captions`,
        hoverSpills.length === 0 &&
          detail.captionBottom <= detail.top &&
          detail.nameHeight < detail.nameFont * 2,
        JSON.stringify({ ...detail, spills: hoverSpills.slice(0, 6) }),
      );
      if (name === "Tank ammunition" || name === "Replay")
        ctx.check(
          `${width}px ${name}: tank hover retains both weapons`,
          detail.weaponRows === 2,
          detail.text,
        );
      if (name === "Suppressed and resupplying")
        ctx.check(
          `${width}px: busy hover retains suppression and resupply`,
          /SUPPRESSED/.test(detail.text) && /SUPPLY/.test(detail.text),
          detail.text,
        );
      await ctx.matchBaseline(page, `army-${width}-${slug}-hover`);
      await pointerAway(page);
      if (name === "Entire force") {
        ctx.check(
          `${width}px: the whole army wraps into several rows`,
          (await page.locator(".hud-army").getAttribute("data-army-rows")) > 1,
        );
      }
      if (name === "Mixed capabilities") {
        const attack = page.getByRole("button", { name: /^Attack-move / });
        await attack.click();
        ctx.check(
          `${width}px: real pointer click arms the command`,
          (await attack.getAttribute("aria-pressed")) === "true",
        );
        await pointerAway(page);
      }
    }
    await page.getByRole("button", { name: "Rifle squad", exact: true }).click();
    const attackGround = page.getByRole("button", { name: /^Attack ground / });
    await attackGround.hover();
    const icon = await attackGround.evaluate(
      (e) => e.querySelector("svg").getBoundingClientRect().height,
    );
    ctx.check(`${width}px: compact commands retain legible icons`, icon >= 24, String(icon));
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
    await ctx.matchBaseline(page, `army-${width}-command-tooltip`);
  }
}
